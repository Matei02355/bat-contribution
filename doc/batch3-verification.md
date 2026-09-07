# Additional issue batch: implementation and verification

80 additional issues are addressed by 62 new pull requests and amendments to two earlier pull requests. Related issues share one coherent change. All 64 pull requests are open; none has been merged.

Repository: [sharkdp/bat](https://github.com/sharkdp/bat). Integration branch: [integration-code-batch3-20260907](https://github.com/Matei02355/bat-contribution/tree/integration-code-batch3-20260907). The integration branch combines code changes for interaction testing; independent syntax and theme asset submissions retain their own validation and are not all bundled into this branch.

## Validation

- The combined native all-feature suite passed **820 tests**, with 10 ignored tests. The final correction and interaction tests then passed a **64-test focused suite**, including four newly added regressions. Two Python notebook tests from the ignored set were explicitly run and passed.
- The final source passed all-target, all-feature Clippy with warnings denied, the minimal library build, formatting and whitespace checks. The regenerated default-feature help passed all 10 help tests.
- The WASI Preview 1 build compiled with official Rust 1.91.1 and passed **30 runtime checks on Node.js 22.23.2**, including input/output identity, Unicode, configuration paths, ten language grammars, enclosing definitions, metadata and hyperlinks.
- **18 real PTY checks passed**: 12 automatic paging and prompt reservation cases using less 668, plus six source-positioning cases using less 590/668, including scroll position combined with prompt reservation.
- The final folding correction passed seven structural unit tests and 13 structural integration tests in its standalone PR, followed by Clippy. Adjacent block comments preserve code on their shared line.
- Each individual PR includes its own validation details. Cross-platform CI results below are separate from local Linux validation.

## CI and merge status

At this snapshot: **56 PRs passed**, **6 have external infrastructure failures**, and **2 are still running**. Infrastructure errors were verified in workflow logs. The account cannot rerun upstream workflows or merge into sharkdp/bat; those actions require maintainers.

## Issue-to-PR map

| Additional issues | Pull request | Change | CI snapshot |
| --- | --- | --- | --- |
| [#1735](https://github.com/sharkdp/bat/issues/1735) | [#3922](https://github.com/sharkdp/bat/pull/3922) (amended) | Highlight decorated Git log headers and embedded patches | Passed |
| [#2249](https://github.com/sharkdp/bat/issues/2249) | [#3924](https://github.com/sharkdp/bat/pull/3924) (amended) | Support theme backgrounds and trailing whitespace inspection | Dependency host returned HTTP 504 during license checks |
| [#3454](https://github.com/sharkdp/bat/issues/3454) | [#3937](https://github.com/sharkdp/bat/pull/3937) | Add sidebar style alias for numbers and Git changes | Passed |
| [#1611](https://github.com/sharkdp/bat/issues/1611) | [#3938](https://github.com/sharkdp/bat/pull/3938) | Support periodic line ranges and highlights | Passed |
| [#2441](https://github.com/sharkdp/bat/issues/2441) | [#3939](https://github.com/sharkdp/bat/pull/3939) | Allow library printers to load custom highlighting assets | Passed |
| [#1324](https://github.com/sharkdp/bat/issues/1324) | [#3940](https://github.com/sharkdp/bat/pull/3940) | Add text indicators for highlighted lines | Passed |
| [#2837](https://github.com/sharkdp/bat/issues/2837) | [#3941](https://github.com/sharkdp/bat/pull/3941) | Allow appending literal arguments to the configured pager | Passed |
| [#1709](https://github.com/sharkdp/bat/issues/1709) | [#3942](https://github.com/sharkdp/bat/pull/3942) | Allow silent fallback for unsupported input syntaxes | Passed |
| [#1867](https://github.com/sharkdp/bat/issues/1867) | [#3943](https://github.com/sharkdp/bat/pull/3943) | Publish SHA-256 sidecars for release artifacts | Passed |
| [#1946](https://github.com/sharkdp/bat/issues/1946) | [#3944](https://github.com/sharkdp/bat/pull/3944) | Use structured issue forms for bug and feature reports | Passed |
| [#2775](https://github.com/sharkdp/bat/issues/2775) | [#3945](https://github.com/sharkdp/bat/pull/3945) | Support configurable aliases for language names | Passed |
| [#2529](https://github.com/sharkdp/bat/issues/2529), [#1523](https://github.com/sharkdp/bat/issues/1523) | [#3946](https://github.com/sharkdp/bat/pull/3946) | Highlight entire lines with Git modifications | Passed |
| [#2784](https://github.com/sharkdp/bat/issues/2784) | [#3947](https://github.com/sharkdp/bat/pull/3947) | Bound input reads before buffering lines | Passed |
| [#1752](https://github.com/sharkdp/bat/issues/1752) | [#3948](https://github.com/sharkdp/bat/pull/3948) | Color Markdown table delimiters in Monokai Extended | Passed |
| [#2648](https://github.com/sharkdp/bat/issues/2648) | [#3949](https://github.com/sharkdp/bat/pull/3949) | Stream plain output without waiting for newlines | Passed |
| [#1701](https://github.com/sharkdp/bat/issues/1701), [#2042](https://github.com/sharkdp/bat/issues/2042) | [#3950](https://github.com/sharkdp/bat/pull/3950) | Add optional file metadata to headers | Dependency host returned HTTP 504 during asset and license jobs |
| [#339](https://github.com/sharkdp/bat/issues/339), [#2764](https://github.com/sharkdp/bat/issues/2764), [#3287](https://github.com/sharkdp/bat/issues/3287) | [#3951](https://github.com/sharkdp/bat/pull/3951) | Allow overriding global theme colors from the command line | Passed |
| [#3303](https://github.com/sharkdp/bat/issues/3303) | [#3952](https://github.com/sharkdp/bat/pull/3952) | Add compact file headings with selected sidebars | Passed |
| [#2422](https://github.com/sharkdp/bat/issues/2422), [#2259](https://github.com/sharkdp/bat/issues/2259) | [#3953](https://github.com/sharkdp/bat/pull/3953) | Recognize OPF metadata and MDWN Markdown files | Passed |
| [#3854](https://github.com/sharkdp/bat/issues/3854), [#3000](https://github.com/sharkdp/bat/issues/3000) | [#3954](https://github.com/sharkdp/bat/pull/3954) | Honor XDG config and cache directory overrides on Windows | Passed |
| [#2507](https://github.com/sharkdp/bat/issues/2507) | [#3955](https://github.com/sharkdp/bat/pull/3955) | Show merged configuration arguments and individual fields | Passed |
| [#1492](https://github.com/sharkdp/bat/issues/1492) | [#3956](https://github.com/sharkdp/bat/pull/3956) | Add Pug syntax highlighting | Passed |
| [#1816](https://github.com/sharkdp/bat/issues/1816) | [#3957](https://github.com/sharkdp/bat/pull/3957) | Add Handlebars syntax highlighting | Passed |
| [#1354](https://github.com/sharkdp/bat/issues/1354) | [#3958](https://github.com/sharkdp/bat/pull/3958) | Expose XML DTD highlighting for external declarations | Passed |
| [#2652](https://github.com/sharkdp/bat/issues/2652) | [#3959](https://github.com/sharkdp/bat/pull/3959) | Detect redirected stdin filenames on Linux and Android | Passed |
| [#1708](https://github.com/sharkdp/bat/issues/1708) | [#3960](https://github.com/sharkdp/bat/pull/3960) | Support per-syntax decoration styles | Passed |
| [#3301](https://github.com/sharkdp/bat/issues/3301) | [#3961](https://github.com/sharkdp/bat/pull/3961) | Support GNOME system color scheme detection on Linux | Dependency host returned HTTP 504 during asset rebuilding |
| [#3147](https://github.com/sharkdp/bat/issues/3147) | [#3962](https://github.com/sharkdp/bat/pull/3962) | Support opt-in inherited project configuration files | Dependency host returned HTTP 504 during license checks |
| [#3427](https://github.com/sharkdp/bat/issues/3427) | [#3963](https://github.com/sharkdp/bat/pull/3963) | Add pictographic and compact nonprintable notations | Passed |
| [#672](https://github.com/sharkdp/bat/issues/672) | [#3964](https://github.com/sharkdp/bat/pull/3964) | Highlight inclusive line and character regions | Passed |
| [#1409](https://github.com/sharkdp/bat/issues/1409), [#2566](https://github.com/sharkdp/bat/issues/2566) | [#3965](https://github.com/sharkdp/bat/pull/3965) | Add Prolog highlighting and document asset licensing | GitHub DNS resolution failed during macOS checkout |
| [#2946](https://github.com/sharkdp/bat/issues/2946) | [#3966](https://github.com/sharkdp/bat/pull/3966) | Display mapped extensions consistently in language lists | Passed |
| [#2289](https://github.com/sharkdp/bat/issues/2289), [#2191](https://github.com/sharkdp/bat/issues/2191), [#3007](https://github.com/sharkdp/bat/issues/3007), [#1554](https://github.com/sharkdp/bat/issues/1554) | [#3967](https://github.com/sharkdp/bat/pull/3967) | Stream each input through a separate filter command | Passed |
| [#3265](https://github.com/sharkdp/bat/issues/3265) | [#3968](https://github.com/sharkdp/bat/pull/3968) | Backport C# record declarations and init accessors | Passed |
| [#2319](https://github.com/sharkdp/bat/issues/2319) | [#3969](https://github.com/sharkdp/bat/pull/3969) | Backport Java records and sealed type declarations | Passed |
| [#2664](https://github.com/sharkdp/bat/issues/2664) | [#3970](https://github.com/sharkdp/bat/pull/3970) | Run Windows test matrix directly in PowerShell | Passed |
| [#3203](https://github.com/sharkdp/bat/issues/3203) | [#3971](https://github.com/sharkdp/bat/pull/3971) | Support optional TOML configuration files | Passed |
| [#3173](https://github.com/sharkdp/bat/issues/3173) | [#3972](https://github.com/sharkdp/bat/pull/3972) | Use standard Debian architectures for musl packages | Passed |
| [#2417](https://github.com/sharkdp/bat/issues/2417) | [#3973](https://github.com/sharkdp/bat/pull/3973) | Publish stable download aliases for release assets | Passed |
| [#3025](https://github.com/sharkdp/bat/issues/3025) | [#3974](https://github.com/sharkdp/bat/pull/3974) | Keep unindented manpage prose out of heading scopes | Passed |
| [#2943](https://github.com/sharkdp/bat/issues/2943) | [#3975](https://github.com/sharkdp/bat/pull/3975) | Import custom VS Code JSON color themes | Dependency host returned HTTP 504 while downloading Zig syntax |
| [#1893](https://github.com/sharkdp/bat/issues/1893) | [#3976](https://github.com/sharkdp/bat/pull/3976) | Highlight xxd, canonical hexdump, and HexViewer output | Passed |
| [#3097](https://github.com/sharkdp/bat/issues/3097) | [#3977](https://github.com/sharkdp/bat/pull/3977) | Highlight Markdown alert and callout headings | Passed |
| [#1158](https://github.com/sharkdp/bat/issues/1158), [#1182](https://github.com/sharkdp/bat/issues/1182) | [#3978](https://github.com/sharkdp/bat/pull/3978) | Backport Bash alias and select loop highlighting fixes | Passed |
| [#2085](https://github.com/sharkdp/bat/issues/2085), [#2559](https://github.com/sharkdp/bat/issues/2559) | [#3979](https://github.com/sharkdp/bat/pull/3979) | Add automatic rebuilding for ordered custom asset sources | Passed |
| [#2616](https://github.com/sharkdp/bat/issues/2616) | [#3980](https://github.com/sharkdp/bat/pull/3980) | Support grayscale syntax and decoration colors | Passed |
| [#1955](https://github.com/sharkdp/bat/issues/1955) | [#3981](https://github.com/sharkdp/bat/pull/3981) | Highlight Python execution traces and embedded source lines | Passed |
| [#3047](https://github.com/sharkdp/bat/issues/3047) | [#3982](https://github.com/sharkdp/bat/pull/3982) | Replace the pinned CSS grammar with CSS3 without duplicate extensions | Passed |
| [#2701](https://github.com/sharkdp/bat/issues/2701) | [#3983](https://github.com/sharkdp/bat/pull/3983) | Add reproducible profile-guided builds and benchmarks | Passed |
| [#1855](https://github.com/sharkdp/bat/issues/1855) | [#3984](https://github.com/sharkdp/bat/pull/3984) | Keep the input filename in pager prompts | Passed |
| [#2245](https://github.com/sharkdp/bat/issues/2245) | [#3985](https://github.com/sharkdp/bat/pull/3985) | Support right-side sidebar decorations | Passed |
| [#2027](https://github.com/sharkdp/bat/issues/2027) | [#3986](https://github.com/sharkdp/bat/pull/3986) | Use native command lookup for pagers | Passed |
| [#1054](https://github.com/sharkdp/bat/issues/1054) | [#3987](https://github.com/sharkdp/bat/pull/3987) | Add CUDA C++ syntax support | Passed |
| [#2671](https://github.com/sharkdp/bat/issues/2671) | [#3988](https://github.com/sharkdp/bat/pull/3988) | Refresh GitHub theme syntax colors and gutter | Passed |
| [#3164](https://github.com/sharkdp/bat/issues/3164) | [#3989](https://github.com/sharkdp/bat/pull/3989) | Replace deprecated serde_yaml with serde_yaml_ng | Passed |
| [#1185](https://github.com/sharkdp/bat/issues/1185), [#2363](https://github.com/sharkdp/bat/issues/2363), [#2576](https://github.com/sharkdp/bat/issues/2576) | [#3990](https://github.com/sharkdp/bat/pull/3990) | Open the pager at source lines and highlighted positions | Passed |
| [#1536](https://github.com/sharkdp/bat/issues/1536), [#2810](https://github.com/sharkdp/bat/issues/2810) | [#3991](https://github.com/sharkdp/bat/pull/3991) | Add optional Git blame annotations for working-tree files | Passed |
| [#1743](https://github.com/sharkdp/bat/issues/1743) | [#3992](https://github.com/sharkdp/bat/pull/3992) | Explain how styles and decorations interact in pipelines | Passed |
| [#1147](https://github.com/sharkdp/bat/issues/1147) | [#3993](https://github.com/sharkdp/bat/pull/3993) | Write each rendered source line as one output operation | Passed |
| [#2225](https://github.com/sharkdp/bat/issues/2225) | [#3994](https://github.com/sharkdp/bat/pull/3994) | Add opt-in highlighting for TODO and FIXME comments | Passed |
| [#2907](https://github.com/sharkdp/bat/issues/2907) | [#3996](https://github.com/sharkdp/bat/pull/3996) | Underline existing literal file paths without replacing syntax colors | Passed |
| [#2661](https://github.com/sharkdp/bat/issues/2661) | [#3997](https://github.com/sharkdp/bat/pull/3997) | Support a portable WASI Preview 1 command-line build | Passed |
| [#2228](https://github.com/sharkdp/bat/issues/2228), [#3005](https://github.com/sharkdp/bat/issues/3005) | [#3998](https://github.com/sharkdp/bat/pull/3998) | Add syntax-aware enclosing definitions and folded output | Checks still running |
| [#1620](https://github.com/sharkdp/bat/issues/1620) | [#3999](https://github.com/sharkdp/bat/pull/3999) | Reserve prompt rows during automatic less paging | Checks still running |

## Scope limits recorded in the PRs

- Enclosing-definition selection indexes complete brace-delimited definitions using syntax scopes. Indentation-only functions are unsupported, and incomplete or unrecognized definitions fall back to the requested line.
- WASI support is a Preview 1 command-line build with runtime filesystem permissions. It does not add browser DOM integration, an iOS port, Git annotations or external processes/pagers.
- Prompt reservation requires less 632 or newer, changes its available viewport, and uses a nonnegative row count. Negative offsets and an independent paging threshold are outside that option.
- Linux PTY and performance results do not establish equivalent performance on Windows or macOS.

