use std::io::Read;
use std::path::Path;

use console::Term;

use crate::{
    assets::HighlightingAssets,
    config::{Config, VisibleLines},
    controller::Controller,
    error::Result,
    input,
    line_range::{HighlightedLineRanges, LineRange, LineRanges},
    output::OutputHandle,
    style::StyleComponent,
    StripAnsiMode, SyntaxMapping, WrappingMode,
};

#[cfg(feature = "paging")]
use crate::paging::PagingMode;

#[derive(Default)]
struct ActiveStyleComponents {
    header_filename: bool,
    header_path: bool,
    header_modified: bool,
    header_permissions: bool,
    #[cfg(feature = "git")]
    vcs_modification_markers: bool,
    #[cfg(feature = "git")]
    vcs_modification_highlighting: bool,
    git_blame: bool,
    grid: bool,
    grid_vertical: bool,
    rule: bool,
    line_numbers: bool,
    highlight_indicator: bool,
    sidebar_right: bool,
    snip: bool,
}

#[non_exhaustive]
pub struct Syntax {
    pub name: String,
    pub file_extensions: Vec<String>,
}

pub struct PrettyPrinter<'a> {
    inputs: Vec<Input<'a>>,
    config: Config<'a>,
    assets: HighlightingAssets,

    highlighted_lines: Vec<LineRange>,
    term_width: Option<usize>,
    active_style_components: ActiveStyleComponents,
}

impl<'a> PrettyPrinter<'a> {
    pub fn new() -> Self {
        let config = Config {
            colored_output: true,
            true_color: true,
            ..Default::default()
        };

        PrettyPrinter {
            inputs: vec![],
            config,
            assets: HighlightingAssets::from_binary(),

            highlighted_lines: vec![],
            term_width: None,
            active_style_components: ActiveStyleComponents::default(),
        }
    }

    /// Create a printer using an existing asset cache, such as one built by
    /// `bat cache --build`. The directory is explicit: this does not read user
    /// configuration or environment variables.
    ///
    /// Invalid or missing caches return an error during construction.
    pub fn from_cache(cache_path: impl AsRef<Path>) -> Result<Self> {
        Self::with_assets(HighlightingAssets::from_cache(cache_path.as_ref())?)
    }

    /// Create a printer with custom syntax and theme assets.
    ///
    /// Syntaxes are loaded immediately so malformed caches return an error
    /// before this printer is used or its syntaxes are enumerated.
    pub fn with_assets(assets: HighlightingAssets) -> Result<Self> {
        assets.get_syntaxes()?;
        Ok(Self {
            assets,
            ..Self::new()
        })
    }

    /// Select decorations for a full syntax name, matched case-insensitively.
    /// Wrapping and tab settings remain global. Later calls for the same syntax win.
    pub fn style_for(
        &mut self,
        language: impl Into<String>,
        components: &[StyleComponent],
    ) -> &mut Self {
        self.config.styles_for_syntax.push((
            language.into(),
            crate::style::StyleComponents(
                components
                    .iter()
                    .flat_map(|component| component.components(true))
                    .copied()
                    .collect(),
            ),
        ));
        self
    }

    /// Add an input which should be pretty-printed
    pub fn input(&mut self, input: Input<'a>) -> &mut Self {
        self.inputs.push(input);
        self
    }

    /// Adds multiple inputs which should be pretty-printed
    pub fn inputs(&mut self, inputs: impl IntoIterator<Item = Input<'a>>) -> &mut Self {
        for input in inputs {
            self.inputs.push(input);
        }
        self
    }

    /// Add a file which should be pretty-printed
    pub fn input_file(&mut self, path: impl AsRef<Path>) -> &mut Self {
        self.input(Input::from_file(path).kind("File"))
    }

    /// Add multiple files which should be pretty-printed
    pub fn input_files<I, P>(&mut self, paths: I) -> &mut Self
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        self.inputs(paths.into_iter().map(Input::from_file))
    }

    /// Add STDIN as an input
    pub fn input_stdin(&mut self) -> &mut Self {
        self.inputs.push(Input::from_stdin());
        self
    }

    /// Add a byte string as an input
    pub fn input_from_bytes(&mut self, content: &'a [u8]) -> &mut Self {
        self.input_from_reader(content)
    }

    /// Add a custom reader as an input
    pub fn input_from_reader<R: Read + 'a>(&mut self, reader: R) -> &mut Self {
        self.inputs.push(Input::from_reader(reader));
        self
    }

    /// Specify the syntax file which should be used (default: auto-detect)
    pub fn language(&mut self, language: &'a str) -> &mut Self {
        self.config.language = Some(language);
        self
    }

    /// Reset highlighting before lines matching the given regular expression.
    pub fn syntax_delimiter(&mut self, pattern: &str) -> Result<&mut Self> {
        self.config.syntax_delimiter = Some(
            regex::Regex::new(pattern)
                .map_err(|error| format!("Invalid syntax delimiter: {error}"))?,
        );
        Ok(self)
    }

    /// The character width of the terminal (default: autodetect)
    pub fn term_width(&mut self, width: usize) -> &mut Self {
        self.term_width = Some(width);
        self
    }

    /// The width of tab characters (default: None - do not turn tabs to spaces)
    pub fn tab_width(&mut self, tab_width: Option<usize>) -> &mut Self {
        self.config.tab_width = tab_width.unwrap_or(0);
        self
    }

    /// Whether or not the output should be colorized (default: true)
    pub fn colored_output(&mut self, yes: bool) -> &mut Self {
        self.config.colored_output = yes;
        self
    }

    /// Reject inputs without a specific syntax and disable paging.
    pub fn fail_if_syntax_unsupported(&mut self, yes: bool) -> &mut Self {
        self.config.fail_if_syntax_unsupported = yes;
        self
    }

    /// Whether or not to output 24bit colors (default: true)
    pub fn true_color(&mut self, yes: bool) -> &mut Self {
        self.config.true_color = yes;
        self
    }

    /// Use compact file headings without horizontal header/footer rules.
    /// Selected line numbers, change markers, and the vertical grid remain visible.
    pub fn compact_headers(&mut self, yes: bool) -> &mut Self {
        self.config.compact_headers = yes;
        self
    }

    /// Convert generated colors to grayscale (default: false).
    /// Input ANSI escape sequences and terminal-default colors are preserved.
    pub fn grayscale(&mut self, yes: bool) -> &mut Self {
        self.config.grayscale = yes;
        self
    }

    /// Whether to show a header with the file name
    pub fn header(&mut self, yes: bool) -> &mut Self {
        self.active_style_components.header_filename = yes;
        self
    }

    /// Whether to show the absolute source path in the header.
    pub fn header_path(&mut self, yes: bool) -> &mut Self {
        self.active_style_components.header_path = yes;
        self
    }

    /// Whether to show the last modification time in UTC.
    pub fn header_modified(&mut self, yes: bool) -> &mut Self {
        self.active_style_components.header_modified = yes;
        self
    }

    /// Whether to show file permissions (Unix mode or a read-only indicator).
    pub fn header_permissions(&mut self, yes: bool) -> &mut Self {
        self.active_style_components.header_permissions = yes;
        self
    }

    /// Whether to show line numbers
    pub fn line_numbers(&mut self, yes: bool) -> &mut Self {
        self.active_style_components.line_numbers = yes;
        self
    }

    /// Enable or disable line numbers and Git modification markers together.
    pub fn sidebar(&mut self, yes: bool) -> &mut Self {
        self.active_style_components.line_numbers = yes;
        #[cfg(feature = "git")]
        {
            self.active_style_components.vcs_modification_markers = yes;
        }
        self
    }

    /// Whether to mark highlighted lines with `>` in the sidebar.
    /// The column is omitted when no highlight ranges are configured.
    pub fn highlight_indicator(&mut self, yes: bool) -> &mut Self {
        self.active_style_components.highlight_indicator = yes;
        self
    }

    /// Whether to place the selected sidebar decorations on the right.
    pub fn sidebar_right(&mut self, yes: bool) -> &mut Self {
        self.active_style_components.sidebar_right = yes;
        self
    }

    /// Whether to paint a grid, separating line numbers, git changes and the code
    pub fn grid(&mut self, yes: bool) -> &mut Self {
        self.active_style_components.grid = yes;
        self
    }

    /// Whether to separate the sidebar from the contents without horizontal borders.
    pub fn grid_vertical(&mut self, yes: bool) -> &mut Self {
        self.active_style_components.grid_vertical = yes;
        self
    }

    /// Whether to paint a horizontal rule to delimit files
    pub fn rule(&mut self, yes: bool) -> &mut Self {
        self.active_style_components.rule = yes;
        self
    }

    /// Whether to show modification markers for VCS changes. This has no effect if
    /// the `git` feature is not activated.
    #[cfg(feature = "git")]
    pub fn vcs_modification_markers(&mut self, yes: bool) -> &mut Self {
        self.active_style_components.vcs_modification_markers = yes;
        self
    }

    /// Highlight lines with Git change markers using the theme's line highlight.
    /// This can be enabled independently of the sidebar modification markers.
    #[cfg(feature = "git")]
    pub fn vcs_modification_highlighting(&mut self, yes: bool) -> &mut Self {
        self.active_style_components.vcs_modification_highlighting = yes;
        self
    }

    /// Show commit attribution before line numbers (default: false).
    #[cfg(feature = "git")]
    pub fn git_blame(&mut self, yes: bool) -> &mut Self {
        self.active_style_components.git_blame = yes;
        self
    }

    /// Set the Git blame annotation format (default: "%h %an").
    #[cfg(feature = "git")]
    pub fn blame_format(&mut self, format: &'a str) -> &mut Self {
        self.config.blame_format = Some(format);
        self
    }

    /// Emphasize TODO and FIXME markers inside syntax comments (default: false).
    pub fn highlight_todos(&mut self, yes: bool) -> &mut Self {
        self.config.highlight_todos = yes;
        self
    }

    /// Underline existing literal file paths (default: false).
    pub fn show_paths(&mut self, yes: bool) -> &mut Self {
        self.config.show_paths = yes;
        self
    }

    /// Whether to print binary content or nonprintable characters (default: no)
    pub fn show_nonprintable(&mut self, yes: bool) -> &mut Self {
        self.config.show_nonprintable = yes;
        self
    }

    /// Report a missing final newline after the last visible line.
    pub fn warn_missing_newline(&mut self, yes: bool) -> &mut Self {
        self.config.warn_missing_newline = yes;
        self
    }

    /// Select the notation used when non-printable characters are shown.
    pub fn nonprintable_notation(&mut self, notation: crate::NonprintableNotation) -> &mut Self {
        self.config.nonprintable_notation = notation;
        self
    }

    /// Whether to show "snip" markers between visible line ranges (default: no)
    pub fn snip(&mut self, yes: bool) -> &mut Self {
        self.active_style_components.snip = yes;
        self
    }

    /// Whether to remove ANSI escape sequences from the input (default: never)
    ///
    /// If `Auto` is used, escape sequences will only be removed when the input
    /// is not plain text.
    pub fn strip_ansi(&mut self, mode: StripAnsiMode) -> &mut Self {
        self.config.strip_ansi = mode;
        self
    }

    /// Whether to sanitize untrusted input for safe display (default: never)
    ///
    /// Strips ANSI escape sequences and additionally substitutes terminal-active
    /// control bytes and bidi / zero-width codepoints with U+FFFD.
    pub fn sanitize(&mut self, mode: StripAnsiMode) -> &mut Self {
        self.config.sanitize = mode;
        self
    }

    /// Text wrapping mode (default: do not wrap)
    pub fn wrapping_mode(&mut self, mode: WrappingMode) -> &mut Self {
        self.config.wrapping_mode = mode;
        self
    }

    /// Whether or not to use ANSI italics (default: off)
    pub fn use_italics(&mut self, yes: bool) -> &mut Self {
        self.config.use_italic_text = yes;
        self
    }

    /// Configure OSC 8 hyperlinks, or disable them with `None`.
    pub fn hyperlinks(&mut self, config: Option<crate::hyperlink::Hyperlink>) -> &mut Self {
        self.config.hyperlink = config;
        self
    }

    /// Whether to honor theme background colors for highlighted text (default: off)
    pub fn use_theme_background(&mut self, yes: bool) -> &mut Self {
        self.config.use_theme_background = yes;
        self
    }

    /// If and how to use a pager (default: no paging)
    #[cfg(feature = "paging")]
    pub fn paging_mode(&mut self, mode: PagingMode) -> &mut Self {
        self.config.paging_mode = mode;
        self
    }

    /// Open the pager at a positive line number in a single input.
    /// Earlier output remains available. Supports standard less and pager wrappers.
    #[cfg(feature = "paging")]
    pub fn scroll_to(&mut self, line: usize) -> &mut Self {
        self.config.scroll_to = Some(line);
        self.config.scroll_to_center = false;
        self.config.center_highlight = false;
        self
    }

    /// Center the first visible highlighted line in the pager.
    #[cfg(feature = "paging")]
    pub fn center_highlight(&mut self, yes: bool) -> &mut Self {
        self.config.center_highlight = yes;
        if yes {
            self.config.scroll_to = None;
        }
        self
    }

    /// Reserve terminal rows from the automatic less pager viewport. Requires
    /// less 632 or newer; has no effect on forced or disabled paging.
    #[cfg(feature = "paging")]
    pub fn paging_reserve(&mut self, rows: u16) -> &mut Self {
        self.config.paging_reserve = rows;
        self
    }

    /// Specify the command to start the pager (default: use "less")
    #[cfg(feature = "paging")]
    pub fn pager(&mut self, cmd: &'a str) -> &mut Self {
        self.config.pager = Some(cmd);
        self
    }

    /// Append a literal argument to the selected external pager.
    /// Can be called repeatedly. The built-in pager does not accept arguments.
    #[cfg(feature = "paging")]
    pub fn pager_arg(&mut self, arg: impl Into<String>) -> &mut Self {
        self.config.pager_args.push(arg.into());
        self
    }

    /// Read at most this many bytes from each input, before line buffering.
    pub fn max_bytes(&mut self, limit: u64) -> &mut Self {
        self.config.max_bytes = Some(limit);
        self
    }

    /// Specify the lines that should be printed (default: all)
    pub fn line_ranges(&mut self, ranges: LineRanges) -> &mut Self {
        self.config.visible_lines = VisibleLines::Ranges(ranges);
        self
    }

    /// Show and highlight the enclosing brace-delimited definition for a source
    /// line. Repeat for more lines. This reads the complete text input first.
    pub fn function_context(&mut self, line: usize) -> &mut Self {
        self.config.function_context.push(line);
        self.highlighted_lines.push(LineRange::new(line, line));
        self
    }

    /// Fold complete syntax-defined blocks, comments, and consecutive imports.
    /// This reads the complete text input before rendering.
    pub fn fold(&mut self, yes: bool) -> &mut Self {
        self.config.fold = yes;
        self
    }

    /// Specify a line that should be highlighted (default: none).
    /// This can be called multiple times to highlight more than one
    /// line. See also: highlight_range.
    pub fn highlight(&mut self, line: usize) -> &mut Self {
        self.highlighted_lines.push(LineRange::new(line, line));
        self
    }

    /// Specify a range of lines that should be highlighted (default: none).
    /// This can be called multiple times to highlight more than one range
    /// of lines.
    pub fn highlight_range(&mut self, from: usize, to: usize) -> &mut Self {
        self.highlighted_lines.push(LineRange::new(from, to));
        self
    }

    /// Highlight lines matching a regular expression, in addition to explicit ranges.
    /// Repeat this method to match any of several patterns.
    pub fn highlight_pattern(&mut self, pattern: &str) -> Result<&mut Self> {
        self.config.highlighted_patterns.push(
            regex::Regex::new(pattern)
                .map_err(|error| format!("Invalid highlight pattern: {error}"))?,
        );
        Ok(self)
    }

    /// Highlight an inclusive range of line and character positions.
    pub fn highlight_region(
        &mut self,
        region: crate::highlight_region::HighlightRegion,
    ) -> &mut Self {
        self.config.highlighted_regions.push(region);
        self
    }

    /// Specify the maximum number of consecutive empty lines to print.
    pub fn squeeze_empty_lines(&mut self, maximum: Option<usize>) -> &mut Self {
        self.config.squeeze_lines = maximum;
        self
    }

    /// Specify the highlighting theme.
    /// You can use [`crate::theme::theme`] to pick a theme based on user preferences
    /// and the terminal's background color.
    pub fn theme(&mut self, theme: impl AsRef<str>) -> &mut Self {
        self.config.theme = theme.as_ref().to_owned();
        self
    }

    /// Override a global theme color without modifying the underlying theme.
    ///
    /// Supported names are `foreground`, `gutterForeground`, and `lineHighlight`.
    /// The value is six hexadecimal digits, optionally prefixed with `#`.
    pub fn set_theme_color(&mut self, name: &str, value: &str) -> Result<&mut Self> {
        self.config.theme_colors.set(name, value)?;
        Ok(self)
    }

    /// Specify custom file extension / file name to syntax mappings
    pub fn syntax_mapping(&mut self, mapping: SyntaxMapping<'a>) -> &mut Self {
        self.config.syntax_mapping = mapping;
        self
    }

    pub fn themes(&self) -> impl Iterator<Item = &str> {
        self.assets.themes()
    }

    pub fn syntaxes(&self) -> impl Iterator<Item = Syntax> + '_ {
        // Embedded assets are valid; custom assets were loaded and validated
        // by with_assets(), so get_syntaxes() cannot fail here.
        self.assets
            .get_syntaxes()
            .unwrap()
            .iter()
            .filter(|s| !s.hidden)
            .map(|s| Syntax {
                name: s.name.clone(),
                file_extensions: s.file_extensions.clone(),
            })
    }

    /// Pretty-print all specified inputs. This method will "use" all stored inputs.
    /// If you want to call 'print' multiple times, you have to call the appropriate
    /// input_* methods again.
    pub fn print(&mut self) -> Result<bool> {
        self.print_with_writer(None::<&mut dyn std::fmt::Write>)
    }

    /// Pretty-print all specified inputs to a specified writer.
    pub fn print_with_writer<W: std::fmt::Write>(&mut self, writer: Option<W>) -> Result<bool> {
        let highlight_lines = std::mem::take(&mut self.highlighted_lines);
        self.config.highlighted_lines = HighlightedLineRanges(LineRanges::from(highlight_lines));
        self.config.term_width = self
            .term_width
            .unwrap_or_else(|| Term::stdout().size().1 as usize);

        self.config.style_components.clear();
        if self.active_style_components.grid {
            self.config.style_components.insert(StyleComponent::Grid);
        }
        if self.active_style_components.grid_vertical {
            self.config
                .style_components
                .insert(StyleComponent::GridVertical);
        }
        if self.active_style_components.rule {
            self.config.style_components.insert(StyleComponent::Rule);
        }
        if self.active_style_components.header_filename {
            self.config
                .style_components
                .insert(StyleComponent::HeaderFilename);
        }
        if self.active_style_components.highlight_indicator {
            self.config
                .style_components
                .insert(StyleComponent::HighlightIndicator);
        }
        for (enabled, component) in [
            (
                self.active_style_components.header_path,
                StyleComponent::HeaderPath,
            ),
            (
                self.active_style_components.header_modified,
                StyleComponent::HeaderModified,
            ),
            (
                self.active_style_components.header_permissions,
                StyleComponent::HeaderPermissions,
            ),
        ] {
            if enabled {
                self.config.style_components.insert(component);
            }
        }
        if self.active_style_components.line_numbers {
            self.config
                .style_components
                .insert(StyleComponent::LineNumbers);
        }
        if self.active_style_components.sidebar_right {
            self.config
                .style_components
                .insert(StyleComponent::SidebarRight);
        }
        if self.active_style_components.snip {
            self.config.style_components.insert(StyleComponent::Snip);
        }
        #[cfg(feature = "git")]
        if self.active_style_components.vcs_modification_highlighting {
            self.config
                .style_components
                .insert(StyleComponent::ChangesHighlight);
        }
        #[cfg(feature = "git")]
        if self.active_style_components.vcs_modification_markers {
            self.config.style_components.insert(StyleComponent::Changes);
        }

        #[cfg(feature = "git")]
        if self.active_style_components.git_blame {
            self.config.style_components.insert(StyleComponent::Blame);
        }

        // Collect the inputs to print
        let inputs = std::mem::take(&mut self.inputs);

        // Run the controller
        let controller = Controller::new(&self.config, &self.assets);

        // If writer is provided, pass it to the controller, otherwise pass None
        if let Some(mut w) = writer {
            controller.run(
                inputs.into_iter().map(|i| i.into()).collect(),
                Some(&mut OutputHandle::FmtWrite(&mut w)),
            )
        } else {
            controller.run(inputs.into_iter().map(|i| i.into()).collect(), None)
        }
    }
}

impl Default for PrettyPrinter<'_> {
    fn default() -> Self {
        Self::new()
    }
}

/// An input source for the pretty printer.
pub struct Input<'a> {
    input: input::Input<'a>,
}

impl<'a> Input<'a> {
    /// A new input from a reader.
    pub fn from_reader<R: Read + 'a>(reader: R) -> Self {
        input::Input::from_reader(Box::new(reader)).into()
    }

    /// A new input from a file.
    pub fn from_file(path: impl AsRef<Path>) -> Self {
        input::Input::ordinary_file(path).into()
    }

    /// A new input from bytes.
    pub fn from_bytes(bytes: &'a [u8]) -> Self {
        Input::from_reader(bytes)
    }

    /// A new input from STDIN.
    pub fn from_stdin() -> Self {
        input::Input::stdin().into()
    }

    /// Read at most this many bytes from this input before line buffering.
    pub fn max_bytes(mut self, limit: u64) -> Self {
        self.input = self.input.with_max_bytes(limit);
        self
    }

    /// The filename of the input.
    /// This affects syntax detection and changes the default header title.
    pub fn name(mut self, name: impl AsRef<Path>) -> Self {
        self.input = self.input.with_name(Some(name));
        self
    }

    /// The description for the type of input (e.g. "File")
    pub fn kind(mut self, kind: impl Into<String>) -> Self {
        let kind = kind.into();
        self.input
            .description_mut()
            .set_kind(if kind.is_empty() { None } else { Some(kind) });
        self
    }

    /// The title for the input (e.g. "Descriptive title")
    /// This defaults to the file name.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.input.description_mut().set_title(Some(title.into()));
        self
    }
}

impl<'a> From<input::Input<'a>> for Input<'a> {
    fn from(input: input::Input<'a>) -> Self {
        Self { input }
    }
}

impl<'a> From<Input<'a>> for input::Input<'a> {
    fn from(Input { input }: Input<'a>) -> Self {
        input
    }
}
