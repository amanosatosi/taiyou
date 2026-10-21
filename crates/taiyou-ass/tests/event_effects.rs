use taiyou_ass::parse_ass;
use taiyou_types::*;

fn parsed(tags: &str) -> ParsedTrack {
    let input = format!("[V4+ Styles]\nFormat: Name,Fontsize\nStyle: Default,20\n[Events]\nFormat: Start,End,Text\nDialogue: 0:00:00.00,0:00:02.00,{{{tags}}}A");
    parse_ass(input.as_bytes(), &ParseOptions::new(InputEncoding::Utf8, Profile::CoreV1, Limits::default(), SyntaxPolicy::Strict), &mut TrackIngestor::new()).unwrap()
}
#[test]
fn event_effects_have_typed_values_and_exact_provenance() {
    for tag in [r"\move(-1.25,2.5,30,40)", r"\move(0,0,10,20,100.9,900tail)", r"\org(-1.25,2.5)", r"\fad(100,200)", r"\fade(255,80,200,0,100,900,1000)", r"\frz-30.5", r"\fr45"] {
        let p = parsed(tag);
        assert!(p.diagnostics().is_empty());
        let TextOp::Override(op) = &p.track().events()[0].program()[0] else { panic!("{tag}"); };
        assert_eq!(op.source().slice(p.track().source()), Some(tag));
        op.validate().unwrap();
        match op {
            OverrideOp::Move { from, to, times_ms, .. } => {
                if tag.contains("100.9") { assert_eq!(*times_ms, Some([100,900])); }
                else { assert_eq!(*times_ms, None); assert_eq!(from.x().get(), -1.25); assert_eq!(to.y().get(), 40.0); }
            }
            OverrideOp::SetOrigin { point, .. } => assert_eq!((point.x().get(),point.y().get()),(-1.25,2.5)),
            OverrideOp::Fade { alpha, times_ms, .. } => {
                if tag.starts_with(r"\fad(") { assert_eq!(*alpha,[255,0,255]); assert_eq!(*times_ms,[-1,100,200,-1]); }
                else { assert_eq!(*alpha,[255,80,200]); assert_eq!(*times_ms,[0,100,900,1000]); }
            }
            OverrideOp::TransformNumber { property, value, .. } => { assert_eq!(*property,TransformProperty::RotationZ); assert_eq!(value.unwrap().get(), if tag.starts_with(r"\frz") {-30.5} else {45.0}); }
            _ => panic!("wrong typed operation"),
        }
    }
}
#[test]
fn effect_arity_recovery_and_reference_numeric_prefixes() {
    for tag in [r"\move(1,2,3)", r"\pos(1,2,3)", r"\org(1)", r"\fad(1,2,3)", r"\fade", r"\move"] {
        let p = parsed(&format!(r"{tag}\pos(7,8)"));
        assert!(p.diagnostics().is_empty());
        assert_eq!(p.track().events()[0].program().len(),2);
        assert!(matches!(p.track().events()[0].program()[0],TextOp::Override(OverrideOp::SetPosition{point,..}) if point.x().get()==7.0));
    }
    let p = parsed(r"\pos(1e2,-2.5tail)");
    assert!(matches!(p.track().events()[0].program()[0],TextOp::Override(OverrideOp::SetPosition{point,..}) if point.x().get()==100.0 && point.y().get()==-2.5));
    for tag in [r"\frx10",r"\fry10",r"\fax1",r"\fay1",r"\clip(0,0,1,1)"] {
        let p=parsed(tag);
        assert!(matches!(p.track().events()[0].program()[0],TextOp::Unsupported{..}));
    }
}