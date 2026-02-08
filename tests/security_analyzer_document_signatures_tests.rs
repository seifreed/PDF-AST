use pdf_ast::security::SecurityAnalyzer;
use pdf_ast::types::{PdfDictionary, PdfName, PdfString, PdfValue};
use pdf_ast::{ast::NodeType, PdfDocument, PdfVersion};
use std::io::Cursor;

#[test]
fn analyze_document_extracts_signatures() {
    let mut document = PdfDocument::new(PdfVersion { major: 1, minor: 7 });
    let mut dict = PdfDictionary::new();
    dict.insert("Type", PdfValue::Name(PdfName::new("Sig")));
    dict.insert(
        "SubFilter",
        PdfValue::Name(PdfName::new("adbe.pkcs7.detached")),
    );
    dict.insert("Contents", PdfValue::String(PdfString::new_hex(b"00")));
    dict.insert("T", PdfValue::String(PdfString::new_literal(b"Sig1")));
    let sig_id = document
        .ast
        .create_node(NodeType::Signature, PdfValue::Dictionary(dict));
    document.ast.set_root(sig_id);

    let mut reader = Cursor::new(Vec::new());
    let report = SecurityAnalyzer::analyze_document(
        &document,
        &mut reader,
        pdf_ast::crypto::CryptoConfig::default(),
    );

    assert_eq!(report.signatures.len(), 1);
    assert_eq!(report.signatures[0].field_name, "Sig1");
}
