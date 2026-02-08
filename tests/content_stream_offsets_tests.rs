use pdf_ast::parser::PdfParser;

fn build_content_pdf() -> Vec<u8> {
    let stream = b"q\nBT 12 Tf 10 10 Td (Hi) Tj ET\nQ\n";

    let mut pdf = String::new();
    pdf.push_str("%PDF-1.4\n");

    let obj1_offset = pdf.len();
    pdf.push_str("1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let obj2_offset = pdf.len();
    pdf.push_str("2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let obj3_offset = pdf.len();
    pdf.push_str("3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R >>\nendobj\n");

    let obj4_offset = pdf.len();
    pdf.push_str("4 0 obj\n<< /Length ");
    pdf.push_str(&format!("{}", stream.len()));
    pdf.push_str(" >>\nstream\n");
    pdf.push_str(std::str::from_utf8(stream).unwrap());
    pdf.push_str("endstream\nendobj\n");

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
fn content_stream_operator_offsets_present() {
    let parser = PdfParser::new();
    let data = build_content_pdf();
    let document = parser.parse_bytes(&data).expect("parse content pdf");

    let mut found_offset = false;
    for node in document.ast.get_all_nodes() {
        if node
            .metadata
            .properties
            .contains_key("content_operator_index")
            && node.metadata.properties.contains_key("stream_local_offset")
        {
            found_offset = true;
            break;
        }
    }

    assert!(
        found_offset,
        "expected operator nodes with stream_local_offset"
    );
}
