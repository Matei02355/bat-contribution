mod utils;

use utils::command::bat;

fn output(args: &[&str], input: &str) -> Vec<u8> {
    bat()
        .args(["--paging=never", "--decorations=always", "--color=never"])
        .args(args)
        .write_stdin(input)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone()
}

#[test]
fn regex_highlights_control_highlighted_hyperlinks() {
    let common = [
        "--osc8-highlight",
        "--file-name=example.txt",
        "--style=numbers",
    ];
    let mut pattern = common.to_vec();
    pattern.push("--highlight-pattern=^match$");
    let mut numeric = common.to_vec();
    numeric.push("--highlight-line=2");
    let text = output(&pattern, "first\nmatch\nlast\n");
    assert_eq!(text, output(&numeric, "first\nmatch\nlast\n"));
    assert_eq!(
        String::from_utf8(text)
            .unwrap()
            .matches("\x1b]8;;file:")
            .count(),
        1
    );
}

#[test]
fn contextual_styles_select_vertical_grid() {
    let text = output(
        &["--style=full", "--style-stdin=numbers,grid-vertical"],
        "one\ntwo\n",
    );
    assert_eq!(
        text,
        output(&["--style=numbers,grid-vertical"], "one\ntwo\n")
    );
    assert_eq!(String::from_utf8(text).unwrap(), "   1 │ one\n   2 │ two\n");
}

#[test]
fn modeline_selected_syntax_resets_at_delimiters() {
    let input = "// -*- rust -*-\n/* unterminated\n---\nfn main() {}\n";
    let common = [
        "--style=plain",
        "--color=always",
        "--theme=TwoDark",
        "--syntax-delimiter=^---$",
        "--line-range=4:",
    ];
    assert_eq!(
        output(&common, input),
        output(
            &[
                "--style=plain",
                "--color=always",
                "--theme=TwoDark",
                "--language=rust"
            ],
            "fn main() {}\n"
        )
    );
}

#[test]
fn skipped_binaries_do_not_generate_newline_warnings_or_header_padding() {
    let dir = tempfile::tempdir().unwrap();
    let binary = dir.path().join("binary");
    let text = dir.path().join("text");
    std::fs::write(&binary, b"\0binary").unwrap();
    std::fs::write(&text, "text").unwrap();
    let args = [
        "--binary=skip",
        "--warning=missing-trailing-newline",
        "--paging=never",
        "--decorations=always",
        "--color=never",
        "--style=header",
    ];
    let actual = bat()
        .args(args)
        .arg(&binary)
        .arg(&text)
        .assert()
        .success()
        .stderr("")
        .get_output()
        .stdout
        .clone();
    let expected = bat()
        .args(args)
        .arg(&text)
        .assert()
        .success()
        .stderr("")
        .get_output()
        .stdout
        .clone();
    assert_eq!(actual, expected);
    assert_eq!(
        String::from_utf8(actual)
            .unwrap()
            .matches("No newline at end of file")
            .count(),
        1
    );
}
