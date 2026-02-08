use pdf_ast::security::SecurityAnalyzer;
use pdf_ast::types::{PdfDictionary, PdfName, PdfString, PdfValue};
use pdf_ast::{ast::NodeType, PdfAstGraph};

#[test]
fn detects_javascript_and_uri_indicators() {
    let mut ast = PdfAstGraph::new();

    let mut js_action = PdfDictionary::new();
    js_action.insert("S", PdfValue::Name(PdfName::new("JavaScript")));
    js_action.insert(
        "JS",
        PdfValue::String(PdfString::new_literal(b"app.launchURL('http://evil')")),
    );
    let js_id = ast.create_node(NodeType::JavaScriptAction, PdfValue::Dictionary(js_action));
    ast.set_root(js_id);

    let mut uri_action = PdfDictionary::new();
    uri_action.insert("S", PdfValue::Name(PdfName::new("URI")));
    uri_action.insert(
        "URI",
        PdfValue::String(PdfString::new_literal(b"http://example.com")),
    );
    ast.create_node(NodeType::URIAction, PdfValue::Dictionary(uri_action));

    let report = SecurityAnalyzer::analyze(&ast);
    let messages = report
        .validation_results
        .iter()
        .map(|r| r.message.clone())
        .collect::<Vec<_>>();

    assert!(messages.iter().any(|m| m.contains("JavaScript")));
    assert!(messages.iter().any(|m| m.contains("External URI")));
}
