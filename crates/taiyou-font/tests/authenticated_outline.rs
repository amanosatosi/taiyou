#![cfg(feature = "native")]
use std::sync::Arc;
use taiyou_font::*;
use taiyou_types::*;

#[test]
fn catalog_authentication_preserves_untrusted_outline_checks_and_results() {
    let bytes: Arc<[u8]> = std::fs::read(std::env::var_os("TAIYOU_TEST_FONT").unwrap())
        .unwrap()
        .into();
    let mut context = FontContext::new().unwrap();
    let mut builder = FontCatalogBuilder::new(Limits::default());
    let id = builder.register(&mut context, &bytes, 0).unwrap();
    let catalog = builder.snapshot(&mut CatalogIngestor::new()).unwrap();
    let blob = catalog.faces()[0].blob();
    for glyph in [0, 1, 72, 2048] {
        let checked = context.outline(blob, glyph, &Limits::default()).unwrap();
        let authenticated = catalog.outline(&mut context, id, glyph, &Limits::default()).unwrap();
        assert_eq!(checked, authenticated);
    }
    let forged = FontBlob::from_unverified(FontId::new([0; 32], 0), bytes);
    assert_eq!(context.outline(&forged, 0, &Limits::default()).unwrap_err().code(), ErrorCode::InvalidArgument);
    assert_eq!(catalog.outline(&mut context, forged.id(), 0, &Limits::default()).unwrap_err().code(), ErrorCode::InvalidArgument);
    for limits in [Limits::default(), Limits::default().with_transient_bytes(1024).unwrap()] {
        let a = context.outline(blob, u32::MAX, &limits).unwrap_err();
        let b = catalog.outline(&mut context, id, u32::MAX, &limits).unwrap_err();
        assert_eq!(a.code(), b.code());
    }
}
