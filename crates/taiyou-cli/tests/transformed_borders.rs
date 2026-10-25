#![cfg(feature = "native")]
use std::{path::PathBuf, process::Command};

fn support(raw: &[u8], axis: usize) -> usize {
    let mut lo = usize::MAX;
    let mut hi = 0;
    for (i, pixel) in raw.chunks_exact(4).enumerate() {
        if pixel[3] != 0 {
            let p = if axis == 0 { i % 320 } else { i / 320 };
            lo = lo.min(p);
            hi = hi.max(p + 1);
        }
    }
    if lo == usize::MAX { 0 } else { hi - lo }
}

#[test]
fn transformed_text_and_drawing_borders_follow_the_surface() {
    let root = std::env::var_os("TAIYOU_TRANSFORMED_BORDER_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::temp_dir().join(format!("taiyou-borders-{}", std::process::id()))
        });
    std::fs::create_dir_all(&root).unwrap();
    let font = PathBuf::from(std::env::var_os("TAIYOU_TEST_FONT").unwrap());
    for (kind, content) in [
        ("drawing", r"{\p1}m -40 -12 l 40 -12 40 12 -40 12"),
        ("text", "ABC"),
    ] {
        let mut x_heights = Vec::new();
        let mut y_widths = Vec::new();
        let mut ordinary = Vec::new();
        for (id, tags) in [
            ("ordinary", ""),
            ("zero", r"\frx0\fry0"),
            ("x45", r"\frx45"),
            ("x80", r"\frx80"),
            ("x89", r"\frx89"),
            ("x90", r"\frx90"),
            ("y45", r"\fry45"),
            ("y80", r"\fry80"),
            ("y89", r"\fry89"),
            ("y90", r"\fry90"),
            ("combined", r"\frx45\fry30\frz37"),
            ("asymmetric", r"\frx45\fry20\xbord12\ybord3"),
            ("x-only", r"\frx45\xbord6\ybord0"),
            ("y-only", r"\fry45\xbord0\ybord6"),
            ("effects", r"\frx80\shad4\blur2"),
        ] {
            let name = format!("{kind}-{id}");
            let source = root.join(format!("{name}.ass"));
            let png = root.join(format!("{name}.png"));
            let raw = root.join(format!("{name}.rgba"));
            std::fs::write(&source, format!(
                "[Script Info]\nPlayResX: 320\nPlayResY: 240\nScaledBorderAndShadow: yes\nWrapStyle: 2\n[V4+ Styles]\nFormat: Name,Fontname,Fontsize,PrimaryColour,SecondaryColour,OutlineColour,BackColour,Outline,Shadow,Alignment\nStyle: Default,Source Sans 3,52,&H00FFFFFF,&H00FFFFFF,&H000000FF,&H00000000,6,0,5\n[Events]\nFormat: Layer,Start,End,Style,Text\nDialogue: 0,0:00:00.00,0:00:01.00,Default,{{\\pos(160,120)\\org(160,120){tags}}}{content}\n"
            )).unwrap();
            let output = Command::new(env!("CARGO_BIN_EXE_taiyou"))
                .args([
                    "render",
                    "--font-source",
                    "explicit",
                    "--time-us",
                    "500000",
                    "--width",
                    "320",
                    "--height",
                    "240",
                    "--input",
                ])
                .arg(&source)
                .arg("--font")
                .arg(&font)
                .arg("--output")
                .arg(&png)
                .arg("--raw")
                .arg(&raw)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{name}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let pixels = std::fs::read(&raw).unwrap();
            assert_eq!(pixels.len(), 320 * 240 * 4);
            println!(
                "TRANSFORMED_BORDER {name} width={} height={}",
                support(&pixels, 0),
                support(&pixels, 1)
            );
            match id {
                "ordinary" => ordinary = pixels.clone(),
                "zero" => {
                    assert_eq!(ordinary, pixels);
                    x_heights.push(support(&pixels, 1));
                    y_widths.push(support(&pixels, 0));
                }
                "x45" | "x80" | "x89" | "x90" => x_heights.push(support(&pixels, 1)),
                "y45" | "y80" | "y89" | "y90" => y_widths.push(support(&pixels, 0)),
                _ => assert!(pixels.chunks_exact(4).any(|p| p[3] != 0)),
            }
        }
        for widths in [x_heights, y_widths] {
            assert!(widths.windows(2).all(|w| w[1] < w[0]), "{kind}: {widths:?}");
            assert_eq!(widths[4], 0, "edge-on surfaces have zero filled area");
        }
    }
}
