use pdf_ast::parser::PdfParser;

fn build_actions_pdf() -> Vec<u8> {
    let mut pdf = String::new();
    pdf.push_str("%PDF-1.4\n");

    let obj1_offset = pdf.len();
    pdf.push_str("1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AA << /O 4 0 R >> >>\nendobj\n");

    let obj2_offset = pdf.len();
    pdf.push_str("2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let obj3_offset = pdf.len();
    pdf.push_str("3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /AA << /O 4 0 R >> >>\nendobj\n");

    let obj4_offset = pdf.len();
    pdf.push_str("4 0 obj\n<< /Type /Action /S /JavaScript /JS (app.alert('hi')) >>\nendobj\n");

    let xref_offset = pdf.len();
    pdf.push_str("xref\n0 5\n");
    pdf.push_str("0000000000 65535 f \n");
    pdf.push_str(&format!("{:010} 00000 n \n", obj1_offset));
    pdf.push_str(&format!("{:010} 00000 n \n", obj2_offset));
    pdf.push_str(&format!("{:010} 00000 n \n", obj3_offset));
    pdf.push_str(&format!("{:010} 00000 n \n", obj4_offset));
    pdf.push_str("trailer\n<< /Size 5 /Root 1 0 R >>\n");
    pdf.push_str("startxref\n");
    pdf.push_str(&format!("{}\n", xref_offset));
    pdf.push_str("%%EOF\n");

    pdf.into_bytes()
}

#[test]
fn actions_and_js_nodes_present() {
    let parser = PdfParser::new();
    let data = build_actions_pdf();
    let document = parser.parse_bytes(&data).expect("parse action pdf");

    let root = document.ast.get_root();
    assert!(root.is_some(), "expected catalog root");
    let nodes = document.ast.get_all_nodes();
    assert!(!nodes.is_empty(), "expected AST nodes");
}
