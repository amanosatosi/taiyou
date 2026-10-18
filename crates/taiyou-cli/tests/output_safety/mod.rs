//! Real CLI regressions: no input or existing output may be truncated on error.
use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);
const OLD_PNG: &[u8] = b"original PNG destination";
const OLD_RAW: &[u8] = b"original raw destination";

struct Fixture {
    directory: PathBuf,
    ass: PathBuf,
    font: PathBuf,
    ass_bytes: Vec<u8>,
    font_bytes: Vec<u8>,
}
impl Fixture {
    fn new() -> Self {
        let directory = std::env::temp_dir().join(format!(
            "taiyou-output-safety-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&directory).unwrap();
        let ass = directory.join("input.ass");
        let font = directory.join("font.otf");
        let ass_bytes = super::source(&super::dialogue("H")).into_bytes();
        let font_bytes = fs::read(
            PathBuf::from(std::env::var_os("RUNNER_TEMP").unwrap())
                .join("taiyou-shaping-fonts/latin.otf"),
        )
        .unwrap();
        fs::write(&ass, &ass_bytes).unwrap();
        fs::write(&font, &font_bytes).unwrap();
        Self {
            directory,
            ass,
            font,
            ass_bytes,
            font_bytes,
        }
    }
    fn run(&self, png: &Path, raw: Option<&Path>) -> Output {
        let mut args: Vec<OsString> = vec![
            "render".into(),
            "--input".into(),
            self.ass.clone().into_os_string(),
            "--font".into(),
            self.font.clone().into_os_string(),
            "--time-us".into(),
            "500000".into(),
            "--width".into(),
            "160".into(),
            "--height".into(),
            "100".into(),
            "--profile".into(),
            "core-v1".into(),
            "--output".into(),
            png.as_os_str().into(),
        ];
        if let Some(raw) = raw {
            args.extend(["--raw".into(), raw.as_os_str().into()]);
        }
        Command::new(env!("CARGO_BIN_EXE_taiyou"))
            .args(args)
            .output()
            .unwrap()
    }
    fn failed(&self, png: &Path, raw: Option<&Path>) {
        let result = self.run(png, raw);
        assert_eq!(
            result.status.code(),
            Some(2),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(String::from_utf8_lossy(&result.stderr).contains("InvalidArgument"));
        self.inputs_intact();
        assert!(
            !fs::read_dir(&self.directory).unwrap().any(|entry| {
                entry
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".taiyou-")
            }),
            "staging file leaked"
        );
    }
    fn inputs_intact(&self) {
        assert_eq!(fs::read(&self.ass).unwrap(), self.ass_bytes);
        assert_eq!(fs::read(&self.font).unwrap(), self.font_bytes);
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.directory).unwrap();
    }
}

#[test]
fn hard_links_to_ass_and_font_are_rejected_for_both_outputs() {
    let f = Fixture::new();
    for input in [&f.ass, &f.font] {
        let alias = f.directory.join("alias");
        fs::hard_link(input, &alias).unwrap();
        let other = f.directory.join("other");
        fs::write(&other, OLD_RAW).unwrap();
        f.failed(&alias, Some(&other));
        assert_eq!(fs::read(&alias).unwrap(), fs::read(input).unwrap());
        assert_eq!(fs::read(&other).unwrap(), OLD_RAW);
        f.failed(&other, Some(&alias));
        assert_eq!(fs::read(&other).unwrap(), OLD_RAW);
        fs::remove_file(alias).unwrap();
    }
}

#[test]
fn hard_linked_outputs_and_identical_resolved_names_preserve_old_bytes() {
    let f = Fixture::new();
    let png = f.directory.join("frame.png");
    let raw = f.directory.join("frame.rgba");
    fs::write(&png, OLD_PNG).unwrap();
    fs::hard_link(&png, &raw).unwrap();
    f.failed(&png, Some(&raw));
    assert_eq!(fs::read(&png).unwrap(), OLD_PNG);
    assert_eq!(fs::read(&raw).unwrap(), OLD_PNG);
    f.failed(&png, Some(&f.directory.join("./frame.png")));
    assert_eq!(fs::read(&png).unwrap(), OLD_PNG);
}

#[test]
fn failed_generation_or_output_preparation_preserves_existing_files() {
    let mut f = Fixture::new();
    let png = f.directory.join("frame.png");
    let raw = f.directory.join("frame.rgba");
    fs::write(&png, OLD_PNG).unwrap();
    fs::write(&raw, OLD_RAW).unwrap();
    f.failed(&png, Some(&f.directory.join("missing/child.rgba")));
    assert_eq!(fs::read(&png).unwrap(), OLD_PNG);
    let directory = f.directory.join("not-a-file");
    fs::create_dir(&directory).unwrap();
    f.failed(&png, Some(&directory));
    assert_eq!(fs::read(&png).unwrap(), OLD_PNG);
    f.failed(&directory, Some(&raw));
    assert_eq!(fs::read(&raw).unwrap(), OLD_RAW);
    // A genuine render failure occurs before either destination is published.
    f.ass_bytes = super::source(&super::dialogue(r"{\blur2}H")).into_bytes();
    fs::write(&f.ass, &f.ass_bytes).unwrap();
    let result = f.run(&png, Some(&raw));
    assert_eq!(result.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&result.stderr).contains("UnsupportedFeature"));
    f.inputs_intact();
    assert_eq!(fs::read(&png).unwrap(), OLD_PNG);
    assert_eq!(fs::read(&raw).unwrap(), OLD_RAW);
}

#[test]
fn successful_publication_replaces_both_outputs_and_repeats_identically() {
    let f = Fixture::new();
    let png = f.directory.join("frame.png");
    let raw = f.directory.join("frame.rgba");
    fs::write(&png, OLD_PNG).unwrap();
    fs::write(&raw, OLD_RAW).unwrap();
    let first = f.run(&png, Some(&raw));
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let expected_png = fs::read(&png).unwrap();
    let expected_raw = fs::read(&raw).unwrap();
    assert!(expected_png.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert_eq!(expected_raw.len(), 160 * 100 * 4);
    assert!(f.run(&png, Some(&raw)).status.success());
    assert_eq!(fs::read(&png).unwrap(), expected_png);
    assert_eq!(fs::read(&raw).unwrap(), expected_raw);
    f.inputs_intact();
}

#[test]
fn symlinks_and_symlinked_parent_destinations_are_rejected() {
    let f = Fixture::new();
    let alias = f.directory.join("alias");
    let png = f.directory.join("frame.png");
    fs::write(&png, OLD_PNG).unwrap();
    for input in [&f.ass, &f.font, &png] {
        symlink_file(input, &alias);
        f.failed(&alias, None);
        f.failed(&png, Some(&alias));
        assert_eq!(fs::read(&png).unwrap(), OLD_PNG);
        fs::remove_file(&alias).unwrap();
    }
    symlink_file(&f.directory.join("absent"), &alias);
    f.failed(&alias, None); // A dangling symlink is not a new destination.
    fs::remove_file(&alias).unwrap();
    let parent = f.directory.join("parent-link");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&f.directory, &parent).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(&f.directory, &parent).unwrap();
    f.failed(&parent.join("input.ass"), None);
    let new = f.directory.join("new.png");
    f.failed(&new, Some(&parent.join("new.png")));
    assert!(!new.exists());
}

fn symlink_file(target: &Path, link: &Path) {
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, link).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(target, link).unwrap();
}

#[cfg(windows)]
#[test]
fn windows_case_stream_device_trimming_and_existing_aliases_are_rejected() {
    let f = Fixture::new();
    f.failed(&f.directory.join("INPUT.ASS"), None);
    f.failed(&f.directory.join("FONT.OTF"), None);
    for name in [
        "input.ass:stream",
        "font.otf:stream",
        "input.ass.",
        "font.otf ",
        "NUL",
        "COM1.png",
        "LPT².raw",
    ] {
        f.failed(&f.directory.join(name), None);
    }
    let png = f.directory.join("frame.png");
    fs::write(&png, OLD_PNG).unwrap();
    f.failed(&png, Some(&f.directory.join("FRAME.PNG")));
    assert_eq!(fs::read(&png).unwrap(), OLD_PNG);
    fs::remove_file(&png).unwrap();
    // Check names that did not exist at preflight, including Unicode case.
    for (lower, upper) in [("frame.png", "FRAME.PNG"), ("é.png", "É.PNG")] {
        let lower = f.directory.join(lower);
        f.failed(&lower, Some(&f.directory.join(upper)));
        assert!(!lower.exists());
    }
    // Extended-length aliases bypass Win32 normalization but not sharing guards.
    f.failed(&f.ass.canonicalize().unwrap(), None);
    f.failed(&f.font.canonicalize().unwrap(), None);
}

#[cfg(windows)]
#[test]
fn windows_locked_output_fails_without_replacing_either_existing_output() {
    use std::os::windows::fs::OpenOptionsExt;
    let f = Fixture::new();
    let png = f.directory.join("frame.png");
    let raw = f.directory.join("frame.rgba");
    fs::write(&png, OLD_PNG).unwrap();
    fs::write(&raw, OLD_RAW).unwrap();
    let lock = fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(&raw)
        .unwrap();
    f.failed(&png, Some(&raw));
    assert_eq!(fs::read(&png).unwrap(), OLD_PNG);
    assert_eq!(fs::read(&raw).unwrap(), OLD_RAW);
    drop(lock);
}
