use pdf_ast::security::{security_info_to_report, security_report_to_json, SecurityAnalyzer};
use pdf_ast::types::{PdfDictionary, PdfName, PdfString, PdfValue};
use pdf_ast::{ast::NodeType, PdfDocument, PdfVersion};
use std::io::Cursor;

#[test]
fn security_report_serializes_to_json() {
    let mut document = PdfDocument::new(PdfVersion { major: 1, minor: 7 });
    let mut dict = PdfDictionary::new();
    dict.insert("Type", PdfValue::Name(PdfName::new("Sig")));
    dict.insert(
        "SubFilter",
        PdfValue::Name(PdfName::new("adbe.pkcs7.detached")),
    );
    dict.insert("Contents", PdfValue::String(PdfString::new_hex(b"00")));
    let sig_id = document
        .ast
        .create_node(NodeType::Signature, PdfValue::Dictionary(dict));
    document.ast.set_root(sig_id);

    let mut reader = Cursor::new(Vec::new());
    let info = SecurityAnalyzer::analyze_document(
        &document,
        &mut reader,
        pdf_ast::crypto::CryptoConfig::default(),
    );
    let report = security_info_to_report(info);
    let json = security_report_to_json(&report).expect("json");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
    assert_eq!(value["report_format_version"], "1.0");
    assert!(value["security"].is_object());
}
