use taiyou_ass::parse_ass;
use taiyou_types::*;

fn parse(styles: &str) -> ParsedTrack {
    parse_ass(styles.as_bytes(), &ParseOptions::new(InputEncoding::Utf8, Profile::CoreV1, Limits::default(), SyntaxPolicy::Strict), &mut TrackIngestor::new()).unwrap()
}

#[test]
fn ssa_alpha_level_clamps_signed_bits_and_replaces_embedded_color_alpha() {
    // Both pinterf STS.cpp and libass ass.c clamp signed AlphaLevel before
    // assigning it to fill/secondary/outline; shadow transparency stays 128.
    for (alpha, opacity) in [("&HFF",0), ("&H30",207), ("&H00",255), ("&HFFFFFFEA",255), ("&H3FF",0), ("-22",255), ("300",0)] {
        let p = parse(&format!("[V4 Styles]\nFormat: Name,PrimaryColour,SecondaryColour,TertiaryColour,BackColour,AlphaLevel\nStyle: Default,&HFF123456,&HEF654321,&HABCDEF,&HFE13206A,{alpha}\n"));
        let s = &p.track().styles()[0];
        assert_eq!(s.primary().channels(), [0x56,0x34,0x12,opacity]);
        assert_eq!(s.secondary().channels(), [0x21,0x43,0x65,opacity]);
        assert_eq!(s.outline().channels(), [0x6A,0x20,0x13,opacity]);
        assert_eq!(s.shadow_color().channels(), [0x6A,0x20,0x13,127]);
    }
}

#[test]
fn ssa_alignment_and_decimal_colors_map_into_existing_style_values() {
    for (legacy, numpad) in [(1,1),(2,2),(3,3),(5,7),(6,8),(7,9),(9,4),(10,5),(11,6)] {
        let p = parse(&format!("[V4 Styles]\nFormat: Name,Alignment,PrimaryColour,BackColour\nStyle: Default,{legacy},-1,1253482\n"));
        let s = &p.track().styles()[0];
        assert_eq!(s.alignment().get(), numpad);
        assert_eq!(s.primary(), Rgba8::new(255,255,255,255));
        assert_eq!(s.outline(), Rgba8::new(0x6A,0x20,0x13,255));
    }
}

#[test]
fn switching_style_sections_does_not_apply_ssa_alpha_or_alignment_to_ass() {
    let p = parse("[V4 Styles]\nFormat: Name,Alignment,AlphaLevel\nStyle: Old,9,255\n[V4+ Styles]\nFormat: Name,Alignment,PrimaryColour,BackColour\nStyle: Default,9,&H30123456,&HFE13206A\n");
    let s = &p.track().styles()[1];
    assert_eq!(s.alignment().get(),9);
    assert_eq!(s.primary().a(),207);
    assert_eq!(s.shadow_color().a(),1);
}
