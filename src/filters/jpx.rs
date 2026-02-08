use crate::filters::FilterError;

pub fn decode_jpx_to_codestream(data: &[u8]) -> Result<Vec<u8>, FilterError> {
    if data.len() < 2 {
        return Err(FilterError::InvalidData("JPX data too short".to_string()));
    }

    // Raw codestream (SOC marker 0xFF4F)
    if data.len() >= 2 && data[0] == 0xFF && data[1] == 0x4F {
        return Ok(data.to_vec());
    }

    // JP2 container signature box
    if data.len() < 12 {
        return Err(FilterError::InvalidData(
            "JP2 container too short".to_string(),
        ));
    }

    let mut pos = 0;
    let mut codestreams = Vec::new();
    let mut saw_signature = false;

    while pos + 8 <= data.len() {
        let length = u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
        let box_type = &data[pos + 4..pos + 8];
        pos += 8;

        let (box_len, header_extra) = if length == 1 {
            if pos + 8 > data.len() {
                return Err(FilterError::InvalidData(
                    "JP2 box missing extended length".to_string(),
                ));
            }
            let ext_len = u64::from_be_bytes([
                data[pos],
                data[pos + 1],
                data[pos + 2],
                data[pos + 3],
                data[pos + 4],
                data[pos + 5],
                data[pos + 6],
                data[pos + 7],
            ]);
            pos += 8;
            if ext_len < 16 {
                return Err(FilterError::InvalidData(
                    "JP2 box extended length invalid".to_string(),
                ));
            }
            (ext_len as usize, 16)
        } else if length == 0 {
            // box extends to end of file
            let remaining = data.len().saturating_sub(pos);
            (remaining + 8, 8)
        } else {
            (length as usize, 8)
        };

        if box_len < header_extra {
            return Err(FilterError::InvalidData(
                "JP2 box length invalid".to_string(),
            ));
        }

        let payload_len = box_len - header_extra;
        if pos + payload_len > data.len() {
            return Err(FilterError::InvalidData(
                "JP2 box length exceeds buffer".to_string(),
            ));
        }

        let payload = &data[pos..pos + payload_len];
        pos += payload_len;

        match box_type {
            b"jP  " => {
                if payload != b"\x0D\x0A\x87\x0A" {
                    return Err(FilterError::InvalidData(
                        "JP2 signature box invalid".to_string(),
                    ));
                }
                saw_signature = true;
            }
            b"jp2c" => {
                codestreams.push(payload.to_vec());
            }
            _ => {}
        }
    }

    if !saw_signature {
        return Err(FilterError::InvalidData(
            "JP2 signature not found".to_string(),
        ));
    }
    if codestreams.is_empty() {
        return Err(FilterError::InvalidData(
            "JP2 codestream not found".to_string(),
        ));
    }

    Ok(codestreams.concat())
}
