use pdf_ast::ast::NodeType;
use pdf_ast::parser::PdfParser;

fn build_font_cmap_pdf() -> Vec<u8> {
    let cmap = b"/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CMapName /Test def\n/CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> def\n1 begincodespacerange\n<00> <FF>\nendcodespacerange\n1 beginbfchar\n<41> <0041>\nendbfchar\nendcmap\nCMapName currentdict /CMap defineresource pop\nend\nend";

    let mut pdf = String::new();
    pdf.push_str("%PDF-1.7\n");

    let obj1_offset = pdf.len();
    pdf.push_str("1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let obj2_offset = pdf.len();
    pdf.push_str("2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let obj3_offset = pdf.len();
    pdf.push_str("3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /Font << /F1 4 0 R >> >> >>\nendobj\n");

    let obj4_offset = pdf.len();
    pdf.push_str("4 0 obj\n<< /Type /Font /Subtype /Type0 /BaseFont /Test /Encoding /WinAnsiEncoding /ToUnicode 5 0 R /DescendantFonts [6 0 R] >>\nendobj\n");

    let obj5_offset = pdf.len();
    pdf.push_str("5 0 obj\n<< /Length ");
    pdf.push_str(&format!("{}", cmap.len()));
    pdf.push_str(" >>\nstream\n");
    pdf.push_str(std::str::from_utf8(cmap).unwrap());
    pdf.push_str("\nendstream\nendobj\n");

    let obj6_offset = pdf.len();
    pdf.push_str("6 0 obj\n<< /Type /Font /Subtype /CIDFontType0 >>\nendobj\n");

    let xref_offset = pdf.len();
    pdf.push_str("xref\n0 7\n");
    pdf.push_str("0000000000 65535 f \n");
    pdf.push_str(&format!("{:010} 00000 n \n", obj1_offset));
    pdf.push_str(&format!("{:010} 00000 n \n", obj2_offset));
    pdf.push_str(&format!("{:010} 00000 n \n", obj3_offset));
    pdf.push_str(&format!("{:010} 00000 n \n", obj4_offset));
    pdf.push_str(&format!("{:010} 00000 n \n", obj5_offset));
    pdf.push_str(&format!("{:010} 00000 n \n", obj6_offset));
    pdf.push_str("trailer\n<< /Size 7 /Root 1 0 R >>\n");
    pdf.push_str("startxref\n");
    pdf.push_str(&format!("{}\n", xref_offset));
    pdf.push_str("%%EOF\n");

    pdf.into_bytes()
}

#[test]
fn font_cmap_nodes_present() {
    let parser = PdfParser::new();
    let data = build_font_cmap_pdf();
    let document = parser.parse_bytes(&data).expect("parse font cmap pdf");

    let has_encoding = !document
        .ast
        .find_nodes_by_type(NodeType::Encoding)
        .is_empty();
    let has_tounicode = !document
        .ast
        .find_nodes_by_type(NodeType::ToUnicode)
        .is_empty();

    assert!(has_encoding, "expected Encoding node");
    assert!(has_tounicode, "expected ToUnicode node");
}
