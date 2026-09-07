mod utils;

#[cfg(unix)]
use std::time::Duration;
use utils::command::bat;

fn child_bat(args: &str) -> String {
    format!(
        "{} --no-config --paging=never {args}",
        shell_words::quote(env!("CARGO_BIN_EXE_bat"))
    )
}

#[test]
fn filters_run_per_file_and_retain_names_and_syntax() {
    let dir = tempfile::tempdir().unwrap();
    let first = dir.path().join("first file.json");
    let second = dir.path().join("second.json");
    std::fs::write(&first, "{}\n").unwrap();
    std::fs::write(&second, "[]\n").unwrap();
    let output = bat()
        .args([
            "--decorations=always",
            "--style=header",
            "--color=always",
            "--process",
        ])
        .arg(child_bat("--color=never --style=plain"))
        .args([&first, &second])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    assert!(text.contains("first file.json"), "{text:?}");
    assert!(text.contains("second.json"), "{text:?}");
    assert!(text.contains("\x1b["), "{text:?}");
    assert_eq!(std::fs::read_to_string(first).unwrap(), "{}\n");
    let numbered = bat()
        .args(["--color=never", "--style=plain", "--process"])
        .arg(child_bat(
            "--color=never --decorations=always --style=numbers",
        ))
        .args([&second, &second])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert_eq!(numbered, b"   1 []\n   1 []\n");
}

#[test]
fn stdin_and_explicit_names_are_filtered() {
    bat()
        .args(["--color=never", "--style=plain", "--process"])
        .arg(child_bat(
            "--color=never --decorations=always --style=numbers",
        ))
        .args(["-", "-"])
        .write_stdin("one\ntwo\n")
        .assert()
        .success()
        .stdout("   1 one\n   2 two\n");
    let output = bat()
        .args([
            "--color=never",
            "--style=header",
            "--decorations=always",
            "--file-name=logical.json",
            "--process",
        ])
        .arg(child_bat("--color=never --style=plain"))
        .write_stdin("{}\n")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert!(String::from_utf8(output).unwrap().contains("logical.json"));
}

#[test]
fn invalid_commands_missing_files_and_directories_fail_without_content() {
    for command in [
        "",
        "''",
        "'unterminated",
        "bat-no-such-process-command-2289",
    ] {
        bat()
            .args(["--process", command])
            .write_stdin("source\n")
            .assert()
            .failure()
            .stdout("");
    }
    let dir = tempfile::tempdir().unwrap();
    for input in [dir.path().to_path_buf(), dir.path().join("missing-file")] {
        bat()
            .arg("--process")
            .arg(child_bat("--style=plain"))
            .arg(input)
            .assert()
            .failure()
            .stdout("");
    }
}

#[test]
fn replacing_contents_preserves_library_input_identity() {
    let input = bat::input::Input::stdin().with_reader(Box::new(&b"replacement\n"[..]));
    assert!(input.is_stdin());
    assert_eq!(input.description().title(), "STDIN");
    let input =
        bat::input::Input::ordinary_file("nonexistent.json").with_reader(Box::new(&b"{}\n"[..]));
    assert!(!input.is_stdin());
    let mut output = String::new();
    let config = bat::config::Config {
        loop_through: true,
        ..Default::default()
    };
    let assets = bat::assets::HighlightingAssets::from_binary();
    assert!(bat::controller::Controller::new(&config, &assets)
        .run(
            vec![input],
            Some(&mut bat::output::OutputHandle::FmtWrite(&mut output))
        )
        .unwrap());
    assert_eq!(output, "{}\n");
}

#[cfg(unix)]
#[test]
fn process_failures_and_stderr_are_reported() {
    bat()
        .args(["--process", "sh -c 'printf warning >&2; exit 7'"])
        .write_stdin("source\n")
        .assert()
        .failure()
        .stdout("")
        .stderr(predicates::str::contains("warning"));
    bat()
        .args(["--process", "sh -c 'exit 0'"])
        .write_stdin("source\n")
        .assert()
        .success()
        .stdout("");
    // The command is split into arguments, not evaluated by a shell.
    bat()
        .args([
            "--style=plain",
            "--color=never",
            "--process",
            "printf '%s' '$HOME; | literal'",
        ])
        .write_stdin("")
        .assert()
        .success()
        .stdout("$HOME; | literal");
}

#[cfg(unix)]
#[test]
fn stopping_at_a_line_range_terminates_and_reaps_the_filter() {
    let dir = tempfile::tempdir().unwrap();
    let pid_path = dir.path().join("pid");
    let script = format!(
        "printf '%s' \"$$\" > {}; while :; do printf 'line\\n'; done",
        shell_words::quote(pid_path.to_str().unwrap())
    );
    let command = format!("sh -c {}", shell_words::quote(&script));
    bat()
        .timeout(Duration::from_secs(10))
        .args([
            "--process",
            &command,
            "--line-range=1:1",
            "--style=plain",
            "--color=never",
        ])
        .write_stdin("")
        .assert()
        .success()
        .stdout("line\n");
    let pid = std::fs::read_to_string(pid_path).unwrap();
    let result = std::process::Command::new("kill")
        .args(["-0", pid.trim()])
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(!result.success(), "filter process {pid} was left running");
}

#[cfg(unix)]
#[test]
fn filters_detect_input_output_cycles() {
    use std::fs::OpenOptions;
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("source");
    std::fs::write(&file, "source\n").unwrap();
    let output = utils::command::bat_raw_command()
        .args(["--process", "cat"])
        .arg(&file)
        .stdout(OpenOptions::new().append(true).open(&file).unwrap())
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("IO circle detected"));
    assert_eq!(std::fs::read_to_string(file).unwrap(), "source\n");
}

#[cfg(feature = "git")]
#[test]
fn process_and_diff_cannot_be_combined() {
    bat()
        .args(["--process", "cat", "--diff"])
        .write_stdin("source\n")
        .assert()
        .failure()
        .stdout("");
}

#[cfg(feature = "lessopen")]
#[test]
fn process_and_lessopen_cannot_be_combined() {
    bat()
        .args(["--process", "cat", "--lessopen"])
        .write_stdin("source\n")
        .assert()
        .failure()
        .stdout("");
}

#[test]
fn filtered_output_obeys_byte_limits_and_stdin_styles() {
    bat()
        .args(["--process"])
        .arg(child_bat("--color=never --style=plain"))
        .args(["--max-bytes=3", "--color=never", "--style=plain"])
        .write_stdin("abcdef\n")
        .assert()
        .success()
        .stdout("abc");
    let expected = bat()
        .args([
            "--color=never",
            "--decorations=always",
            "--style=plain",
            "--style-stdin=numbers",
        ])
        .write_stdin("abc\n")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    bat()
        .arg("--process")
        .arg(child_bat("--color=never --style=plain"))
        .args([
            "--color=never",
            "--decorations=always",
            "--style=plain",
            "--style-stdin=numbers",
        ])
        .write_stdin("abc\n")
        .assert()
        .success()
        .stdout(expected);
}
