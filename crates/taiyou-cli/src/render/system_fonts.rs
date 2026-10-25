//! Host-only font discovery before immutable renderer publication.
use super::*;
use taiyou_render::shaping::{ShapeOptions, SystemFontSource, resolve_family, shape_event};

pub(super) fn extend(
    parsed: &ParsedTrack,
    request: &RenderRequest,
    context: &mut FontContext,
    builder: &mut FontCatalogBuilder,
    limits: &Limits,
) -> Result<(), RenderError> {
    let evaluated = taiyou_render::evaluate_active(
        parsed.track(),
        request.now,
        Profile::CoreV1,
        request.unsupported,
        limits,
    )?;
    let mut ingestion = CatalogIngestor::new();
    let mut catalog = builder.snapshot(&mut ingestion).map_err(font_error)?;
    // Lazily create the OS collection only if explicit/attached faces cannot
    // satisfy a request. Fully covered pinned text never discovers host fonts.
    let mut source = None;
    let mut discover = |text: &str,
                        style: &ShapeStyle,
                        context: &mut FontContext,
                        builder: &mut FontCatalogBuilder|
     -> Result<(), RenderError> {
        if source.is_none() {
            source = Some(SystemFontSource::new(limits.clone()).map_err(font_error)?);
        }
        if let Some(source) = &mut source {
            #[cfg(windows)]
            source
                .extend_catalog_for_style(
                    context,
                    builder,
                    text,
                    style,
                    parsed.track().script().layout_res().map_or(1.0, |res| {
                        f64::from(res.height())
                            / f64::from(parsed.track().script().play_res().height())
                    }),
                )
                .map_err(font_error)?;
            #[cfg(not(windows))]
            source
                .extend_catalog(context, builder, text, style.font())
                .map_err(font_error)?;
        }
        Ok(())
    };
    for event in evaluated.events() {
        for run in event.runs().iter().filter(|r| r.drawing().is_none()) {
            if let Err(error) = resolve_family(&catalog, run.shaping().font(), limits) {
                if error.code() != ErrorCode::FontUnavailable {
                    return Err(font_error(error));
                }
                if !run.text().is_empty() {
                    // Resolve a primary face first; mixed-script text may need
                    // several fallbacks, discovered from shaping clusters below.
                    discover(" ", run.shaping(), context, builder)?;
                    catalog = builder.snapshot(&mut ingestion).map_err(font_error)?;
                }
            }
        }
        let shaped = shape_event(context, &catalog, event, &ShapeOptions::default(), limits)
            .map_err(font_error)?;
        for missing in shaped.missing() {
            let run = event
                .runs()
                .iter()
                .find(|r| {
                    r.decoded().is_some_and(|span| {
                        span.start() <= missing.decoded.start()
                            && missing.decoded.start() < span.end()
                    })
                })
                .ok_or_else(|| invalid("missing cluster has no evaluated font request"))?;
            discover(missing.decoded.text(), run.shaping(), context, builder)?;
        }
        #[cfg(windows)]
        for glyph in shaped.runs().iter().flat_map(|r| r.glyphs()) {
            let run = event
                .runs()
                .iter()
                .find(|r| {
                    r.decoded().is_some_and(|s| {
                        s.start() <= glyph.cluster().start() && glyph.cluster().start() < s.end()
                    })
                })
                .ok_or_else(|| invalid("host cluster has no evaluated font request"))?;
            let primary =
                resolve_family(&catalog, run.shaping().font(), limits).map_err(font_error)?;
            let ly = parsed.track().script().layout_res().map_or(1.0, |res| {
                f64::from(res.height()) / f64::from(parsed.track().script().play_res().height())
            });
            let size = FiniteF64::new(run.shaping().font_size().get() * ly).map_err(parse_error)?;
            let vertical =
                run.shaping().font().vertical() || run.shaping().font().family().starts_with('@');
            if catalog.needs_host_font_link(
                primary,
                glyph.font(),
                glyph.cluster().text(),
                size,
                run.shaping().font(),
                vertical,
            ) {
                discover(glyph.cluster().text(), run.shaping(), context, builder)?;
            }
        }
        if !shaped.missing().is_empty() || catalog.has_host_font_links() {
            catalog = builder.snapshot(&mut ingestion).map_err(font_error)?;
        }
    }
    for face in catalog.faces() {
        eprintln!(
            "font-catalog family={:?} id={:?}",
            face.family(),
            face.blob().id()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_fallback_keeps_explicit_latin_clusters_and_catalog_immutable() {
        let limits = playback_limits(true).unwrap();
        let mut context = FontContext::with_limits(limits.clone()).unwrap();
        let mut builder = FontCatalogBuilder::new(limits.clone());
        let bytes = std::fs::read(std::env::var("TAIYOU_TEST_FONT").unwrap()).unwrap();
        let explicit = builder.register(&mut context, &bytes, 0).unwrap();
        let before = builder.snapshot(&mut CatalogIngestor::new()).unwrap();
        let text = if cfg!(windows) {
            "〈a\u{301}〉"
        } else {
            "اa\u{301}ا"
        };
        let source = format!(
            "[Script Info]\nPlayResX: 1920\nPlayResY: 1080\n[V4+ Styles]\nFormat: Name,Fontname,Fontsize\nStyle: Default,Source Sans 3,60\n[Events]\nFormat: Start,End,Text\nDialogue: 0:00:00.00,0:00:10.00,{text}"
        );
        let parsed = taiyou_ass::parse_ass(
            source.as_bytes(),
            &ParseOptions::default(),
            &mut TrackIngestor::new(),
        )
        .unwrap();
        let request = RenderRequest {
            now: TimeUs::from_micros(1_000_000),
            viewport: Viewport::full(Size::new(1920, 1080)).unwrap(),
            unsupported: UnsupportedPolicy::Strict,
        };
        let event = taiyou_render::evaluate(
            parsed.track(),
            parsed.track().events()[0].id(),
            request.now,
            Profile::CoreV1,
        )
        .unwrap();
        let a = shape_event(
            &mut context,
            &before,
            &event,
            &ShapeOptions::default(),
            &limits,
        )
        .unwrap();
        assert!(
            !a.missing().is_empty(),
            "fixture must exercise missing clusters"
        );
        extend(&parsed, &request, &mut context, &mut builder, &limits).unwrap();
        let after = builder.snapshot(&mut CatalogIngestor::new()).unwrap();
        let b = shape_event(
            &mut context,
            &after,
            &event,
            &ShapeOptions::default(),
            &limits,
        )
        .unwrap();
        assert!(b.missing().is_empty());
        assert_eq!(before.faces().len(), 1);
        assert!(after.faces().len() > before.faces().len());
        let explicit_glyphs = |s: &ShapedEvent| {
            s.runs()
                .iter()
                .flat_map(|r| r.glyphs())
                .filter(|g| g.font() == explicit && g.glyph() != 0)
                .map(|g| {
                    (
                        g.glyph(),
                        g.cluster().text().to_owned(),
                        g.advance_x(),
                        g.offset_x(),
                        g.offset_y(),
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(explicit_glyphs(&a), explicit_glyphs(&b));
        assert!(explicit_glyphs(&b).iter().any(|g| g.1 == "a\u{301}"));
        let again = shape_event(
            &mut context,
            &before,
            &event,
            &ShapeOptions::default(),
            &limits,
        )
        .unwrap();
        assert_eq!(a, again, "host discovery cannot mutate explicit snapshots");
    }

    #[test]
    fn provider_substitute_resolves_uninstalled_family_without_changing_old_snapshot() {
        let limits = Limits::default();
        let mut context = FontContext::new().unwrap();
        let mut builder = FontCatalogBuilder::new(limits.clone());
        let before = builder.snapshot(&mut CatalogIngestor::new()).unwrap();
        let request = FontRequest::new(
            std::sync::Arc::from("Taiyou absent family 8e9c4937"),
            400,
            false,
            false,
        );
        let mut source = SystemFontSource::new(limits.clone()).unwrap();
        source
            .extend_catalog(&mut context, &mut builder, "A", &request)
            .unwrap();
        let after = builder.snapshot(&mut CatalogIngestor::new()).unwrap();
        assert!(resolve_family(&before, &request, &limits).is_err());
        let id = resolve_family(&after, &request, &limits).unwrap();
        assert!(after.faces().iter().any(|face| face.blob().id() == id));
        let copied = FontCatalogBuilder::from_catalog(&mut context, &after, limits.clone())
            .unwrap()
            .snapshot(&mut CatalogIngestor::new())
            .unwrap();
        assert_eq!(resolve_family(&copied, &request, &limits).unwrap(), id);
        let tiny = Limits::default().with_transient_bytes(1).unwrap();
        let mut source = SystemFontSource::new(tiny.clone()).unwrap();
        let mut limited = FontCatalogBuilder::new(tiny);
        assert!(
            source
                .extend_catalog(&mut context, &mut limited, "A", &request)
                .is_err()
        );
        assert_eq!(limited.registered_bytes(), 0);
    }
}
