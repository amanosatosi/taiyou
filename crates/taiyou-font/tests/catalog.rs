#![cfg(feature="native")]
use std::sync::Arc;
use taiyou_font::*;
use taiyou_types::*;
fn fixture(name:&str)->Vec<u8> {std::fs::read(std::env::var(name).expect("native CI must provision pinned fonts")).unwrap()}
fn request(family:&str,weight:u16,italic:bool)->FontRequest {FontRequest::new(Arc::from(family),weight,italic,false)}
#[test]
fn catalog_identity_dedup_order_and_owned_lifetime() {
    fn shared<T:Send+Sync>() {} shared::<FontCatalog>();
    let regular=fixture("TAIYOU_TEST_FONT"); let bold=fixture("TAIYOU_TEST_BOLD");
    let mut ctx=FontContext::new().unwrap(); assert_eq!(ctx.version(),(2,14,3));
    let mut builder=FontCatalogBuilder::new(Limits::default());
    let a=builder.register(&mut ctx,&regular,0).unwrap(); let b=builder.register(&mut ctx,&bold,0).unwrap();
    assert_ne!(a,b); assert_eq!(builder.register(&mut ctx,&regular,0).unwrap(),a);
    assert_eq!(builder.registered_bytes(),u64::try_from(regular.len()+bold.len()).unwrap());
    let old=builder.snapshot().unwrap(); let newer=builder.snapshot().unwrap();
    assert_eq!(old.revision().get()+1,newer.revision().get()); assert_eq!(old.faces().len(),2);
    assert_eq!(old.faces()[0].blob().id(),a); assert_eq!(old.faces()[1].blob().id(),b);
    assert_eq!(old.faces()[0].weight(),400); assert_eq!(old.faces()[1].weight(),700);
    let family=old.faces()[0].family(); assert_eq!(family,old.faces()[1].family());
    assert_eq!(resolve_family(&old,&request(&family.to_uppercase(),400,false),&Limits::default()).unwrap(),a);
    assert_eq!(resolve_family(&old,&request(family,700,false),&Limits::default()).unwrap(),b);
    drop(builder); drop(regular); drop(bold); drop(ctx);
    let mut reload=FontContext::new().unwrap(); assert_eq!(reload.glyph_count(old.faces()[0].blob()).unwrap(),2478);
}
#[test]
fn catalog_collection_face_indices_and_shared_byte_budget() {
    let collection=fixture("TAIYOU_TEST_COLLECTION"); let mut ctx=FontContext::new().unwrap();
    let limits=Limits::default().with_font_bytes(u64::try_from(collection.len()).unwrap()).unwrap();
    let mut builder=FontCatalogBuilder::new(limits);
    let a=builder.register(&mut ctx,&collection,0).unwrap(); let b=builder.register(&mut ctx,&collection,1).unwrap();
    assert_eq!(a.sha256(),b.sha256()); assert_ne!(a.face_index(),b.face_index());
    assert_eq!(builder.registered_bytes(),u64::try_from(collection.len()).unwrap());
    for index in [2,65536,u32::MAX] {assert_eq!(builder.register(&mut ctx,&collection,index).unwrap_err().code(),ErrorCode::InvalidArgument);}
    let catalog=builder.snapshot().unwrap(); assert_eq!(catalog.faces().len(),2);
    assert!(Arc::ptr_eq(&catalog.faces()[0].blob().shared_bytes(),&catalog.faces()[1].blob().shared_bytes()));
}
#[test]
fn catalog_budget_and_malformed_failures_do_not_corrupt_existing_snapshot() {
    let font=fixture("TAIYOU_TEST_FONT"); let bold=fixture("TAIYOU_TEST_BOLD"); let mut ctx=FontContext::new().unwrap();
    let limits=Limits::default().with_font_bytes(u64::try_from(font.len()).unwrap()).unwrap();
    let mut builder=FontCatalogBuilder::new(limits); let id=builder.register(&mut ctx,&font,0).unwrap(); let old=builder.snapshot().unwrap();
    assert_eq!(builder.register(&mut ctx,&bold,0).unwrap_err().code(),ErrorCode::LimitExceeded);
    assert_eq!(builder.registered_bytes(),u64::try_from(font.len()).unwrap());
    let mut malformed=FontCatalogBuilder::new(Limits::default());
    for length in [1,2,16,64,128,1024] {let e=malformed.register(&mut ctx,&vec![0xA5;length],0).unwrap_err(); assert_eq!(e.code(),ErrorCode::NativeFailure); assert!(e.native_code().is_some());}
    assert_eq!(malformed.registered_bytes(),0); assert!(malformed.snapshot().unwrap().faces().is_empty());
    assert_eq!(ctx.glyph_count(old.faces()[0].blob()).unwrap(),2478); assert_eq!(old.faces()[0].blob().id(),id);
    assert_eq!(builder.register(&mut ctx,&font,1).unwrap_err().code(),ErrorCode::InvalidArgument);
}
#[test]
fn catalog_default_full_unicode_fold_is_locale_free_without_normalization() {
    let limits=Limits::default();
    assert_eq!(fold_family("Straße",&limits).unwrap(),fold_family("STRASSE",&limits).unwrap());
    assert_eq!(fold_family("Σςσ",&limits).unwrap(),"σσσ");
    assert_eq!(fold_family("İI",&limits).unwrap(),"i\u{307}i");
    assert_ne!(fold_family("Ａ",&limits).unwrap(),fold_family("A",&limits).unwrap());
    assert_ne!(fold_family("É",&limits).unwrap(),fold_family("E\u{301}",&limits).unwrap());
    assert!(fold_family("family",&limits.with_transient_bytes(1).unwrap()).is_err());
}
#[test]
fn catalog_ties_follow_weight_then_italic_then_registration_order() {
    // Independently authored immutable metadata isolates C4 tie ordering from font
    // availability. Runtime catalogs are produced by the validated native builder.
    let bytes:Arc<[u8]>=Arc::from([1]); let mut owner=CatalogIngestor::new();
    let records=[(0,300,false),(1,500,true),(2,500,false),(3,500,false)].into_iter().map(|(ordinal,weight,italic)|FontRecord::new(FontBlob::new(FontId::new([ordinal;32],0),bytes.clone()),Arc::from("Family"),Arc::from("family"),weight,italic).unwrap()).collect::<Vec<_>>();
    let catalog=owner.publish(records.into()).unwrap(); let limits=Limits::default();
    assert_eq!(resolve_family(&catalog,&request("Family",400,true),&limits).unwrap().sha256(),&[1;32]);
    assert_eq!(resolve_family(&catalog,&request("Family",500,false),&limits).unwrap().sha256(),&[2;32]);
    assert_eq!(resolve_family(&catalog,&request("absent",400,false),&limits).unwrap_err().code(),ErrorCode::FontUnavailable);
}
