use pdf_ast::forms::{count_fields_in_acroform, has_hybrid_forms};
use pdf_ast::types::{PdfArray, PdfDictionary, PdfName, PdfString, PdfValue};

#[test]
fn acroform_field_stats_counts_nested() {
    let mut child = PdfDictionary::new();
    child.insert("T", PdfValue::String(PdfString::new_literal(b"Child")));
    child.insert("FT", PdfValue::Name(PdfName::new("Tx")));

    let mut kids = PdfArray::new();
    kids.push(PdfValue::Dictionary(child));

    let mut parent = PdfDictionary::new();
    parent.insert("Kids", PdfValue::Array(kids));

    let mut fields = PdfArray::new();
    fields.push(PdfValue::Dictionary(parent));

    let mut acroform = PdfDictionary::new();
    acroform.insert("Fields", PdfValue::Array(fields));

    let stats = count_fields_in_acroform(&acroform);
    assert_eq!(stats.field_count, 2);
}

#[test]
fn acroform_hybrid_detection() {
    let mut field = PdfDictionary::new();
    field.insert("T", PdfValue::String(PdfString::new_literal(b"Field1")));

    let mut fields = PdfArray::new();
    fields.push(PdfValue::Dictionary(field));

    let mut acroform = PdfDictionary::new();
    acroform.insert("Fields", PdfValue::Array(fields));

    assert!(has_hybrid_forms(true, &acroform));
    assert!(!has_hybrid_forms(false, &acroform));
}
