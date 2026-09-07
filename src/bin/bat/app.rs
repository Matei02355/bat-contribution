use std::collections::HashSet;
use std::env;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::thread::available_parallelism;

use crate::{
    clap_app,
    config::{
        get_args_from_config_file, get_args_from_env_opts_var, get_args_from_env_vars,
        get_args_from_local_config,
    },
};
use bat::style::StyleComponentList;
use bat::theme::{theme, ThemeName, ThemeOptions, ThemePreference};
use bat::BinaryBehavior;
use bat::StripAnsiMode;
use clap::ArgMatches;

use console::Term;

use crate::input::{new_file_input, new_stdin_input};
use bat::{
    bat_warning,
    config::{Config, VisibleLines},
    error::*,
    input::Input,
    line_range::{HighlightedLineRanges, LineRange, LineRanges},
    style::{StyleComponent, StyleComponents},
    MappingTarget, NonprintableNotation, PagingMode, SyntaxMapping, WrappingMode,
};

fn is_truecolor_terminal() -> bool {
    env::var("COLORTERM")
        .map(|colorterm| colorterm == "truecolor" || colorterm == "24bit")
        .unwrap_or(false)
}

pub fn env_no_color() -> bool {
    env::var_os("NO_COLOR").is_some_and(|x| !x.is_empty())
}

fn parse_strip_ansi_value(raw: Option<&str>, flag_name: &str) -> StripAnsiMode {
    match raw {
        Some("never") | None => StripAnsiMode::Never,
        Some("always") => StripAnsiMode::Always,
        Some("auto") => StripAnsiMode::Auto,
        _ => unreachable!("other values for {flag_name} are not allowed"),
    }
}

// Only reinterpret a missing literal path when the remaining path names a file.
// Existing colon names, symlinks and Windows alternate data streams take precedence.
fn split_file_position(path: &Path) -> Option<(PathBuf, usize)> {
    if std::fs::symlink_metadata(path).err()?.kind() != std::io::ErrorKind::NotFound {
        return None;
    }
    let (file, line) = path.to_str()?.rsplit_once(':')?;
    if line.is_empty() || !line.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let line = line.parse::<usize>().ok().filter(|&line| line > 0)?;
    let file = Path::new(file);
    file.is_file().then(|| (file.to_path_buf(), line))
}

enum HelpType {
    Short,
    Long,
}

pub struct App {
    pub matches: ArgMatches,
    interactive_output: bool,
    /// True if -n / --number was passed on the command line
    /// (not from config file or environment variables).
    /// This is used to honor the flag when piping output, similar to `cat -n`.
    number_from_cli: bool,
    /// True if -b / --number-nonblank was passed on the command line
    /// (not from config file or environment variables).
    /// This is used to honor the flag when piping output, similar to `cat -b`.
    number_nonblank_from_cli: bool,
    file_position: Option<(PathBuf, usize)>,
}

impl App {
    pub fn new() -> Result<Self> {
        #[cfg(windows)]
        let _ = nu_ansi_term::enable_ansi_support();

        let interactive_output = std::io::stdout().is_terminal();

        // Parse the original command line separately from config and environment arguments.
        // Using clap here keeps CLI-only detection consistent with the final parser, including
        // option terminators, combined short flags, and options that take values.
        let cli_matches = Self::cli_matches(interactive_output);
        let number_from_cli = cli_matches.get_flag("number");
        let number_nonblank_from_cli = cli_matches.get_flag("number-nonblank");

        let matches = Self::matches(
            interactive_output,
            cli_matches.get_flag("no-system-config"),
            cli_matches.get_flag("local-config"),
        )?;

        if matches.get_flag("help") {
            let help_type = if wild::args_os().any(|arg| arg == "--help") {
                HelpType::Long
            } else {
                HelpType::Short
            };

            Self::display_help(interactive_output, help_type, &matches)?;
            std::process::exit(0);
        }

        let file_positions = if matches.get_flag("literal-file-names") {
            Vec::new()
        } else {
            matches
                .get_many::<PathBuf>("FILE")
                .into_iter()
                .flatten()
                .filter_map(|path| split_file_position(path))
                .collect::<Vec<_>>()
        };
        if !file_positions.is_empty()
            && matches.get_many::<PathBuf>("FILE").map(|files| files.len()) != Some(1)
        {
            return Err("A file:line position requires exactly one input".into());
        }
        let file_position = file_positions.into_iter().next();

        Ok(App {
            file_position,
            matches,
            interactive_output,
            number_from_cli,
            number_nonblank_from_cli,
        })
    }

    fn display_help(
        interactive_output: bool,
        help_type: HelpType,
        matches: &ArgMatches,
    ) -> Result<()> {
        use crate::assets::assets_from_cache_or_binary;
        use crate::directories::PROJECT_DIRS;
        use bat::{
            config::Config,
            controller::Controller,
            input::Input,
            style::{StyleComponent, StyleComponents},
            theme::theme,
            PagingMode,
        };

        let use_pager = match matches.get_one::<String>("paging").map(|s| s.as_str()) {
            Some("never") => false,
            _ => !matches.get_flag("no-paging"),
        };
        let use_color = matches.get_flag("force-colorization")
            || match matches.get_one::<String>("color").map(|s| s.as_str()) {
                Some("always") => true,
                Some("never") => false,
                _ => interactive_output,
            };
        let pager = matches.get_one::<String>("pager").map(|s| s.as_str());
        let pager_args = matches
            .get_many::<String>("pager-arg")
            .map(|args| args.cloned().collect())
            .unwrap_or_default();

        let grayscale = matches.get_flag("grayscale");
        let mut cmd = clap_app::build_app(interactive_output);
        let help_text = match help_type {
            HelpType::Short => cmd.render_help().to_string(),
            HelpType::Long => cmd.render_long_help().to_string(),
        };

        let inputs: Vec<Input> = vec![Input::from_reader(Box::new(help_text.as_bytes()))];

        let paging_mode = if use_pager {
            PagingMode::QuitIfOneScreen
        } else {
            PagingMode::Never
        };

        let help_config = Config {
            style_components: StyleComponents::new(StyleComponent::Plain.components(false)),
            paging_mode,
            pager,
            pager_args,
            colored_output: use_color,
            true_color: use_color,
            grayscale,
            language: if use_color { Some("help") } else { None },
            theme: theme(Self::theme_options_from_matches(matches)).to_string(),
            theme_colors: Self::theme_colors_from_matches(matches)?,
            ..Default::default()
        };

        let cache_dir = PROJECT_DIRS.cache_dir();
        let assets = assets_from_cache_or_binary(!matches.get_flag("no-custom-assets"), cache_dir)?;
        Controller::new(&help_config, &assets)
            .run(inputs, None)
            .ok();

        Ok(())
    }

    /// Build argument list with env vars and CLI args (without config file)
    fn build_args_without_config() -> Vec<std::ffi::OsString> {
        let mut cli_args = wild::args_os();
        let mut args = get_args_from_env_vars();

        // Put the zero-th CLI argument (program name) first
        args.insert(0, cli_args.next().unwrap());

        // .. and the rest at the end
        cli_args.for_each(|a| args.push(a));

        args
    }

    fn cli_matches(interactive_output: bool) -> ArgMatches {
        clap_app::build_app(interactive_output).get_matches_from(wild::args_os())
    }

    fn matches(
        interactive_output: bool,
        skip_system_config: bool,
        use_local_config: bool,
    ) -> Result<ArgMatches> {
        // Check if we should skip config file processing for special arguments
        // that don't require full application setup (version, diagnostic)
        let should_skip_config = wild::args_os().any(|arg| {
            matches!(
                arg.to_str(),
                Some("-V" | "--version" | "--diagnostic" | "--diagnostics")
            )
        });

        // Check if help was requested - help should read the config file but be
        // forgiving of invalid arguments (so configured theme etc. can be used)
        let help_requested =
            wild::args_os().any(|arg| matches!(arg.to_str(), Some("-h" | "--help")));

        if wild::args_os().nth(1) == Some("cache".into()) {
            // Skip the config file and env vars
            let args = wild::args_os().collect::<Vec<_>>();
            return Ok(clap_app::build_app(interactive_output).get_matches_from(args));
        }

        if wild::args_os().any(|arg| arg == "--no-config") || should_skip_config {
            // Skip the arguments in bats config file when --no-config is present
            // or when user requests version or diagnostic information
            let args = Self::build_args_without_config();
            return Ok(clap_app::build_app(interactive_output).get_matches_from(args));
        }

        // Build arguments with config file
        let mut cli_args = wild::args_os();

        // Read arguments from bats config file
        let config_args = match get_args_from_env_opts_var() {
            Some(result) => result,
            None => get_args_from_config_file(skip_system_config),
        };

        // For help, ignore config file parse errors (use empty config instead)
        // For non-help, propagate the error
        let mut args = if help_requested {
            config_args.unwrap_or_default()
        } else {
            config_args?
        };

        if use_local_config {
            let local_args = get_args_from_local_config();
            args.extend(if help_requested {
                local_args.unwrap_or_default()
            } else {
                local_args?
            });
        }

        // Selected env vars supersede config vars
        args.extend(get_args_from_env_vars());

        // Put the zero-th CLI argument (program name) first
        args.insert(0, cli_args.next().unwrap());

        // .. and the rest at the end
        cli_args.for_each(|a| args.push(a));

        // For help, try parsing with config, and if clap fails (e.g., invalid
        // argument in config), fall back to parsing without config file args
        if help_requested {
            let app = clap_app::build_app(interactive_output);
            match app.try_get_matches_from(args) {
                Ok(matches) => Ok(matches),
                Err(_) => {
                    // Config has invalid arguments, fall back to just env vars + CLI args
                    let fallback_args = Self::build_args_without_config();
                    Ok(clap_app::build_app(interactive_output).get_matches_from(fallback_args))
                }
            }
        } else {
            Ok(clap_app::build_app(interactive_output).get_matches_from(args))
        }
    }

    pub fn config(&self, inputs: &[Input]) -> Result<Config<'_>> {
        let style_components = self.style_components(inputs)?;

        let extra_plain = self.matches.get_count("plain") > 1;
        let plain_last_index = self
            .matches
            .indices_of("plain")
            .and_then(Iterator::max)
            .unwrap_or_default();
        let paging_last_index = self
            .matches
            .indices_of("paging")
            .and_then(Iterator::max)
            .unwrap_or_default();

        let paging_mode = match self.matches.get_one::<String>("paging").map(|s| s.as_str()) {
            Some("always") => {
                // Disable paging if the second -p (or -pp) is specified after --paging=always
                if extra_plain && plain_last_index > paging_last_index {
                    PagingMode::Never
                } else {
                    PagingMode::Always
                }
            }
            Some("never") => PagingMode::Never,
            Some("auto") | None => {
                // If we have -pp as an option when in auto mode, the pager should be disabled.
                if extra_plain || self.matches.get_flag("no-paging") {
                    PagingMode::Never
                } else if inputs.iter().any(Input::is_stdin)
                    // ignore stdin when --list-themes is used because in that case no input will be read anyways
                    && !self.matches.get_flag("list-themes")
                {
                    // If we are reading from stdin, only enable paging if we write to an
                    // interactive terminal and if we do not *read* from an interactive
                    // terminal.
                    if self.interactive_output && !std::io::stdin().is_terminal() {
                        PagingMode::QuitIfOneScreen
                    } else {
                        PagingMode::Never
                    }
                } else if self.interactive_output {
                    PagingMode::QuitIfOneScreen
                } else {
                    PagingMode::Never
                }
            }
            _ => unreachable!("other values for --paging are not allowed"),
        };

        let mut syntax_mapping = SyntaxMapping::new();
        // start building glob matchers for builtin mappings immediately
        // this is an appropriate approach because it's statistically likely that
        // all the custom mappings need to be checked
        if available_parallelism()?.get() > 1 {
            syntax_mapping.start_offload_build_all();
        }

        if let Some(values) = self.matches.get_many::<String>("ignored-suffix") {
            for suffix in values {
                syntax_mapping.insert_ignored_suffix(suffix);
            }
        }

        if let Some(values) = self.matches.get_many::<String>("map-syntax") {
            // later args take precedence over earlier ones, hence `.rev()`
            // see: https://github.com/sharkdp/bat/pull/2755#discussion_r1456416875
            for from_to in values.rev() {
                // Preserve filename globs containing '=' when a ':' is present.
                if !from_to.contains(':') {
                    if let Some((alias, target)) = from_to.split_once('=') {
                        syntax_mapping.insert_language_alias(alias, target)?;
                        continue;
                    }
                }
                let parts: Vec<_> = from_to.split(':').collect();

                if parts.len() != 2 {
                    return Err("Invalid syntax mapping. The format of the -m/--map-syntax option is '<glob-pattern>:<syntax-name>'. For example: '*.cpp:C++'. Language aliases use '<alias>=<syntax-name>', for example 'csharp=C#'.".into());
                }

                syntax_mapping.insert(parts[0], MappingTarget::MapTo(parts[1]))?;
            }
        }

        let maybe_term_width = self
            .matches
            .get_one::<String>("terminal-width")
            .and_then(|w| {
                if w.starts_with('+') || w.starts_with('-') {
                    // Treat argument as a delta to the current terminal width
                    w.parse().ok().map(|delta: i16| {
                        let old_width: u16 = Term::stdout().size().1;
                        let new_width: i32 = i32::from(old_width) + i32::from(delta);

                        if new_width <= 0 {
                            old_width as usize
                        } else {
                            new_width as usize
                        }
                    })
                } else {
                    w.parse().ok()
                }
            });

        Ok(Config {
            true_color: is_truecolor_terminal(),
            grayscale: self.matches.get_flag("grayscale"),
            language: self
                .matches
                .get_one::<String>("language")
                .map(|s| s.as_str())
                .or_else(|| {
                    if self.matches.get_flag("show-all") {
                        Some("show-nonprintable")
                    } else {
                        None
                    }
                }),
            fallback_syntax: self
                .matches
                .get_one::<String>("fallback-syntax")
                .map(|s| s.as_str()),
            syntax_delimiter: self
                .matches
                .get_one::<regex::Regex>("syntax-delimiter")
                .cloned(),
            show_nonprintable: self.matches.get_flag("show-all"),
            nonprintable_notation: match self
                .matches
                .get_one::<String>("nonprintable-notation")
                .map(|s| s.as_str())
            {
                Some("unicode") => NonprintableNotation::Unicode,
                Some("caret") => NonprintableNotation::Caret,
                Some("symbols") => NonprintableNotation::Symbols,
                Some("period") => NonprintableNotation::Period,
                Some("binary") => NonprintableNotation::Binary,
                _ => unreachable!("other values for --nonprintable-notation are not allowed"),
            },
            binary: match self.matches.get_one::<String>("binary").map(|s| s.as_str()) {
                Some("as-text") => BinaryBehavior::AsText,
                Some("no-printing") => BinaryBehavior::NoPrinting,
                Some("skip") => BinaryBehavior::Skip,
                _ => unreachable!("other values for --binary are not allowed"),
            },
            wrapping_mode: {
                if self.matches.get_flag("chop-long-lines") {
                    WrappingMode::NoWrapping(true)
                } else {
                    match self.matches.get_one::<String>("wrap").map(|s| s.as_str()) {
                        Some("character") => WrappingMode::Character,
                        Some("word") => WrappingMode::Word,
                        Some("truncate") => WrappingMode::Truncate,
                        Some("never") => WrappingMode::NoWrapping(true),
                        Some("auto") | None => {
                            let has_sidebar = style_components.numbers();
                            #[cfg(feature = "git")]
                            let has_sidebar = has_sidebar || style_components.changes();

                            if self.interactive_output || maybe_term_width.is_some() {
                                if !has_sidebar && maybe_term_width.is_none() {
                                    WrappingMode::NoWrapping(false)
                                } else {
                                    WrappingMode::Character
                                }
                            } else {
                                // We don't have the tty width when piping to another program.
                                // There's no point in wrapping when this is the case.
                                WrappingMode::NoWrapping(false)
                            }
                        }
                        _ => unreachable!("other values for --wrap are not allowed"),
                    }
                }
            },
            colored_output: self.matches.get_flag("force-colorization")
                || match self.matches.get_one::<String>("color").map(|s| s.as_str()) {
                    Some("always") => true,
                    Some("never") => false,
                    Some("auto") => !env_no_color() && self.interactive_output,
                    _ => unreachable!("other values for --color are not allowed"),
                },
            paging_mode,
            scroll_to: self
                .matches
                .get_one::<usize>("scroll-to")
                .copied()
                .or_else(|| {
                    if self.matches.get_flag("center-highlight") {
                        None
                    } else {
                        self.file_position.as_ref().map(|(_, line)| *line)
                    }
                }),
            scroll_to_center: self.file_position.is_some()
                && self.matches.get_one::<usize>("scroll-to").is_none(),
            center_highlight: self.matches.get_flag("center-highlight"),
            term_width: maybe_term_width.unwrap_or(Term::stdout().size().1 as usize),
            loop_through: !(self.interactive_output
                || self.matches.get_one::<String>("color").map(|s| s.as_str()) == Some("always")
                || matches!(
                    self.matches
                        .get_one::<String>("decorations")
                        .map(String::as_str),
                    Some("always" | "compact")
                )
                || self.matches.get_flag("force-colorization")
                || self.number_from_cli
                || self.number_nonblank_from_cli
                || self.matches.get_one::<String>("wrap").map(|s| s.as_str()) == Some("truncate")),
            tab_width: self
                .matches
                .get_one::<String>("tabs")
                .map(String::from)
                .and_then(|t| t.parse().ok())
                .unwrap_or(
                    if style_components.plain() && paging_mode == PagingMode::Never {
                        0
                    } else {
                        4
                    },
                ),
            strip_ansi: {
                let sanitize = parse_strip_ansi_value(
                    self.matches
                        .get_one::<String>("sanitize")
                        .map(|s| s.as_str()),
                    "--sanitize",
                );
                let strip_ansi = parse_strip_ansi_value(
                    self.matches
                        .get_one::<String>("strip-ansi")
                        .map(|s| s.as_str()),
                    "--strip-ansi",
                );
                // --sanitize implies --strip-ansi to the same value.
                if sanitize != StripAnsiMode::Never {
                    sanitize
                } else {
                    strip_ansi
                }
            },
            sanitize: parse_strip_ansi_value(
                self.matches
                    .get_one::<String>("sanitize")
                    .map(|s| s.as_str()),
                "--sanitize",
            ),
            fail_if_syntax_unsupported: self.matches.get_flag("fail-if-syntax-unsupported"),
            quiet_empty: self.matches.get_flag("quiet-empty"),
            warn_missing_newline: self
                .matches
                .get_one::<String>("warning")
                .map(String::as_str)
                == Some("missing-trailing-newline"),
            max_bytes: self.matches.get_one::<u64>("max-bytes").copied(),
            unbuffered: self.matches.get_flag("unbuffered"),
            number_nonblank: self.matches.get_flag("number-nonblank")
                || self.number_nonblank_from_cli,
            theme: theme(self.theme_options()).to_string(),
            theme_colors: Self::theme_colors_from_matches(&self.matches)?,
            visible_lines: match self.matches.try_contains_id("diff").unwrap_or_default()
                && self.matches.get_flag("diff")
            {
                #[cfg(feature = "git")]
                true => VisibleLines::DiffContext(
                    self.matches
                        .get_one::<String>("diff-context")
                        .and_then(|t| t.parse().ok())
                        .unwrap_or(2),
                ),

                _ => VisibleLines::Ranges(
                    self.matches
                        .get_many::<String>("line-range")
                        .map(|vs| vs.map(|s| LineRange::from(s.as_str())).collect())
                        .transpose()?
                        .map(LineRanges::from)
                        .unwrap_or_default(),
                ),
            },
            styles_for_syntax: self.styles_for_syntax(&style_components)?,
            style_components,
            compact_headers: self
                .matches
                .get_one::<String>("decorations")
                .map(String::as_str)
                == Some("compact"),
            syntax_mapping,
            pager: self.matches.get_one::<String>("pager").map(|s| s.as_str()),
            pager_args: self
                .matches
                .get_many::<String>("pager-arg")
                .map(|args| args.cloned().collect())
                .unwrap_or_default(),
            use_italic_text: self
                .matches
                .get_one::<String>("italic-text")
                .map(|s| s.as_str())
                == Some("always"),
            hyperlink: if self.matches.get_flag("osc8") || self.matches.get_flag("osc8-highlight") {
                Some(bat::hyperlink::Hyperlink::new(
                    self.matches
                        .get_one::<String>("hyperlink-format")
                        .expect("default format"),
                    self.matches.get_flag("osc8-highlight"),
                )?)
            } else {
                None
            },

            use_theme_background: self
                .matches
                .get_one::<String>("theme-background")
                .map(|s| s.as_str())
                == Some("always"),
            highlighted_lines: self
                .matches
                .get_many::<String>("highlight-line")
                .map(|ws| {
                    ws.filter(|s| !s.contains('.'))
                        .map(|s| LineRange::from(s.as_str()))
                        .collect()
                })
                .transpose()?
                .map(LineRanges::from)
                .map(HighlightedLineRanges)
                .unwrap_or_default(),
            highlighted_patterns: self
                .matches
                .get_many::<regex::Regex>("highlight-pattern")
                .map(|patterns| patterns.cloned().collect())
                .unwrap_or_default(),
            highlighted_regions: self
                .matches
                .get_many::<String>("highlight-line")
                .into_iter()
                .flatten()
                .filter(|s| s.contains('.'))
                .map(|s| s.parse())
                .collect::<Result<Vec<_>>>()?,
            use_custom_assets: !self.matches.get_flag("no-custom-assets"),
            #[cfg(feature = "lessopen")]
            use_lessopen: self.matches.get_flag("lessopen"),
            set_terminal_title: self.matches.get_flag("set-terminal-title"),
            squeeze_lines: if self.matches.get_flag("squeeze-blank") {
                Some(
                    self.matches
                        .get_one::<usize>("squeeze-limit")
                        .map(|limit| limit.to_owned())
                        .unwrap_or(1),
                )
            } else {
                None
            },
        })
    }

    /// Display argument values after config files, environment, and CLI parsing.
    /// Keep automatic modes and ordered repeatable values as configured.
    pub fn show_config(&self, field: &str) -> Result<String> {
        use clap::parser::ValueSource;
        use std::fmt::Write as _;
        let app = clap_app::build_app(self.interactive_output);
        let excluded = [
            "help",
            "version",
            "show-config",
            "no-config",
            "completion",
            "diagnostic",
            "list-languages",
            "list-themes",
            "config-file",
            "generate-config-file",
            "config-dir",
            "cache-dir",
            "acknowledgements",
        ];
        let mut fields = app
            .get_arguments()
            .filter_map(|arg| arg.get_long().map(|name| (name, arg.get_id().as_str())))
            .filter(|(_, id)| !excluded.contains(id))
            .collect::<Vec<_>>();
        fields.sort_unstable();
        let mut output = String::new();
        if field != "*" {
            let id = fields
                .iter()
                .find(|(name, _)| *name == field)
                .map(|(_, id)| *id)
                .ok_or_else(|| format!("unknown configuration field '{field}'"))?;
            if let Some(values) = self.matches.get_raw(id) {
                for value in values {
                    writeln!(
                        output,
                        "{}",
                        bat::sanitize_for_terminal(&value.to_string_lossy())
                    )?;
                }
            }
            return Ok(output);
        }
        for (name, id) in fields {
            // Hide parser defaults when listing the user's configured values.
            if self.matches.value_source(id) == Some(ValueSource::DefaultValue) {
                continue;
            }
            if let Some(values) = self.matches.get_raw(id) {
                for value in values {
                    writeln!(
                        output,
                        "{name}: {}",
                        bat::sanitize_for_terminal(&value.to_string_lossy())
                    )?;
                }
            }
        }
        Ok(output)
    }

    pub fn inputs(&self) -> Result<Vec<Input<'_>>> {
        let command = self
            .matches
            .get_one::<crate::process::ProcessCommand>("process");
        let new_stdin_input = |name| {
            let input = new_stdin_input(name);
            if let Some(command) = command {
                input.with_reader(Box::new(command.reader(None)))
            } else {
                input
            }
        };
        let new_file_input = |file, name| {
            let input = new_file_input(file, name);
            if let Some(command) = command {
                input.with_reader(Box::new(command.reader(Some(file))))
            } else {
                input
            }
        };

        let filenames: Option<Vec<&Path>> = self
            .matches
            .get_many::<PathBuf>("file-name")
            .map(|vs| vs.map(|p| p.as_path()).collect::<Vec<_>>());

        let files: Option<Vec<&Path>> = self
            .matches
            .get_many::<PathBuf>("FILE")
            .map(|vs| vs.map(|p| p.as_path()).collect::<Vec<_>>());

        // verify equal length of file-names and input FILEs
        if filenames.is_some()
            && files.is_some()
            && filenames.as_ref().map(|v| v.len()) != files.as_ref().map(|v| v.len())
        {
            return Err("Must be one file name per input type.".into());
        }

        let mut filenames_or_none: Box<dyn Iterator<Item = Option<&Path>>> = match filenames {
            Some(filenames) => Box::new(filenames.into_iter().map(Some)),
            None => Box::new(std::iter::repeat(None)),
        };
        if files.is_none() {
            return Ok(vec![new_stdin_input(
                filenames_or_none.next().unwrap_or(None),
            )]);
        }
        let files_or_none: Box<dyn Iterator<Item = _>> = match files {
            Some(ref files) => Box::new(files.iter().map(|name| Some(*name))),
            None => Box::new(std::iter::repeat(None)),
        };

        let mut file_input = Vec::new();
        for (filepath, provided_name) in files_or_none.zip(filenames_or_none) {
            if let Some(filepath) = filepath {
                if filepath.to_str().unwrap_or_default() == "-" {
                    file_input.push(new_stdin_input(provided_name));
                } else {
                    let filepath = self
                        .file_position
                        .as_ref()
                        .map_or(filepath, |(path, _)| path.as_path());
                    file_input.push(new_file_input(filepath, provided_name));
                }
            }
        }
        Ok(file_input)
    }

    fn forced_style_components(&self) -> Option<StyleComponents> {
        // No components if `--decorations=never``.
        if self
            .matches
            .get_one::<String>("decorations")
            .map(|s| s.as_str())
            == Some("never")
        {
            return Some(StyleComponents(HashSet::new()));
        }

        // Only line numbers if `--number`.
        if self.matches.get_flag("number") {
            return Some(StyleComponents(HashSet::from([
                StyleComponent::LineNumbers,
            ])));
        }

        // Only line numbers for non-blank lines if `--number-nonblank`.
        if self.matches.get_flag("number-nonblank") || self.number_nonblank_from_cli {
            return Some(StyleComponents(HashSet::from([
                StyleComponent::LineNumbers,
            ])));
        }

        // Plain if `--plain` is specified at least once.
        if self.matches.get_count("plain") > 0 {
            let mut components = HashSet::from([StyleComponent::Plain]);
            // When --diff is active, preserve change markers and snip separators
            // so that diff output remains visually useful.
            if self.matches.try_contains_id("diff").unwrap_or_default()
                && self.matches.get_flag("diff")
            {
                #[cfg(feature = "git")]
                components.insert(StyleComponent::Changes);
                components.insert(StyleComponent::Snip);
            }
            return Some(StyleComponents(components));
        }

        // Default behavior.
        None
    }

    fn style_components(&self, inputs: &[Input]) -> Result<StyleComponents> {
        let matches = &self.matches;
        let context = if inputs.len() > 1 {
            "style-multiple-files"
        } else if inputs.first().is_some_and(Input::is_stdin) {
            "style-stdin"
        } else {
            "style-single-file"
        };
        let mut styled_components = match self.forced_style_components() {
            Some(mut forced_components) => {
                // Number/plain shortcuts choose decorations; sidebar-right only
                // changes their placement and does not enable another decoration.
                if let Some(styles) = matches.get_many::<String>("style") {
                    let lists = styles
                        .map(|style| StyleComponentList::from_str(style))
                        .collect::<Result<Vec<_>>>()?;
                    if StyleComponentList::to_components(lists, self.interactive_output, false)
                        .sidebar_right()
                    {
                        forced_components.insert(StyleComponent::SidebarRight);
                    }
                }
                forced_components
            }

            // Parse the `--style` arguments and merge them.
            None if matches.contains_id("style") || matches.contains_id(context) => {
                let lists = matches
                    .get_many::<String>("style")
                    .into_iter()
                    .flatten()
                    .chain(matches.get_many::<String>(context).into_iter().flatten())
                    .map(|v| StyleComponentList::from_str(v))
                    .collect::<Result<Vec<StyleComponentList>>>()?;

                StyleComponentList::to_components(lists, self.interactive_output, true)
            }

            // Use the default.
            None => StyleComponents(HashSet::from_iter(
                StyleComponent::Default
                    .components(self.interactive_output)
                    .iter()
                    .cloned(),
            )),
        };

        // If `grid` is set, remove `rule` as it is a subset of `grid`, and print a warning.
        if styled_components.grid() && styled_components.0.remove(&StyleComponent::Rule) {
            bat_warning!("Style 'rule' is a subset of style 'grid', 'rule' will not be visible.");
        }

        // Auto-disable line numbers in unbuffered mode to avoid confusion with partial lines
        if self.matches.get_flag("unbuffered") {
            styled_components.0.remove(&StyleComponent::LineNumbers);
        }

        Ok(styled_components)
    }

    fn styles_for_syntax(
        &self,
        general: &StyleComponents,
    ) -> Result<Vec<(String, StyleComponents)>> {
        let mut styles: Vec<(String, StyleComponents)> = Vec::new();
        if let Some(values) = self.matches.get_many::<String>("style-for") {
            let values: Vec<_> = values.collect();
            for pair in values.as_chunks::<2>().0 {
                let language = pair[0];
                if language.is_empty() {
                    return Err("The language for --style-for cannot be empty".into());
                }
                let list = StyleComponentList::from_str(pair[1])?;
                let index = styles
                    .iter()
                    .position(|(name, _)| name.eq_ignore_ascii_case(language));
                let components = if let Some(index) = index {
                    &mut styles[index].1
                } else {
                    styles.push((language.clone(), general.clone()));
                    &mut styles.last_mut().unwrap().1
                };
                list.apply_to(components, self.interactive_output);
            }
        }
        for (_, components) in &mut styles {
            if let Some(forced) = self.forced_style_components() {
                let sidebar_right = components.sidebar_right();
                *components = forced;
                if sidebar_right {
                    components.insert(StyleComponent::SidebarRight);
                }
            }
            if components.grid() {
                components.0.remove(&StyleComponent::Rule);
            }
            if self.matches.get_flag("unbuffered") {
                components.0.remove(&StyleComponent::LineNumbers);
            }
        }
        Ok(styles)
    }

    pub(crate) fn theme_options(&self) -> ThemeOptions {
        Self::theme_options_from_matches(&self.matches)
    }

    fn theme_colors_from_matches(matches: &ArgMatches) -> Result<bat::theme::ThemeColorOverrides> {
        let mut colors = bat::theme::ThemeColorOverrides::default();
        if let Some(mut values) = matches.get_many::<String>("set-theme-color") {
            while let (Some(name), Some(value)) = (values.next(), values.next()) {
                colors.set(name, value)?;
            }
        }
        Ok(colors)
    }

    fn theme_options_from_matches(matches: &ArgMatches) -> ThemeOptions {
        let theme = matches
            .get_one::<String>("theme")
            .map(|t| ThemePreference::from_str(t).unwrap())
            .unwrap_or_default();
        let theme_dark = matches
            .get_one::<String>("theme-dark")
            .map(|t| ThemeName::from_str(t).unwrap());
        let theme_light = matches
            .get_one::<String>("theme-light")
            .map(|t| ThemeName::from_str(t).unwrap());
        ThemeOptions {
            theme,
            theme_dark,
            theme_light,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wrapping_mode(args: &[&str], interactive_output: bool) -> WrappingMode {
        let app = App {
            file_position: None,
            matches: clap_app::build_app(interactive_output).get_matches_from(
                ["bat", "--paging=never"]
                    .into_iter()
                    .chain(args.iter().copied()),
            ),
            interactive_output,
            number_from_cli: false,
            number_nonblank_from_cli: false,
        };
        let wrapping_mode = app.config(&[]).unwrap().wrapping_mode;
        wrapping_mode
    }

    #[test]
    fn automatic_wrapping_leaves_non_sidebar_styles_to_the_terminal() {
        for style in ["plain", "header", "grid", "rule,snip", "header,grid,snip"] {
            assert_eq!(
                wrapping_mode(&[&format!("--style={style}")], true),
                WrappingMode::NoWrapping(false),
                "{style}"
            );
        }
        assert_eq!(
            wrapping_mode(&["--style=numbers"], true),
            WrappingMode::Character
        );
        #[cfg(feature = "git")]
        assert_eq!(
            wrapping_mode(&["--style=changes"], true),
            WrappingMode::Character
        );
    }

    #[test]
    fn automatic_wrapping_preserves_explicit_width_and_wrap_requests() {
        for interactive_output in [false, true] {
            assert_eq!(
                wrapping_mode(
                    &["--style=header", "--terminal-width=20"],
                    interactive_output
                ),
                WrappingMode::Character
            );
            assert_eq!(
                wrapping_mode(&["--style=header", "--wrap=character"], interactive_output),
                WrappingMode::Character
            );
            assert_eq!(
                wrapping_mode(&["--style=header", "--wrap=word"], interactive_output),
                WrappingMode::Word
            );
            assert_eq!(
                wrapping_mode(&["--style=numbers", "--wrap=never"], interactive_output),
                WrappingMode::NoWrapping(true)
            );
        }
        assert_eq!(
            wrapping_mode(&["--style=numbers"], false),
            WrappingMode::NoWrapping(false)
        );
    }
}
