#![cfg(feature = "native")]
use std::{path::PathBuf, sync::Arc};
use taiyou_render::{
    collision_footprint::collision_footprint, geometry::DeviceMapping, layout, renderer::*,
    shaping::shape_event,
};

fn catalog() -> Arc<ValidatedFontCatalog> {
    let mut ctx = FontContext::with_limits(Limits::default()).unwrap();
    let mut builder = FontCatalogBuilder::new(Limits::default());
    let path =
        PathBuf::from(std::env::var("RUNNER_TEMP").unwrap()).join("taiyou-shaping-fonts/latin.otf");
    builder
        .register(&mut ctx, &std::fs::read(path).unwrap(), 0)
        .unwrap();
    builder.snapshot(&mut CatalogIngestor::new()).unwrap()
}
fn source(text: &str) -> String {
    format!(
        "[Script Info]\nPlayResX: 160\nPlayResY: 100\nWrapStyle: 2\nScaledBorderAndShadow: no\n[V4+ Styles]\nFormat: Name,Fontname,Fontsize,PrimaryColour,BackColour,OutlineColour,Outline,Shadow,Alignment,MarginL,MarginR,MarginV,Angle\nStyle: Default,Source Sans 3,20,&H00FFFFFF,&HFF000000,&H000000FF,0,0,7,13,21,9,0\n[Events]\nFormat: Layer,Start,End,Style,Text\n{text}"
    )
}
fn row(text: &str) -> String {
    format!("Dialogue: 0,0:00:00.00,0:00:02.00,Default,{text}\n")
}
fn track(input: &str) -> Arc<Track> {
    taiyou_ass::parse_ass(
        input.as_bytes(),
        &ParseOptions::new(
            InputEncoding::Utf8,
            Profile::CoreV1,
            Limits::default(),
            SyntaxPolicy::Strict,
        ),
        &mut TrackIngestor::new(),
    )
    .unwrap()
    .track()
    .clone()
}
fn render(input: &str, fonts: Arc<ValidatedFontCatalog>, size: Size, limits: Limits) -> Frame {
    Renderer::new(RendererConfig {
        limits,
        ..Default::default()
    })
    .unwrap()
    .render(
        track(input),
        fonts,
        &RenderRequest {
            now: TimeUs::from_micros(750_000),
            viewport: Viewport::full(size).unwrap(),
            unsupported: UnsupportedPolicy::Strict,
        },
    )
    .unwrap()
}
fn footprint(
    input: &str,
    fonts: &ValidatedFontCatalog,
    size: Size,
) -> Result<(ScriptBounds, ScriptBounds), RenderError> {
    let track = track(input);
    let evaluated = taiyou_render::evaluate(
        &track,
        EventId::new(0),
        TimeUs::from_micros(750_000),
        Profile::CoreV1,
    )?;
    let mut ctx = FontContext::with_limits(Limits::default()).unwrap();
    let shaped = shape_event(
        &mut ctx,
        fonts,
        &evaluated,
        &Default::default(),
        &Limits::default(),
    )
    .unwrap();
    let local = layout(
        &shaped,
        &LayoutConstraints::new(evaluated.placement().alignment()),
    )?;
    Ok((
        local.collision_bounds(),
        collision_footprint(
            &local,
            DeviceMapping::new(
                track.script().play_res(),
                Viewport::full(size).unwrap().content(),
            )?,
            track.script().scaled_border_shadow(),
            UnsupportedPolicy::Strict,
        )?,
    ))
}

#[test]
fn thousands_of_explicit_kfx_events_keep_automatic_frames_and_fit_bounded_staging() {
    let fonts = catalog();
    let auto = format!("{}{}", row("H"), row(r"{\1c&HFF0000&}H"));
    let expected = render(
        &source(&auto),
        fonts.clone(),
        Size::new(160, 100),
        Limits::default(),
    );
    let positioned = row(r"{\pos(-100,-100)\fs1}HHHHHHHHHHHHHHHHHHHH").repeat(2_000);
    let tiny = Limits::default()
        .with_transient_bytes(8 * 1024 * 1024)
        .unwrap();
    for input in [format!("{positioned}{auto}"), format!("{auto}{positioned}")] {
        let actual = render(
            &source(&input),
            fonts.clone(),
            Size::new(160, 100),
            tiny.clone(),
        );
        assert_eq!(actual.pixels(), expected.pixels());
    }
    let all_positioned = source(&positioned);
    assert!(
        render(&all_positioned, fonts, Size::new(160, 100), tiny)
            .pixels()
            .iter()
            .all(|&v| v == 0)
    );
}

#[test]
fn thick_borders_change_real_displacement_but_hidden_effects_do_not_pad() {
    let fonts = catalog();
    let size = Size::new(160, 100);
    let mut heights = Vec::new();
    for border in [2, 12] {
        let text = format!("{{\\bord{border}}}H");
        let input = source(&row(&text).repeat(2));
        let (_, support) = footprint(&input, &fonts, size).unwrap();
        heights.push(support.height());
        let expected = source(&format!(
            "{}{}",
            row(&format!("{{\\pos(13,9)}}{text}")),
            row(&format!("{{\\pos(13,{})}}{text}", 9.0 + support.height()))
        ));
        assert_eq!(
            render(&input, fonts.clone(), size, Limits::default()).pixels(),
            render(&expected, fonts.clone(), size, Limits::default()).pixels()
        );
    }
    assert!(heights[1] > heights[0]);
    let hidden = source(&row(
        r"{\bord12\3a&HFF&\4a&HFF&\shad1000000000000000000000000}H",
    ));
    let (base, support) = footprint(&hidden, &fonts, size).unwrap();
    assert_eq!(base, support);
    let shadow = source(&row(r"{\bord12\3a&HFF&\4a&H00&\shad3.5}H"));
    let (base, support) = footprint(&shadow, &fonts, size).unwrap();
    assert!(support.width() > base.width());
}

#[test]
fn effect_coordinate_spaces_rotation_multiple_runs_and_offscreen_geometry() {
    let fonts = catalog();
    for (sx, sy) in [(2, 2), (2, 3)] {
        let size = Size::new(160 * sx, 100 * sy);
        let a = source(&row(r"{\bord12\shad0\4a&HFF&}H"));
        let b = source(&row(r"{\bord6\shad0\4a&HFF&}H"))
            .replace("ScaledBorderAndShadow: no", "ScaledBorderAndShadow: yes");
        if sx == sy {
            assert_eq!(
                footprint(&a, &fonts, size).unwrap().1,
                footprint(&b, &fonts, size).unwrap().1
            );
        } else {
            assert_eq!(
                footprint(&b, &fonts, size).unwrap_err().code(),
                ErrorCode::UnsupportedFeature
            );
        }
        let shadow = source(&row(r"{\bord0\shad30.5\4a&H00&}H"));
        assert!(
            footprint(&shadow, &fonts, size).unwrap().1.width()
                > footprint(&shadow.replace("\\4a&H00&", "\\4a&HFF&"), &fonts, size)
                    .unwrap()
                    .1
                    .width()
        );
    }
    let rotated = source(&row(r"{\bord12\shad-2.25\4a&H00&}H{\bord1\shad3.5}H"))
        .replace(",9,0\n[Events]", ",9,90\n[Events]");
    let (base, support) = footprint(&rotated, &fonts, Size::new(160, 100)).unwrap();
    assert!(support.y0().get() < base.y0().get());
    let explicit = source(&row(r"{\pos(-2,-3)\bord12\shad3.5\4a&H00&}H"));
    assert!(
        render(&explicit, fonts, Size::new(160, 100), Limits::default())
            .pixels()
            .iter()
            .any(|&v| v != 0)
    );
}

#[test]
fn before_after_measurements_on_native_linux_actions() {
    if !cfg!(target_os = "linux")
        || std::env::var("RUSTUP_TOOLCHAIN").unwrap_or_default() != "1.99.1"
    {
        return;
    }
    let Some(folder) = std::env::var_os("TAIYOU_RENDER_SAMPLE_DIR") else {
        return;
    };
    let script =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../experiments/t017/measure.py");
    let result = std::process::Command::new("python")
        .arg(script)
        .arg(folder)
        .status()
        .unwrap();
    assert!(result.success());
}
