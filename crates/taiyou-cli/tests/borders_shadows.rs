#![cfg(feature = "native")]

use std::{path::PathBuf, process::Command};

fn ass(outline: u8, shadow: u8, text: &str) -> String {
    format!(
        "[Script Info]\nPlayResX: 160\nPlayResY: 100\nWrapStyle: 2\n[V4+ Styles]\nFormat: Name,Fontname,Fontsize,PrimaryColour,OutlineColour,BackColour,Outline,Shadow,Alignment,MarginL,MarginR,MarginV\nStyle: Default,Source Sans 3,20,&H00FFFFFF,&H000000FF,&HFFFF0000,{outline},{shadow},7,10,10,10\n[Events]\nFormat: Layer,Start,End,Style,Text\nDialogue: 0,0:00:00.10,0:00:01.00,Default,{text}"
    )
}

#[test]
fn cli_renders_style_and_override_borders_and_shadows_to_png_and_raw_rgba() {
    let directory = std::env::temp_dir().join(format!("taiyou-t016-cli-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let font = PathBuf::from(std::env::var("RUNNER_TEMP").unwrap())
        .join("taiyou-shaping-fonts")
        .join("latin.otf");
    let plain_ass = directory.join("plain.ass");
    let effect_ass = directory.join("effects.ass");
    let disabled_ass = directory.join("disabled.ass");
    let shadow_ass = directory.join("shadow-only.ass");
    let plain_png = directory.join("plain.png");
    let effect_png = directory.join("effects.png");
    let disabled_png = directory.join("disabled.png");
    let shadow_png = directory.join("shadow-only.png");
    let plain_raw = directory.join("plain.rgba");
    let effect_raw = directory.join("effects.rgba");
    let disabled_raw = directory.join("disabled.rgba");
    let shadow_raw = directory.join("shadow-only.rgba");
    std::fs::write(&plain_ass, ass(0, 0, "H")).unwrap();
    std::fs::write(
        &effect_ass,
        ass(
            2,
            2,
            r"{\bord2.5\shad-1.5\3c&H0000FF&\4c&HFF0000&\3a&H20&\4a&H40&\alpha&H10&\1a&HFF&}H",
        ),
    )
    .unwrap();
    std::fs::write(&disabled_ass, ass(2, 2, r"{\bord0\shad0\4a&HFF&}H")).unwrap();
    std::fs::write(&shadow_ass, ass(0, 2, r"{\bord0\4a&H00&}H")).unwrap();

    for (input, png, raw) in [
        (&plain_ass, &plain_png, &plain_raw),
        (&effect_ass, &effect_png, &effect_raw),
        (&disabled_ass, &disabled_png, &disabled_raw),
        (&shadow_ass, &shadow_png, &shadow_raw),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_taiyou"))
            .args([
                "render",
                "--input",
                input.to_str().unwrap(),
                "--font",
                font.to_str().unwrap(),
                "--time-us",
                "500000",
                "--width",
                "160",
                "--height",
                "100",
                "--profile",
                "core-v1",
                "--output",
                png.to_str().unwrap(),
                "--raw",
                raw.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "CLI failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            std::fs::read(png)
                .unwrap()
                .starts_with(b"\x89PNG\r\n\x1a\n")
        );
    }

    let plain = std::fs::read(plain_raw).unwrap();
    let effects = std::fs::read(effect_raw).unwrap();
    let disabled = std::fs::read(disabled_raw).unwrap();
    let shadow_only = std::fs::read(shadow_raw).unwrap();
    assert_eq!(plain.len(), 160 * 100 * 4);
    assert_eq!(effects.len(), plain.len());
    assert_eq!(
        disabled, plain,
        "bord0 and transparent shad0 preserve fill-only output"
    );
    assert_ne!(effects, plain);
    assert_ne!(shadow_only, plain, "shadow renders without an outline");
    assert!(effects.chunks_exact(4).any(|p| p[3] > 0));
    assert!(effects.chunks_exact(4).any(|p| p[0] > 0));
    assert!(effects.chunks_exact(4).any(|p| p[2] > 0));
}

#[test]
fn cli_keeps_anisotropic_overrides_unsupported_in_strict_and_tolerant_modes() {
    let directory = std::env::temp_dir().join(format!("taiyou-t016-policy-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let input = directory.join("unsupported.ass");
    let strict_png = directory.join("strict.png");
    let tolerant_png = directory.join("tolerant.png");
    let font = PathBuf::from(std::env::var("RUNNER_TEMP").unwrap())
        .join("taiyou-shaping-fonts")
        .join("latin.otf");
    std::fs::write(&input, ass(0, 0, r"{\xbord2\yshad1}H")).unwrap();
    let base = [
        "render",
        "--input",
        input.to_str().unwrap(),
        "--font",
        font.to_str().unwrap(),
        "--time-us",
        "500000",
        "--width",
        "160",
        "--height",
        "100",
        "--profile",
        "core-v1",
    ];
    let strict = Command::new(env!("CARGO_BIN_EXE_taiyou"))
        .args(base)
        .args(["--output", strict_png.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(strict.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&strict.stderr).contains("UnsupportedFeature"));
    assert!(!strict_png.exists());

    let tolerant = Command::new(env!("CARGO_BIN_EXE_taiyou"))
        .args(base)
        .args([
            "--unsupported",
            "tolerant",
            "--output",
            tolerant_png.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        tolerant.status.success(),
        "tolerant CLI failed: {}",
        String::from_utf8_lossy(&tolerant.stderr)
    );
    assert!(String::from_utf8_lossy(&tolerant.stderr).contains("UnsupportedFeature"));
    assert!(
        std::fs::read(tolerant_png)
            .unwrap()
            .starts_with(b"\x89PNG\r\n\x1a\n")
    );
}
