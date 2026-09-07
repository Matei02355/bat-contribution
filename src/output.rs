use std::fmt;
use std::io;
#[cfg(all(feature = "paging", not(target_os = "wasi")))]
use std::process::Child;
#[cfg(all(feature = "paging", not(target_os = "wasi")))]
use std::thread::{spawn, JoinHandle};

use crate::error::*;
#[cfg(all(feature = "paging", not(target_os = "wasi")))]
use crate::less::{retrieve_less_version, LessVersion};
#[cfg(feature = "paging")]
use crate::paging::PagingMode;
#[cfg(feature = "paging")]
use crate::wrapping::WrappingMode;

#[cfg(all(feature = "paging", not(target_os = "wasi")))]
fn prompt_filename(filename: &str) -> String {
    crate::preprocessor::sanitize_for_terminal(filename).replace('\t', "^I")
}

#[cfg(all(feature = "paging", not(target_os = "wasi")))]
fn less_filename_prompts(filename: &str) -> [String; 4] {
    let mut escaped = String::new();
    for character in prompt_filename(filename).chars() {
        if matches!(character, '?' | ':' | '.' | '%' | '\\') {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    // If the user opens another file with :e, less knows its real name.
    let name = format!("?f%f:{escaped}.");
    let detail = " ?ltlines %lt-%lb?L/%L.. ?e(END).%t";
    [
        format!("s{name} ?e(END).%t"),
        format!("m{name}{detail}"),
        format!("M{name}{detail}"),
        format!("={name}{detail}"),
    ]
}

#[cfg(all(feature = "paging", not(target_os = "wasi")))]
pub struct BuiltinPager {
    pager: minus::Pager,
    handle: Option<JoinHandle<Result<()>>>,
}

#[cfg(all(feature = "paging", not(target_os = "wasi")))]
impl BuiltinPager {
    fn new(filename: Option<&str>) -> Self {
        let pager = minus::Pager::new();
        if let Some(filename) = filename {
            pager
                .set_prompt(prompt_filename(filename))
                .expect("failed to set prompt on newly created pager");
        }

        let mut input_register = minus::input::HashedEventRegister::default();
        input_register.add_key_events(&["home"], |_, _| {
            minus::input::InputEvent::UpdateUpperMark(0)
        });
        input_register.add_key_events(&["end"], |_, _| {
            minus::input::InputEvent::UpdateUpperMark(usize::MAX)
        });
        pager
            .set_input_classifier(Box::new(input_register))
            .expect("failed to set input classifier on newly created pager");

        let handle = {
            let pager = pager.clone();
            Some(spawn(move || {
                minus::dynamic_paging(pager).map_err(Error::from)
            }))
        };
        Self { pager, handle }
    }
}

#[cfg(all(feature = "paging", not(target_os = "wasi")))]
impl std::fmt::Debug for BuiltinPager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BuiltinPager")
            //.field("pager", &self.pager) /// minus::Pager doesn't implement fmt::Debug
            .field("handle", &self.handle)
            .finish()
    }
}

#[cfg(all(feature = "paging", not(target_os = "wasi")))]
#[derive(Debug, PartialEq)]
enum SingleScreenAction {
    Quit,
    Nothing,
}

#[derive(Debug)]
pub enum OutputType {
    #[cfg(all(feature = "paging", not(target_os = "wasi")))]
    Pager(Child),
    #[cfg(all(feature = "paging", not(target_os = "wasi")))]
    BuiltinPager(BuiltinPager),
    Stdout(io::Stdout),
}

impl OutputType {
    #[cfg(all(feature = "paging", target_os = "wasi"))]
    pub fn from_mode(
        paging_mode: crate::paging::PagingMode,
        _wrapping_mode: crate::wrapping::WrappingMode,
        _pager: Option<&str>,
    ) -> Result<Self> {
        if paging_mode == crate::paging::PagingMode::Always {
            return Err("Paging is unavailable in WASI; use --paging=never".into());
        }
        Ok(Self::stdout())
    }

    #[cfg(all(feature = "paging", not(target_os = "wasi")))]
    pub fn from_mode(
        paging_mode: PagingMode,
        wrapping_mode: WrappingMode,
        pager: Option<&str>,
    ) -> Result<Self> {
        Self::from_mode_with_args(paging_mode, wrapping_mode, pager, &[])
    }

    /// Select output and append literal arguments to an external pager.
    #[cfg(feature = "paging")]
    pub fn from_mode_with_args(
        paging_mode: PagingMode,
        wrapping_mode: WrappingMode,
        pager: Option<&str>,
        pager_args: &[String],
    ) -> Result<Self> {
        Self::from_mode_with_args_and_reserve(paging_mode, wrapping_mode, pager, pager_args, 0)
    }

    /// Select output while reserving rows from automatic less paging.
    #[cfg(feature = "paging")]
    pub fn from_mode_with_reserve(
        paging_mode: PagingMode,
        wrapping_mode: WrappingMode,
        pager: Option<&str>,
        reserve: u16,
    ) -> Result<Self> {
        Self::from_mode_with_args_and_reserve(paging_mode, wrapping_mode, pager, &[], reserve)
    }

    /// Append literal pager arguments and reserve automatic paging rows.
    #[cfg(feature = "paging")]
    pub fn from_mode_with_args_and_reserve(
        paging_mode: PagingMode,
        wrapping_mode: WrappingMode,
        pager: Option<&str>,
        pager_args: &[String],
        reserve: u16,
    ) -> Result<Self> {
        Self::from_mode_with_args_and_filename(
            paging_mode,
            wrapping_mode,
            pager,
            pager_args,
            None,
            reserve,
        )
    }

    #[cfg(feature = "paging")]
    pub(crate) fn from_mode_with_args_and_filename(
        paging_mode: PagingMode,
        wrapping_mode: WrappingMode,
        pager: Option<&str>,
        pager_args: &[String],
        filename: Option<&str>,
        reserve: u16,
    ) -> Result<Self> {
        Self::from_mode_at(
            paging_mode,
            wrapping_mode,
            pager,
            pager_args,
            filename,
            None,
            reserve,
        )
    }

    #[cfg(feature = "paging")]
    pub(crate) fn from_mode_at(
        paging_mode: PagingMode,
        wrapping_mode: WrappingMode,
        pager: Option<&str>,
        pager_args: &[String],
        filename: Option<&str>,
        start: Option<crate::scroll::PagerStart>,
        reserve: u16,
    ) -> Result<Self> {
        #[cfg(target_os = "wasi")]
        {
            let _ = (pager_args, filename, start, reserve);
            Self::from_mode(paging_mode, wrapping_mode, pager)
        }
        #[cfg(not(target_os = "wasi"))]
        {
            use self::PagingMode::*;
            Ok(match paging_mode {
                Always => OutputType::try_pager(
                    SingleScreenAction::Nothing,
                    wrapping_mode,
                    pager,
                    pager_args,
                    filename,
                    start,
                    0,
                )?,
                QuitIfOneScreen => OutputType::try_pager(
                    SingleScreenAction::Quit,
                    wrapping_mode,
                    pager,
                    pager_args,
                    filename,
                    start,
                    reserve,
                )?,
                _ => OutputType::stdout(),
            })
        }
    }

    /// Try to launch the pager. Fall back to stdout in case of errors.
    #[cfg(all(feature = "paging", not(target_os = "wasi")))]
    fn try_pager(
        single_screen_action: SingleScreenAction,
        wrapping_mode: WrappingMode,
        pager_from_config: Option<&str>,
        pager_args: &[String],
        filename: Option<&str>,
        start: Option<crate::scroll::PagerStart>,
        reserve: u16,
    ) -> Result<Self> {
        use crate::pager::{self, PagerKind, PagerSource};
        use std::process::{Command, Stdio};

        let pager_opt =
            pager::get_pager(pager_from_config).map_err(|_| "Could not parse pager command.")?;

        let pager = match pager_opt {
            Some(pager) => pager,
            None => return Ok(OutputType::stdout()),
        };

        if pager.kind == PagerKind::Bat {
            return Err(Error::InvalidPagerValueBat);
        }

        let reserved_version = if reserve > 0 {
            if pager.kind != PagerKind::Less {
                return Err("--paging-reserve requires less 632 or newer".into());
            }
            match retrieve_less_version(&pager.bin) {
                Some(LessVersion::Less(version)) if version >= 632 => Some(version),
                _ => return Err("--paging-reserve requires less 632 or newer".into()),
            }
        } else {
            None
        };
        if pager.kind == PagerKind::Builtin {
            if !pager_args.is_empty() {
                return Err("The built-in pager does not accept additional arguments".into());
            }
            return Ok(OutputType::BuiltinPager(BuiltinPager::new(filename)));
        }

        let args = pager.args;

        let child = pager::run_command(pager.bin.as_ref(), |program| {
            let mut p = Command::new(program);
            if pager.kind == PagerKind::Less {
                let less_version = if let Some(version) = reserved_version {
                    Some(LessVersion::Less(version))
                } else if filename.is_some()
                    || args.is_empty()
                    || pager.source == PagerSource::EnvVarPager
                {
                    retrieve_less_version(&pager.bin)
                } else {
                    None
                };
                if reserve > 0 {
                    let height = console::Term::stdout().size().0;
                    let rows = if reserve < height {
                        format!("-{reserve}")
                    } else {
                        "1".to_owned()
                    };
                    p.env("LESS_LINES", rows);
                }
                let less_options = std::env::var("LESS").unwrap_or_default();
                if let (Some(filename), Some(LessVersion::Less(_))) = (filename, &less_version) {
                    // LESS is a separate option language, not shell words. Be conservative
                    // when it might contain a user prompt; do not replace that prompt.
                    if !less_options.contains('P') && !less_options.contains("prompt") {
                        for prompt in less_filename_prompts(filename) {
                            // Keep the value in a separate argument. In a combined -Pvalue
                            // option, less treats '$' inside a filename as an option separator.
                            p.arg("-P").arg(prompt);
                        }
                    }
                }
                // less needs to be called with the '-R' option in order to properly interpret the
                // ANSI color sequences printed by bat. If someone has set PAGER="less -F", we
                // therefore need to overwrite the arguments and add '-R'.
                //
                // We only do this for PAGER (as it is not specific to 'bat'), not for BAT_PAGER
                // or bats '--pager' command line option.
                let replace_arguments_to_less = pager.source == PagerSource::EnvVarPager;

                if args.is_empty() || replace_arguments_to_less {
                    p.arg("-R"); // Short version of --RAW-CONTROL-CHARS for maximum compatibility
                    if single_screen_action == SingleScreenAction::Quit {
                        p.arg("-F"); // Short version of --quit-if-one-screen for compatibility
                    }

                    if wrapping_mode == WrappingMode::NoWrapping(true) {
                        p.arg("-S"); // Short version of --chop-long-lines for compatibility
                    }

                    // Ensures that 'less' quits together with 'bat'
                    // The BusyBox version of less does not support -K
                    if less_version != Some(LessVersion::BusyBox) {
                        p.arg("-K"); // Short version of '--quit-on-intr'
                    }

                    // Passing '--no-init' fixes a bug with '--quit-if-one-screen' in older
                    // versions of 'less'. Unfortunately, it also breaks mouse-wheel support.
                    //
                    // See: http://www.greenwoodsoftware.com/less/news.530.html
                    //
                    // For newer versions (530 or 558 on Windows), we omit '--no-init' as it
                    // is not needed anymore.
                    if single_screen_action == SingleScreenAction::Quit {
                        match less_version {
                            None => {
                                p.arg("--no-init");
                            }
                            Some(LessVersion::Less(version))
                                if (version < 530 || (cfg!(windows) && version < 558)) =>
                            {
                                p.arg("--no-init");
                            }
                            _ => {}
                        }
                    }
                } else {
                    p.args(&args);
                }
                if reserve > 0 {
                    p.arg("-F");
                }
                p.env("LESSCHARSET", "UTF-8");

                #[cfg(feature = "lessopen")]
                // Ensures that 'less' does not preprocess input again if '$LESSOPEN' is set.
                p.arg("--no-lessopen");
            } else {
                p.args(&args);
            };

            p.args(pager_args);

            if let Some(start) = start {
                start.configure(&mut p, &pager.kind);
            }
            p.stdin(Stdio::piped()).spawn()
        });
        Ok(child.map(OutputType::Pager).unwrap_or_else(|_| {
            crate::bat_warning!(
                "Pager '{}' not found, outputting to stdout instead",
                &pager.bin
            );
            OutputType::stdout()
        }))
    }

    pub(crate) fn stdout() -> Self {
        OutputType::Stdout(io::stdout())
    }

    #[cfg(all(feature = "paging", not(target_os = "wasi")))]
    pub(crate) fn is_pager(&self) -> bool {
        matches!(self, OutputType::Pager(_) | OutputType::BuiltinPager(_))
    }

    #[cfg(any(not(feature = "paging"), target_os = "wasi"))]
    pub(crate) fn is_pager(&self) -> bool {
        false
    }

    pub fn handle<'a>(&'a mut self) -> Result<OutputHandle<'a>> {
        Ok(match *self {
            #[cfg(all(feature = "paging", not(target_os = "wasi")))]
            OutputType::Pager(ref mut command) => OutputHandle::IoWrite(
                command
                    .stdin
                    .as_mut()
                    .ok_or("Could not open stdin for pager")?,
            ),
            #[cfg(all(feature = "paging", not(target_os = "wasi")))]
            OutputType::BuiltinPager(ref mut pager) => OutputHandle::FmtWrite(&mut pager.pager),
            OutputType::Stdout(ref mut handle) => OutputHandle::IoWrite(handle),
        })
    }
}

#[cfg(all(feature = "paging", not(target_os = "wasi")))]
impl Drop for OutputType {
    fn drop(&mut self) {
        match *self {
            OutputType::Pager(ref mut command) => {
                let _ = command.wait();
            }
            OutputType::BuiltinPager(ref mut pager) => {
                if let Some(handle) = pager.handle.take() {
                    let _ = handle.join();
                }
            }
            OutputType::Stdout(_) => (),
        }
    }
}

pub enum OutputHandle<'a> {
    IoWrite(&'a mut dyn io::Write),
    FmtWrite(&'a mut dyn fmt::Write),
}

impl OutputHandle<'_> {
    pub fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> Result<()> {
        match self {
            Self::IoWrite(handle) => handle.write_fmt(args).map_err(Into::into),
            Self::FmtWrite(handle) => handle.write_fmt(args).map_err(Into::into),
        }
    }

    pub fn flush(&mut self) -> Result<()> {
        match self {
            Self::IoWrite(handle) => handle.flush().map_err(Into::into),
            Self::FmtWrite(_) => Ok(()),
        }
    }
}
