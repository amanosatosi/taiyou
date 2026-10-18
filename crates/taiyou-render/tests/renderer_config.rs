use taiyou_render::renderer::*;
#[test]
fn checked_viewport_and_nine_margin_anchors() {
    let size=Size::new(160,100);
    assert!(Viewport::full(Size::new(0,1)).is_err());
    assert!(Viewport::new(size,PixelRect::new(-1,0,100,50).unwrap()).is_err());
    assert!(Viewport::new(size,PixelRect::new(0,0,161,100).unwrap()).is_err());
    let viewport=Viewport::new(size,PixelRect::new(7,5,140,90).unwrap()).unwrap();
    assert_eq!(viewport.content().x0(),7);
    for a in 1..=9 {
        let style=Style::core_default().with_alignment(Alignment::new(a).unwrap()).with_margins(Margins::new(8,12,6));
        let placement=Placement::new(&style,Margins::new(0,0,0),Anchor::Default);
        let p=alignment_anchor(&placement,size).unwrap();
        assert_eq!(p.x().get(),[8.0,78.0,148.0][((a-1)%3) as usize]);
        assert_eq!(p.y().get(),[94.0,50.0,6.0][((a-1)/3) as usize]);
    }
    fn assert_send<T:Send>(){} assert_send::<Renderer>();
}
#[test]
fn bounded_diagnostics_merge_and_frame_lease() {
    let mut producer=DiagnosticBuffer::new();
    for _ in 0..(MAX_DIAGNOSTICS+11) {producer.push(Diagnostic::new(DiagnosticCode::MissingGlyph,None,None,"missing")).unwrap();}
    let mut combined=DiagnosticBuffer::new();combined.extend_finished(&producer.finish().unwrap()).unwrap();
    assert_eq!(combined.suppressed_count(),11);
    assert!(combined.extend_finished(&[Diagnostic::new(DiagnosticCode::DiagnosticsSuppressed,None,None,"forged")]).is_err());
    let l=Limits::default(); let mut output=OutputBudget::new(&l);
    let f=Frame::from_owned_pixels(Size::new(1,1),vec![0;4],&l,output.reserve(4).unwrap()).unwrap().with_diagnostics(combined).unwrap();
    let clone=f.clone();drop(f);assert_eq!(output.retained_bytes(),4);assert_eq!(clone.diagnostics().len(),MAX_DIAGNOSTICS+1);drop(clone);assert_eq!(output.retained_bytes(),0);
}
