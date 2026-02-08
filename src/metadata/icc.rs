#[derive(Debug, Clone)]
pub struct IccProfileInfo {
    pub size: u32,
    pub cmm_type: String,
    pub version: String,
    pub device_class: String,
    pub color_space: String,
    pub pcs: String,
    pub signature: String,
}

pub fn parse_icc_profile(data: &[u8]) -> Option<IccProfileInfo> {
    if data.len() < 128 {
        return None;
    }

    let size = read_u32_be(&data[0..4]);
    let cmm_type = read_tag(&data[4..8]);
    let version = format!("{}.{}", data[8] >> 4, data[8] & 0x0F);
    let device_class = read_tag(&data[12..16]);
    let color_space = read_tag(&data[16..20]);
    let pcs = read_tag(&data[20..24]);
    let signature = read_tag(&data[36..40]);

    Some(IccProfileInfo {
        size,
        cmm_type,
        version,
        device_class,
        color_space,
        pcs,
        signature,
    })
}

fn read_u32_be(bytes: &[u8]) -> u32 {
    ((bytes[0] as u32) << 24)
        | ((bytes[1] as u32) << 16)
        | ((bytes[2] as u32) << 8)
        | (bytes[3] as u32)
}

fn read_tag(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| {
            if b.is_ascii_graphic() {
                *b as char
            } else {
                '?'
            }
        })
        .collect()
}
