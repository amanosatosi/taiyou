#![cfg(feature = "native")]
use std::sync::Arc;
use taiyou_font::*;
use taiyou_types::*;
fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(std::env::var(name).expect("native CI must provision pinned fonts")).unwrap()
}
fn request(family: &str, weight: u16, italic: bool) -> FontRequest {
    FontRequest::new(Arc::from(family), weight, italic, false)
}
#[test]
fn catalog_identity_dedup_order_and_owned_lifetime() {
    fn shared<T: Send + Sync>() {}
    shared::<ValidatedFontCatalog>();
    let regular = fixture("TAIYOU_TEST_FONT");
    let bold = fixture("TAIYOU_TEST_BOLD");
    let mut ingestion = CatalogIngestor::new();
    let mut ctx = FontContext::new().unwrap();
    assert_eq!(ctx.version(), (2, 14, 3));
    let mut builder = FontCatalogBuilder::new(Limits::default());
    let a = builder.register(&mut ctx, &regular, 0).unwrap();
    assert_eq!(
        a.sha256()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "08df266400933d3178d081a45f94a08814c3e55b4b7dd2e0ff69cb1329f13ab6"
    );
    let b = builder.register(&mut ctx, &bold, 0).unwrap();
    assert_ne!(a, b);
    assert_eq!(builder.register(&mut ctx, &regular, 0).unwrap(), a);
    assert_eq!(
        builder.registered_bytes(),
        u64::try_from(regular.len() + bold.len()).unwrap()
    );
    let old = builder.snapshot(&mut ingestion).unwrap();
    let newer = builder.snapshot(&mut ingestion).unwrap();
    assert_eq!(old.revision().get() + 1, newer.revision().get());
    assert_eq!(old.faces().len(), 2);
    assert_eq!(old.faces()[0].blob().id(), a);
    assert_eq!(old.faces()[1].blob().id(), b);
    assert_eq!(old.faces()[0].weight(), 400);
    assert_eq!(old.faces()[1].weight(), 700);
    let family = old.faces()[0].family();
    assert_eq!(family, old.faces()[1].family());
    assert_eq!(
        resolve_family(
            &old,
            &request(&family.to_uppercase(), 400, false),
            &Limits::default()
        )
        .unwrap(),
        a
    );
    assert_eq!(
        resolve_family(&old, &request(family, 700, false), &Limits::default()).unwrap(),
        b
    );
    drop(builder);
    drop(regular);
    drop(bold);
    drop(ctx);
    let mut reload = FontContext::new().unwrap();
    assert_eq!(reload.glyph_count(old.faces()[0].blob()).unwrap(), 2478);
}
#[test]
fn catalog_collection_face_indices_and_shared_byte_budget() {
    let collection = fixture("TAIYOU_TEST_COLLECTION");
    let mut ingestion = CatalogIngestor::new();
    let mut ctx = FontContext::new().unwrap();
    let limits = Limits::default()
        .with_font_bytes(u64::try_from(collection.len()).unwrap())
        .unwrap();
    let mut builder = FontCatalogBuilder::new(limits);
    let a = builder.register(&mut ctx, &collection, 0).unwrap();
    let b = builder.register(&mut ctx, &collection, 1).unwrap();
    assert_eq!(a.sha256(), b.sha256());
    assert_ne!(a.face_index(), b.face_index());
    assert_eq!(
        builder.registered_bytes(),
        u64::try_from(collection.len()).unwrap()
    );
    for index in [2, 65536, u32::MAX] {
        assert_eq!(
            builder
                .register(&mut ctx, &collection, index)
                .unwrap_err()
                .code(),
            ErrorCode::InvalidArgument
        );
    }
    let catalog = builder.snapshot(&mut ingestion).unwrap();
    assert_eq!(catalog.faces().len(), 2);
    assert!(Arc::ptr_eq(
        &catalog.faces()[0].blob().shared_bytes(),
        &catalog.faces()[1].blob().shared_bytes()
    ));
}
#[test]
fn catalog_budget_and_malformed_failures_do_not_corrupt_existing_snapshot() {
    let font = fixture("TAIYOU_TEST_FONT");
    let bold = fixture("TAIYOU_TEST_BOLD");
    let mut ingestion = CatalogIngestor::new();
    let mut ctx = FontContext::new().unwrap();
    let limits = Limits::default()
        .with_font_bytes(u64::try_from(font.len()).unwrap())
        .unwrap();
    let mut builder = FontCatalogBuilder::new(limits);
    let id = builder.register(&mut ctx, &font, 0).unwrap();
    let old = builder.snapshot(&mut ingestion).unwrap();
    assert_eq!(
        builder.register(&mut ctx, &bold, 0).unwrap_err().code(),
        ErrorCode::LimitExceeded
    );
    assert_eq!(
        builder.registered_bytes(),
        u64::try_from(font.len()).unwrap()
    );
    let mut malformed = FontCatalogBuilder::new(Limits::default());
    assert_eq!(
        malformed.register(&mut ctx, &[], 0).unwrap_err().code(),
        ErrorCode::InvalidArgument
    );
    for length in [1, 2, 16, 64, 128, 1024] {
        let e = malformed
            .register(&mut ctx, &vec![0xA5; length], 0)
            .unwrap_err();
        assert_eq!(e.code(), ErrorCode::NativeFailure);
        assert!(e.native_code().is_some());
    }
    assert_eq!(malformed.registered_bytes(), 0);
    assert!(
        malformed
            .snapshot(&mut ingestion)
            .unwrap()
            .faces()
            .is_empty()
    );
    assert_eq!(ctx.glyph_count(old.faces()[0].blob()).unwrap(), 2478);
    assert_eq!(old.faces()[0].blob().id(), id);
    assert_eq!(
        builder.register(&mut ctx, &font, 1).unwrap_err().code(),
        ErrorCode::InvalidArgument
    );
}
#[test]
fn catalog_default_full_unicode_fold_is_locale_free_without_normalization() {
    let limits = Limits::default();
    assert_eq!(
        fold_family("Straße", &limits).unwrap(),
        fold_family("STRASSE", &limits).unwrap()
    );
    assert_eq!(fold_family("Σςσ", &limits).unwrap(), "σσσ");
    assert_eq!(fold_family("İI", &limits).unwrap(), "i\u{307}i");
    assert_ne!(
        fold_family("Ａ", &limits).unwrap(),
        fold_family("A", &limits).unwrap()
    );
    assert_ne!(
        fold_family("É", &limits).unwrap(),
        fold_family("E\u{301}", &limits).unwrap()
    );
    assert!(fold_family("family", &limits.with_transient_bytes(1).unwrap()).is_err());
}
#[test]
fn catalog_ties_follow_weight_then_italic_then_registration_order() {
    // The synthetic 300/500/italic fixture and its original assertions now live
    // in the private selector unit test, never in validated runtime publication.
    let mut context = FontContext::new().unwrap();
    let mut ingestion = CatalogIngestor::new();
    let mut builder = FontCatalogBuilder::new(Limits::default());
    let regular = builder
        .register(&mut context, &fixture("TAIYOU_TEST_FONT"), 0)
        .unwrap();
    let bold = builder
        .register(&mut context, &fixture("TAIYOU_TEST_BOLD"), 0)
        .unwrap();
    let catalog = builder.snapshot(&mut ingestion).unwrap();
    let family = catalog.faces()[0].family();
    let limits = Limits::default();
    assert_eq!(
        resolve_family(&catalog, &request(family, 550, true), &limits).unwrap(),
        regular
    );
    assert_eq!(
        resolve_family(&catalog, &request(family, 700, false), &limits).unwrap(),
        bold
    );
    assert_eq!(
        resolve_family(&catalog, &request("absent", 400, false), &limits)
            .unwrap_err()
            .code(),
        ErrorCode::FontUnavailable
    );
}

#[test]
fn catalog_transient_reservations_fail_before_publication_and_native_work() {
    let font = fixture("TAIYOU_TEST_FONT");
    let mut ingestion = CatalogIngestor::new();
    let mut context = FontContext::new().unwrap();
    let limits = Limits::default().with_transient_bytes(1).unwrap();
    let mut builder = FontCatalogBuilder::new(limits.clone());
    assert_eq!(
        builder.register(&mut context, &font, 0).unwrap_err().code(),
        ErrorCode::LimitExceeded
    );
    assert_eq!(builder.registered_bytes(), 0);
    assert!(builder.snapshot(&mut ingestion).unwrap().faces().is_empty());
    let blob = FontBlob::from_unverified(FontId::new([0; 32], 0), font.into());
    let mut restricted = FontContext::with_limits(limits).unwrap();
    assert_eq!(
        restricted.glyph_count(&blob).unwrap_err().code(),
        ErrorCode::LimitExceeded
    );
}

#[test]
fn catalog_replacement_builders_share_one_session_identity_owner() {
    let mut ingestion = CatalogIngestor::new();
    let regular = fixture("TAIYOU_TEST_FONT");
    let bold = fixture("TAIYOU_TEST_BOLD");
    let mut context = FontContext::new().unwrap();
    let mut first = FontCatalogBuilder::new(Limits::default());
    first.register(&mut context, &regular, 0).unwrap();
    let old = first.snapshot(&mut ingestion).unwrap();
    drop(first);
    let mut replacement = FontCatalogBuilder::new(Limits::default());
    replacement.register(&mut context, &bold, 0).unwrap();
    let newer = replacement.snapshot(&mut ingestion).unwrap();
    assert_eq!(old.revision().get(), 0);
    assert_eq!(newer.revision().get(), 1);
    assert_ne!(old.faces()[0].blob().id(), newer.faces()[0].blob().id());
    assert_eq!(context.glyph_count(old.faces()[0].blob()).unwrap(), 2478);
}

#[test]
fn catalog_unverified_import_recomputes_identity_and_metadata_before_publication() {
    let mut context = FontContext::new().unwrap();
    let mut session = CatalogIngestor::new();
    let mut builder = FontCatalogBuilder::new(Limits::default());
    builder
        .register(&mut context, &fixture("TAIYOU_TEST_FONT"), 0)
        .unwrap();
    let verified = builder.snapshot(&mut session).unwrap();
    let original = &verified.faces()[0];
    let mut drafts = CatalogIngestor::new();
    let draft = drafts
        .publish_unverified(Arc::from([original.clone()]))
        .unwrap();
    let imported = validate_catalog(&draft, &mut context, &mut session, Limits::default()).unwrap();
    assert_eq!(verified.revision().get(), 0);
    assert_eq!(imported.revision().get(), 1);
    assert_eq!(imported.faces()[0].blob().id(), original.blob().id());
    assert_eq!(
        resolve_family(
            &imported,
            &request(original.family(), 400, false),
            &Limits::default()
        )
        .unwrap(),
        original.blob().id()
    );

    let forge = |blob, family: &str, folded: &str, weight, italic| {
        FontRecord::from_unverified(blob, Arc::from(family), Arc::from(folded), weight, italic)
            .unwrap()
    };
    let genuine_blob = original.blob().clone();
    let forged_identity = forge(
        FontBlob::from_unverified(FontId::new([0; 32], 0), genuine_blob.shared_bytes()),
        original.family(),
        original.folded_family(),
        original.weight(),
        original.italic(),
    );
    let conflicting_bytes = forge(
        FontBlob::from_unverified(genuine_blob.id(), fixture("TAIYOU_TEST_BOLD").into()),
        original.family(),
        original.folded_family(),
        original.weight(),
        original.italic(),
    );
    let cases = [
        vec![forged_identity],
        vec![original.clone(), conflicting_bytes],
        vec![original.clone(), original.clone()],
        vec![forge(
            genuine_blob.clone(),
            "Forged family",
            original.folded_family(),
            original.weight(),
            original.italic(),
        )],
        vec![forge(
            genuine_blob.clone(),
            original.family(),
            "forged folded family",
            original.weight(),
            original.italic(),
        )],
        vec![forge(
            genuine_blob.clone(),
            original.family(),
            original.folded_family(),
            700,
            original.italic(),
        )],
        vec![forge(
            genuine_blob,
            original.family(),
            original.folded_family(),
            original.weight(),
            !original.italic(),
        )],
    ];
    for records in cases {
        let draft = drafts.publish_unverified(records.into()).unwrap();
        assert_eq!(
            validate_catalog(&draft, &mut context, &mut session, Limits::default())
                .unwrap_err()
                .code(),
            ErrorCode::InvalidArgument
        );
    }
    let after = builder.snapshot(&mut session).unwrap();
    assert_eq!(
        after.revision().get(),
        2,
        "failed imports must not consume validated output identities"
    );
    assert_eq!(context.glyph_count(original.blob()).unwrap(), 2478);
}

#[test]
fn catalog_native_blob_consumption_rejects_forged_digests_and_face_indices() {
    let bytes: Arc<[u8]> = fixture("TAIYOU_TEST_FONT").into();
    let forged = FontBlob::from_unverified(FontId::new([0; 32], 0), bytes);
    let mut context = FontContext::new().unwrap();
    assert_eq!(
        context.glyph_count(&forged).unwrap_err().code(),
        ErrorCode::InvalidArgument
    );
    let mut builder = FontCatalogBuilder::new(Limits::default());
    let id = builder.register(&mut context, forged.bytes(), 0).unwrap();
    let bad_index = FontBlob::from_unverified(FontId::new(*id.sha256(), 1), forged.shared_bytes());
    assert_eq!(
        context.glyph_count(&bad_index).unwrap_err().code(),
        ErrorCode::InvalidArgument
    );
}
