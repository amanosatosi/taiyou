#![cfg(feature = "native")]
use std::{path::Path, process::Command};

fn command(binary: &str, source: &Path, font: &Path, folder: &Path, name: &str) -> Command {
    let mut command = Command::new(binary);
    command
        .arg("render")
        .arg("--input")
        .arg(source)
        .arg("--font")
        .arg(font)
        .args([
            "--time-us",
            "500000",
            "--width",
            "320",
            "--height",
            "180",
            "--profile",
            "core-v1",
        ])
        .arg("--output")
        .arg(folder.join(format!("{name}.png")))
        .arg("--raw")
        .arg(folder.join(format!("{name}.rgba")));
    command
}

#[test]
fn typed_adapter_preserves_t014_pixels_and_error_identity() {
    let folder = std::env::temp_dir().join(format!("taiyou-t015-adapter-{}", std::process::id()));
    std::fs::create_dir(&folder).unwrap();
    let source =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/taiyou/core-slice/first.ass");
    let font = std::path::PathBuf::from(std::env::var_os("RUNNER_TEMP").unwrap())
        .join("taiyou-shaping-fonts/latin.otf");
    for (binary, name) in [
        (env!("CARGO_BIN_EXE_taiyou-cli"), "normal"),
        (env!("CARGO_BIN_EXE_taiyou-compare"), "adapter"),
    ] {
        let output = command(binary, &source, &font, &folder, name)
            .output()
            .unwrap();
        assert!(output.status.success(), "{:?}", output);
    }
    for extension in ["rgba", "png"] {
        assert_eq!(
            std::fs::read(folder.join(format!("normal.{extension}"))).unwrap(),
            std::fs::read(folder.join(format!("adapter.{extension}"))).unwrap()
        );
    }
    let unsupported = folder.join("unsupported.ass");
    std::fs::write(
        &unsupported,
        std::fs::read_to_string(&source)
            .unwrap()
            .replace("Taiyou", "{\\blur2}Taiyou"),
    )
    .unwrap();
    let rejected = command(
        env!("CARGO_BIN_EXE_taiyou-compare"),
        &unsupported,
        &font,
        &folder,
        "unsupported",
    )
    .output()
    .unwrap();
    assert_eq!(rejected.status.code(), Some(20));
    assert!(!folder.join("unsupported.png").exists());
    assert_eq!(
        Command::new(env!("CARGO_BIN_EXE_taiyou-compare"))
            .args(["render", "--unknown", "value"])
            .output()
            .unwrap()
            .status
            .code(),
        Some(10)
    );
    std::fs::remove_dir_all(folder).unwrap();
}
