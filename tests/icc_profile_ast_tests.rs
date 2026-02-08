use pdf_ast::parser::PdfParser;

fn build_icc_pdf() -> Vec<u8> {
    let mut icc = vec![0u8; 128];
    icc[0..4].copy_from_slice(&128u32.to_be_bytes());
    icc[4..8].copy_from_slice(b"Lino");
    icc[8] = 0x02 << 4;
    icc[12..16].copy_from_slice(b"scnr");
    icc[16..20].copy_from_slice(b"RGB ");
    icc[20..24].copy_from_slice(b"XYZ ");
    icc[36..40].copy_from_slice(b"acsp");

    let mut pdf: Vec<u8> = Vec::new();
    let push_str = |s: &str, buf: &mut Vec<u8>| {
        buf.extend_from_slice(s.as_bytes());
    };

    push_str("%PDF-1.7\n", &mut pdf);

    let obj1_offset = pdf.len();
    push_str(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
        &mut pdf,
    );

    let obj2_offset = pdf.len();
    push_str(
        "2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n",
        &mut pdf,
    );

    let obj3_offset = pdf.len();
    push_str("3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /ColorSpace << /CS1 [/ICCBased 5 0 R] >> >> >>\nendobj\n", &mut pdf);

    let obj5_offset = pdf.len();
    push_str("5 0 obj\n<< /N 3 /Length ", &mut pdf);
    push_str(&format!("{}", icc.len()), &mut pdf);
    push_str(" >>\nstream\n", &mut pdf);
    pdf.extend_from_slice(&icc);
    push_str("\nendstream\nendobj\n", &mut pdf);

    let xref_offset = pdf.len();
    push_str("xref\n0 6\n", &mut pdf);
    push_str("0000000000 65535 f \n", &mut pdf);
    push_str(&format!("{:010} 00000 n \n", obj1_offset), &mut pdf);
    push_str(&format!("{:010} 00000 n \n", obj2_offset), &mut pdf);
    push_str(&format!("{:010} 00000 n \n", obj3_offset), &mut pdf);
    push_str("0000000000 00000 f \n", &mut pdf);
    push_str(&format!("{:010} 00000 n \n", obj5_offset), &mut pdf);
    push_str("trailer\n<< /Size 6 /Root 1 0 R >>\n", &mut pdf);
    push_str("startxref\n", &mut pdf);
    push_str(&format!("{}\n", xref_offset), &mut pdf);
    push_str("%%EOF\n", &mut pdf);

    pdf
}

#[test]
fn icc_profile_nodes_present() {
    let parser = PdfParser::new();
    let data = build_icc_pdf();
    let document = parser.parse_bytes(&data).expect("parse icc pdf");

    let mut found = false;
    for node in document.ast.get_all_nodes() {
        if node
            .metadata
            .properties
            .get("metadata_kind")
            .map(|v| v == "icc_profile")
            .unwrap_or(false)
        {
            found = true;
            break;
        }
    }

    assert!(found, "expected icc_profile metadata node");
}
