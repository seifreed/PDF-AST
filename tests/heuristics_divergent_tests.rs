use pdf_ast::security::heuristics::detect_missing_endobj_count;

#[test]
fn detect_missing_endobj() {
    let data = b"1 0 obj\n<< /Type /Catalog >>\n2 0 obj\n<< /Type /Pages >>\nendobj\n";
    let count = detect_missing_endobj_count(data);
    assert!(count >= 1);
}
