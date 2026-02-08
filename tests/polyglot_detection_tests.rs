use pdf_ast::security::polyglot::{count_eof_markers, detect_polyglot_hits, detect_trailing_data};

#[test]
fn detect_polyglot_zip_signature() {
    let head = b"%PDF-1.7\n...PK\x03\x04";
    let hits = detect_polyglot_hits(head, 0, &[], 0);
    assert!(hits.iter().any(|h| h.format == "ZIP"));
}

#[test]
fn detect_trailing_data_after_eof() {
    let buffer = b"%PDF-1.7\n1 0 obj\nendobj\n%%EOF\nMZ";
    let offset = detect_trailing_data(buffer, 0).expect("trailing data offset");
    assert!(offset > 0);
}

#[test]
fn detect_multiple_eof_markers() {
    let buffer = b"%PDF-1.7\n%%EOF\n%%EOF";
    assert_eq!(count_eof_markers(buffer), 2);
}
