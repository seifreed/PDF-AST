use pdf_ast::ast::NodeType;
use pdf_ast::ast::{PdfDocument, PdfVersion};
use pdf_ast::types::{
    ObjectId, PdfArray, PdfDictionary, PdfName, PdfReference, PdfString, PdfValue,
};
use pdf_ast::validation::{SchemaRegistry, ValidationSeverity};

fn build_pdf20_document() -> PdfDocument {
    let mut document = PdfDocument::new(PdfVersion { major: 2, minor: 0 });

    // Catalog
    let catalog_value = PdfValue::Dictionary({
        let mut dict = PdfDictionary::new();
        dict.insert("Type", PdfValue::Name(PdfName::new("Catalog")));
        dict.insert("Pages", PdfValue::Reference(PdfReference::new(2, 0)));
        dict
    });
    let catalog_id = document.ast.create_node(NodeType::Catalog, catalog_value);
    document.set_catalog(catalog_id);

    // Pages tree
    let pages_value = PdfValue::Dictionary({
        let mut dict = PdfDictionary::new();
        dict.insert("Type", PdfValue::Name(PdfName::new("Pages")));
        dict.insert("Count", PdfValue::Integer(1));
        dict.insert(
            "Kids",
            PdfValue::Array(PdfArray::from(vec![PdfValue::Reference(
                PdfReference::new(3, 0),
            )])),
        );
        dict
    });
    document.ast.create_node(NodeType::Pages, pages_value);

    // Page
    let page_value = PdfValue::Dictionary({
        let mut dict = PdfDictionary::new();
        dict.insert("Type", PdfValue::Name(PdfName::new("Page")));
        dict.insert(
            "MediaBox",
            PdfValue::Array(PdfArray::from(vec![
                PdfValue::Integer(0),
                PdfValue::Integer(0),
                PdfValue::Integer(100),
                PdfValue::Integer(100),
            ])),
        );
        dict
    });
    document.ast.create_node(NodeType::Page, page_value);

    // Trailer
    let mut trailer = PdfDictionary::new();
    trailer.insert("Root", PdfValue::Reference(PdfReference::new(1, 0)));
    trailer.insert("Size", PdfValue::Integer(4));
    trailer.insert(
        "ID",
        PdfValue::Array(PdfArray::from(vec![
            PdfValue::String(PdfString::new_literal(b"id1".to_vec())),
            PdfValue::String(PdfString::new_literal(b"id2".to_vec())),
        ])),
    );
    document.set_trailer(trailer);

    document.add_xref_entry(
        ObjectId::new(1, 0),
        pdf_ast::ast::document::XRefEntry::InUse {
            offset: 0,
            generation: 0,
        },
    );
    document.add_xref_entry(
        ObjectId::new(2, 0),
        pdf_ast::ast::document::XRefEntry::InUse {
            offset: 10,
            generation: 0,
        },
    );
    document.add_xref_entry(
        ObjectId::new(3, 0),
        pdf_ast::ast::document::XRefEntry::InUse {
            offset: 20,
            generation: 0,
        },
    );

    document
}

#[test]
fn pdf20_constraints_pass_on_minimal_document() {
    let document = build_pdf20_document();
    let registry = SchemaRegistry::new();
    let report = registry.validate(&document, "PDF-2.0").expect("report");

    let has_errors = report.issues.iter().any(|issue| {
        matches!(
            issue.severity,
            ValidationSeverity::Error | ValidationSeverity::Critical
        )
    });
    assert!(
        !has_errors,
        "expected no errors, got {} issues",
        report.issues.len()
    );
}

#[test]
fn pdf20_detects_pages_count_mismatch() {
    let mut document = build_pdf20_document();
    if let Some(pages_id) = document
        .ast
        .find_nodes_by_type(NodeType::Pages)
        .first()
        .copied()
    {
        if let Some(node) = document.ast.get_node_mut(pages_id) {
            if let PdfValue::Dictionary(dict) = &mut node.value {
                dict.insert("Count", PdfValue::Integer(2));
            }
        }
    }

    let registry = SchemaRegistry::new();
    let report = registry.validate(&document, "PDF-2.0").expect("report");

    let mismatch = report
        .issues
        .iter()
        .any(|issue| issue.code == "PAGES_COUNT_MISMATCH");
    assert!(mismatch, "expected pages count mismatch issue");
}

#[test]
fn pdf20_detects_bad_trailer_id() {
    let mut document = build_pdf20_document();
    let mut trailer = PdfDictionary::new();
    trailer.insert("Root", PdfValue::Reference(PdfReference::new(1, 0)));
    trailer.insert("Size", PdfValue::Integer(4));
    trailer.insert("ID", PdfValue::Integer(1));
    document.set_trailer(trailer);

    let registry = SchemaRegistry::new();
    let report = registry.validate(&document, "PDF-2.0").expect("report");

    let id_issue = report
        .issues
        .iter()
        .any(|issue| issue.code == "TRAILER_ID_INVALID");
    assert!(id_issue, "expected invalid trailer ID issue");
}
