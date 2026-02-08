use pdf_ast::parser::PdfParser;

fn build_xmp_pdf() -> Vec<u8> {
    let xmp = r#"<?xpacket begin="\"" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:Description xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:xmp="http://ns.adobe.com/xap/1.0/">
<dc:title>Test Title</dc:title>
<xmp:CreatorTool>PDF-AST</xmp:CreatorTool>
</rdf:Description>
</rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#;

    let mut pdf = String::new();
    pdf.push_str("%PDF-1.7\n");

    let obj1_offset = pdf.len();
    pdf.push_str("1 0 obj\n<< /Type /Catalog /Pages 2 0 R /Metadata 4 0 R >>\nendobj\n");

    let obj2_offset = pdf.len();
    pdf.push_str("2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let obj3_offset = pdf.len();
    pdf.push_str("3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>\nendobj\n");

    let obj4_offset = pdf.len();
    pdf.push_str("4 0 obj\n<< /Type /Metadata /Subtype /XML /Length ");
    pdf.push_str(&format!("{}", xmp.len()));
    pdf.push_str(" >>\nstream\n");
    pdf.push_str(xmp);
    pdf.push_str("\nendstream\nendobj\n");

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
fn xmp_nodes_created() {
    let parser = PdfParser::new();
    let data = build_xmp_pdf();
    let document = parser.parse_bytes(&data).expect("parse xmp pdf");

    let mut has_property = false;
    for node in document.ast.get_all_nodes() {
        if node
            .metadata
            .properties
            .get("metadata_kind")
            .map(|v| v == "xmp_property")
            .unwrap_or(false)
        {
            has_property = true;
            break;
        }
    }

    assert!(has_property, "expected xmp_property nodes");
}
