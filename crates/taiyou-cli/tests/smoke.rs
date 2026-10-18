use std::process::{Command, Output};

fn cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_taiyou-cli"))
        .args(args)
        .output()
        .expect("CI must be able to launch the CLI")
}

fn assert_unimplemented(output: Output) {
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    #[cfg(not(feature = "native"))]
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "taiyou: rendering is not implemented (T001 foundation; no output produced)\n"
    );
    #[cfg(feature = "native")]
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("InvalidArgument")
    );
}

#[test]
fn version_succeeds_without_rendering() {
    let output = cli(&["--version"]);
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!("taiyou {}\n", env!("CARGO_PKG_VERSION"))
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn render_request_fails_and_creates_no_output() {
    let path = std::env::temp_dir().join(format!("taiyou-t001-{}.png", std::process::id()));
    assert!(!path.exists(), "smoke output path must be unused");
    let output = Command::new(env!("CARGO_BIN_EXE_taiyou-cli"))
        .args(["render", "missing.ass", "--output"])
        .arg(&path)
        .output()
        .unwrap();
    assert_unimplemented(output);
    assert!(!path.exists());
}

#[test]
fn empty_and_extra_arguments_cannot_report_success() {
    assert_unimplemented(cli(&[]));
    assert_unimplemented(cli(&["--version", "render", "missing.ass"]));
    assert_unimplemented(cli(&["--unknown"]));
}

#[cfg(unix)]
#[test]
fn non_utf8_arguments_fail_without_panicking() {
    use std::os::unix::ffi::OsStrExt;

    let output = Command::new(env!("CARGO_BIN_EXE_taiyou-cli"))
        .arg(std::ffi::OsStr::from_bytes(b"\xff"))
        .output()
        .unwrap();
    assert_unimplemented(output);
}
