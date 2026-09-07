mod utils;
use utils::command::bat;

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
