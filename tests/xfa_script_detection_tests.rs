use pdf_ast::forms::XfaDocument;
use pdf_ast::types::{PdfDictionary, PdfStream, PdfValue};

#[test]
fn detects_xfa_script_nodes() {
    let xml = b"<xfa><form><event><script>app.alert('x')</script></event></form></xfa>".to_vec();
    let stream = PdfStream::new(PdfDictionary::new(), xml);

    let mut acroform = PdfDictionary::new();
    acroform.insert("XFA", PdfValue::Stream(stream));

    let doc = XfaDocument::from_acroform(&acroform).unwrap();
    let stats = doc.script_stats();
    assert!(stats.has_scripts);
    assert!(stats.script_nodes >= 1);
}
