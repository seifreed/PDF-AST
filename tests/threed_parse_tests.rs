use pdf_ast::multimedia::threed::extract_threed_info;
use pdf_ast::types::{PdfDictionary, PdfStream, PdfValue};

#[test]
fn threed_annotation_extracts_format() {
    let mut stream_dict = PdfDictionary::new();
    stream_dict.insert("Subtype", PdfValue::Name("U3D".into()));
    let stream = PdfStream::new(stream_dict, b"U3D\0data".to_vec());

    let mut annot = PdfDictionary::new();
    annot.insert("3DD", PdfValue::Stream(stream.clone()));

    let info = extract_threed_info(&annot, Some(&stream));
    assert_eq!(info.format.as_deref(), Some("U3D"));
    assert!(info.byte_len > 0);
}

#[test]
fn threed_annotation_flags() {
    let mut annot = PdfDictionary::new();
    annot.insert("3DV", PdfValue::Dictionary(PdfDictionary::new()));
    annot.insert("3DA", PdfValue::Dictionary(PdfDictionary::new()));

    let info = extract_threed_info(&annot, None);
    assert!(info.has_view);
    assert!(info.has_activation);
}
