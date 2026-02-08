use pdf_ast::ast::{NodeType, PdfDocument, PdfVersion};
use pdf_ast::types::{
    ObjectId, PdfArray, PdfDictionary, PdfName, PdfReference, PdfString, PdfValue,
};
use pdf_ast::validation::SchemaRegistry;

fn build_pdf20_document() -> PdfDocument {
    let mut document = PdfDocument::new(PdfVersion { major: 2, minor: 0 });

    let catalog_value = PdfValue::Dictionary({
        let mut dict = PdfDictionary::new();
        dict.insert("Type", PdfValue::Name(PdfName::new("Catalog")));
        dict.insert("Pages", PdfValue::Reference(PdfReference::new(2, 0)));
        dict
    });
    let catalog_id = document.ast.create_node(NodeType::Catalog, catalog_value);
    document.set_catalog(catalog_id);

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

    document
}

#[test]
fn pdf20_iso_metadata_present() {
    let document = build_pdf20_document();
    let registry = SchemaRegistry::new();
    let report = registry.validate(&document, "PDF-2.0").expect("report");

    assert!(report.metadata.contains_key("iso.has-catalog"));
    assert!(report.metadata.contains_key("iso.has-pages-tree"));
}
