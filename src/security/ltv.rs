use crate::types::{PdfDictionary, PdfValue};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LtvInfo {
    pub has_dss: bool,
    pub vri_count: usize,
    pub certs_count: usize,
    pub ocsp_count: usize,
    pub crl_count: usize,
    pub timestamp_count: usize,
}

pub fn extract_ltv_info(dss: &PdfDictionary) -> LtvInfo {
    let mut info = LtvInfo::default();
    info.has_dss = true;

    info.certs_count = count_array_items(dss.get("Certs"));
    info.ocsp_count = count_array_items(dss.get("OCSPs"));
    info.crl_count = count_array_items(dss.get("CRLs"));
    info.timestamp_count = count_array_items(dss.get("TS"));
    info.vri_count = count_vri_entries(dss.get("VRI"));

    info
}

fn count_array_items(value: Option<&PdfValue>) -> usize {
    match value {
        Some(PdfValue::Array(arr)) => arr.len(),
        Some(PdfValue::Dictionary(_)) => 1,
        _ => 0,
    }
}

fn count_vri_entries(value: Option<&PdfValue>) -> usize {
    match value {
        Some(PdfValue::Dictionary(dict)) => dict.len(),
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PdfArray;

    #[test]
    fn parse_ltv_info() {
        let mut dss = PdfDictionary::new();
        let mut certs = PdfArray::new();
        certs.push(PdfValue::Dictionary(PdfDictionary::new()));
        certs.push(PdfValue::Dictionary(PdfDictionary::new()));
        dss.insert("Certs", PdfValue::Array(certs));

        let mut vri = PdfDictionary::new();
        vri.insert("A", PdfValue::Dictionary(PdfDictionary::new()));
        vri.insert("B", PdfValue::Dictionary(PdfDictionary::new()));
        dss.insert("VRI", PdfValue::Dictionary(vri));

        let info = extract_ltv_info(&dss);
        assert!(info.has_dss);
        assert_eq!(info.certs_count, 2);
        assert_eq!(info.vri_count, 2);
    }
}
