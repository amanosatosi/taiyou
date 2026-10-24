//! Actions-only diagnostic for the owner-requested MUA reduction; no oracle.
#[cfg(all(feature = "native", windows))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::path::PathBuf;
    use taiyou_render::{renderer::*, shaping::*};

    let paths: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    assert_eq!(paths.len(), 2, "expected ASS and explicit MUA font paths");
    let limits = Limits::default().with_transient_bytes(256 * 1024 * 1024)?;
    let mut context = FontContext::with_limits(limits.clone())?;
    let mut builder = FontCatalogBuilder::new(limits.clone());
    builder.register(&mut context, &std::fs::read(&paths[1])?, 0)?;
    let parsed = taiyou_ass::parse_ass(
        &std::fs::read(&paths[0])?,
        &ParseOptions::default(),
        &mut TrackIngestor::new(),
    )?;
    let event = taiyou_render::evaluate(
        parsed.track(),
        parsed.track().events()[0].id(),
        TimeUs::from_micros(1_000_000),
        Profile::CoreV1,
    )?;
    let options = ShapeOptions::default().with_kerning(false);
    let mut ingestion = CatalogIngestor::new();
    let explicit = builder.snapshot(&mut ingestion)?;
    let initial = shape_event(&mut context, &explicit, &event, &options, &limits)?;
    if !initial.missing().is_empty() {
        let mut host = SystemFontSource::new(limits.clone())?;
        for missing in initial.missing() {
            let run = event
                .runs()
                .iter()
                .find(|r| {
                    r.decoded().is_some_and(|s| {
                        s.start() <= missing.decoded.start() && missing.decoded.start() < s.end()
                    })
                })
                .ok_or("missing cluster has no font request")?;
            host.extend_catalog_for_style(
                &mut context,
                &mut builder,
                missing.decoded.text(),
                run.shaping(),
                1.0,
            )?;
        }
    }
    let catalog = builder.snapshot(&mut ingestion)?;
    let shaped = shape_event(&mut context, &catalog, &event, &options, &limits)?;
    for face in catalog.faces() {
        let id = face.blob().id();
        println!(
            "FACE family={:?} id={id:?} cell={:?} scale={:?} decorations={:?}",
            face.family(),
            catalog.ass_cell_metrics(id)?,
            catalog.ass_font_scale(id, FiniteF64::new(60.0)?)?,
            catalog.decoration_metrics(&mut context, id)?
        );
    }
    let primary = resolve_family(&catalog, event.runs()[0].shaping().font(), &limits)?;
    let mut pen_x = 0.0;
    for run in shaped.runs() {
        println!(
            "RUN text={:?} metrics={:?} face_metrics={:?}",
            run.decoded().text(),
            run.metrics(),
            run.face_metrics()
        );
        for g in run.glyphs() {
            println!(
                "GLYPH font={:?} id={} cluster={:?} advance=({:?},{:?}) offset=({:?},{:?})",
                g.font(),
                g.glyph(),
                g.cluster().text(),
                g.advance_x(),
                g.advance_y(),
                g.offset_x(),
                g.offset_y()
            );
            // Diagnostic only: preserve FreeType's design-unit commands and
            // expose their pre-placement native coordinates for comparison
            // with the independent GDI TextOut/GetPath capture. No raster or
            // reference expectation is generated from these observations.
            let scale = catalog
                .admitted_font_scale(
                    primary,
                    g.font(),
                    g.cluster().text(),
                    FiniteF64::new(60.0)?,
                    event.runs()[0].shaping().font(),
                    false,
                )?
                .get();
            let outline = catalog.outline(&mut context, g.font(), g.glyph(), &limits)?;
            println!(
                "NATIVE_GLYPH start={} glyph={} pen={} baseline={} scale={}",
                g.cluster().start(),
                g.glyph(),
                (pen_x + g.offset_x().get()) * 64.0,
                (run.metrics().ascent().get() + g.offset_y().get()) * 64.0,
                scale * 64.0
            );
            for op in outline.outline().ops() {
                let point = |p: FontPoint| (p.x().get(), p.y().get());
                match *op {
                    PathOp::MoveTo(p) => println!("NATIVE_PATH M {:?}", point(p)),
                    PathOp::LineTo(p) => println!("NATIVE_PATH L {:?}", point(p)),
                    PathOp::QuadTo(a, b) => println!("NATIVE_PATH Q {:?} {:?}", point(a), point(b)),
                    PathOp::CubicTo(a, b, c) => println!(
                        "NATIVE_PATH C {:?} {:?} {:?}",
                        point(a),
                        point(b),
                        point(c)
                    ),
                    PathOp::Close => println!("NATIVE_PATH Z"),
                }
            }
            pen_x += g.advance_x().get();
        }
    }
    assert!(
        shaped.missing().is_empty(),
        "diagnostic has missing clusters"
    );
    Ok(())
}

#[cfg(not(all(feature = "native", windows)))]
fn main() {}
