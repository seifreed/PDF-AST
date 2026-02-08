use pdf_ast::security::ltv::extract_ltv_info;
use pdf_ast::types::{PdfArray, PdfDictionary, PdfValue};

#[test]
fn parse_dss_counts() {
    let mut dss = PdfDictionary::new();

    let mut certs = PdfArray::new();
    certs.push(PdfValue::Dictionary(PdfDictionary::new()));
    certs.push(PdfValue::Dictionary(PdfDictionary::new()));
    dss.insert("Certs", PdfValue::Array(certs));

    let mut ocsp = PdfArray::new();
    ocsp.push(PdfValue::Dictionary(PdfDictionary::new()));
    dss.insert("OCSPs", PdfValue::Array(ocsp));

    let mut vri = PdfDictionary::new();
    vri.insert("hash1", PdfValue::Dictionary(PdfDictionary::new()));
    vri.insert("hash2", PdfValue::Dictionary(PdfDictionary::new()));
    dss.insert("VRI", PdfValue::Dictionary(vri));

    let info = extract_ltv_info(&dss);
    assert!(info.has_dss);
    assert_eq!(info.certs_count, 2);
    assert_eq!(info.ocsp_count, 1);
    assert_eq!(info.vri_count, 2);
}
