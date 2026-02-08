use pdf_ast::forms::XfaDocument;
use pdf_ast::types::{PdfArray, PdfDictionary, PdfStream, PdfString, PdfValue};

#[test]
fn parse_xfa_from_stream_packet() {
    let xml = b"<xfa><datasets><data>ok</data></datasets></xfa>".to_vec();
    let stream = PdfStream::new(PdfDictionary::new(), xml);

    let mut acroform = PdfDictionary::new();
    acroform.insert("XFA", PdfValue::Stream(stream));

    let doc = XfaDocument::from_acroform(&acroform).unwrap();
    assert_eq!(doc.packets.len(), 1);
    assert_eq!(doc.packets[0].root.name, "xfa");
}

#[test]
fn parse_xfa_from_array_packets() {
    let xml = PdfString::new_literal(b"<xfa><form>v</form></xfa>");
    let mut arr = PdfArray::new();
    arr.push(PdfValue::Name("form".into()));
    arr.push(PdfValue::String(xml));

    let mut acroform = PdfDictionary::new();
    acroform.insert("XFA", PdfValue::Array(arr));

    let doc = XfaDocument::from_acroform(&acroform).unwrap();
    assert_eq!(doc.packets.len(), 1);
    assert_eq!(doc.packets[0].name, "form");
}
