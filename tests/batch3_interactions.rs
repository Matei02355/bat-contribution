mod utils;
use utils::command::bat;

#[test]
fn syntax_delimiters_reset_comment_annotations() {
    let output = |text: &str, extra: &[&str]| {
        bat()
            .args([
                "--paging=never",
                "--color=always",
                "--style=plain",
                "--language=rust",
                "--theme=TwoDark",
                "--highlight-todos",
            ])
            .args(extra)
            .write_stdin(text)
            .assert()
            .success()
            .get_output()
            .stdout
            .clone()
    };
    assert_eq!(
        output(
            "/* TODO unfinished\n---\nfn main() {} // FIXME after reset\n",
            &["--syntax-delimiter=^---$", "--line-range=3:"]
        ),
        output("fn main() {} // FIXME after reset\n", &[])
    );
}

#[test]
fn enclosing_context_preserves_per_syntax_styles_and_highlighting() {
    let output = |selection: &[&str]| {
        bat()
            .args([
                "--paging=never",
                "--color=always",
                "--decorations=always",
                "--language=C",
                "--theme=TwoDark",
                "--style=plain",
                "--style-for",
                "C",
                "numbers,highlight-indicator",
                "--highlight-todos",
            ])
            .args(selection)
            .write_stdin("int outside;\nint value() {\n return 1; // TODO improve\n}\nint after;\n")
            .assert()
            .success()
            .get_output()
            .stdout
            .clone()
    };
    assert_eq!(output(&["-W3"]), output(&["-r2:4", "-H3"]));
}

#[test]
fn byte_limits_combine_with_periodic_line_filters() {
    bat()
        .args(["--max-bytes=8", "--line-range=1:~2"])
        .write_stdin("1\n2\n3\n4\n5\n")
        .assert()
        .success()
        .stdout("1\n3\n");
}

#[test]
fn byte_limits_preserve_syntax_validation_before_streaming() {
    bat()
        .args([
            "--max-bytes=4",
            "--fail-if-syntax-unsupported",
            "--file-name=unknown.extension",
        ])
        .write_stdin("data\n")
        .assert()
        .failure()
        .stdout("")
        .stderr("");
    bat()
        .args([
            "--max-bytes=4",
            "--fail-if-syntax-unsupported",
            "--language=JSON",
        ])
        .write_stdin("[12345]\n")
        .assert()
        .success()
        .stdout("[123");
}

#[test]
fn regex_highlights_create_the_text_indicator_column() {
    let mut results = Vec::new();
    for selection in ["--highlight-pattern=^match$", "--highlight-line=2"] {
        results.push(
            bat()
                .args([
                    "--decorations=always",
                    "--color=never",
                    "--style=highlight-indicator,numbers",
                    selection,
                ])
                .write_stdin("first\nmatch\nlast\n")
                .assert()
                .success()
                .get_output()
                .stdout
                .clone(),
        );
    }
    assert_eq!(results[0], results[1]);
    assert!(results[0].contains(&b'>'));
}

#[test]
fn compact_headers_keep_links_and_selected_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("example.txt");
    std::fs::write(&file, "abcdef\n").unwrap();
    let out = bat()
        .args([
            "--decorations=compact",
            "--color=never",
            "--osc8",
            "--max-bytes=4",
            "--style=header,header-path,numbers,grid",
        ])
        .arg(&file)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    assert!(text.starts_with("\x1b]8;;file:"), "{text:?}");
    assert!(text.contains("===> "), "{text:?}");
    assert!(text.contains(" <===\x1b]8;;\x1b\\\n"), "{text:?}");
    assert!(
        text.contains(&format!("Path: {}\n", file.display())),
        "{text:?}"
    );
    assert!(text.contains("abcd\n"), "{text:?}");
    assert!(!text.contains('─'), "{text:?}");
}

#[test]
fn syntax_styles_preserve_theme_overrides_and_byte_limits() {
    let out = bat()
        .env("COLORTERM", "truecolor")
        .args([
            "--color=always",
            "--decorations=compact",
            "--paging=never",
            "--theme=Monokai Extended",
            "--language=JSON",
            "--style=plain",
            "--style-for",
            "JSON",
            "header,numbers",
            "--set-theme-color",
            "foreground",
            "123456",
            "--max-bytes=2",
        ])
        .write_stdin("{}extra\n")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    assert!(text.contains("===>"), "{text:?}");
    assert!(text.contains("38;2;18;52;86m"), "{text:?}");
    let plain = regex::Regex::new(r"\x1b\[[0-9;]*m")
        .unwrap()
        .replace_all(&text, "");
    assert!(plain.contains("{}"), "{text:?}");
    assert!(!text.contains("extra"), "{text:?}");
}

#[test]
fn syntax_styles_preserve_regex_highlight_indicators() {
    let output = |style_options: &[&str]| {
        bat()
            .args([
                "--color=never",
                "--decorations=always",
                "--paging=never",
                "--language=JSON",
                "--highlight-pattern=match",
            ])
            .args(style_options)
            .write_stdin("{\"match\": 1}\n{}\n")
            .assert()
            .success()
            .get_output()
            .stdout
            .clone()
    };
    assert_eq!(
        output(&["--style=highlight-indicator,numbers"]),
        output(&[
            "--style=plain",
            "--style-for",
            "JSON",
            "highlight-indicator,numbers"
        ])
    );
}
