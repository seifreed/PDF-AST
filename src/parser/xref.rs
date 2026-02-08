use crate::ast::document::XRefEntry;
use crate::ast::linearization::LinearizationInfo;
use crate::filters::decode_stream;
use crate::types::{ObjectId, PdfStream, PdfValue};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_while1},
    character::complete::{char, digit1, multispace0, space1},
    combinator::{map_res, opt},
    multi::many1,
    IResult,
};

type XRefParseResult<'a> = IResult<&'a [u8], (Vec<(ObjectId, XRefEntry)>, Option<PdfStream>)>;

pub fn parse_xref_table(input: &[u8]) -> IResult<&[u8], Vec<(ObjectId, XRefEntry)>> {
    let (input, _) = tag(b"xref")(input)?;
    let (input, _) = multispace0(input)?;
    let (input, sections) = many1(parse_xref_section)(input)?;

    let mut entries = Vec::new();
    for section in sections {
        entries.extend(section);
    }

    Ok((input, entries))
}

fn parse_xref_section(input: &[u8]) -> IResult<&[u8], Vec<(ObjectId, XRefEntry)>> {
    let (input, (start_obj, count)) = parse_xref_subsection_header(input)?;
    let (input, raw_entries) = many1(parse_xref_entry)(input)?;

    let mut entries = Vec::new();
    for (i, entry) in raw_entries.into_iter().take(count as usize).enumerate() {
        let obj_id = ObjectId::new(start_obj + i as u32, entry.generation());
        entries.push((obj_id, entry));
    }

    Ok((input, entries))
}

fn parse_xref_subsection_header(input: &[u8]) -> IResult<&[u8], (u32, u32)> {
    let (input, start_obj) = map_res(digit1, |s: &[u8]| {
        std::str::from_utf8(s).unwrap().parse::<u32>()
    })(input)?;
    let (input, _) = space1(input)?;
    let (input, count) = map_res(digit1, |s: &[u8]| {
        std::str::from_utf8(s).unwrap().parse::<u32>()
    })(input)?;
    let (input, _) = multispace0(input)?;

    Ok((input, (start_obj, count)))
}

fn parse_xref_entry(input: &[u8]) -> IResult<&[u8], XRefEntry> {
    let (input, offset) = map_res(take_while1(|c: u8| c.is_ascii_digit()), |s: &[u8]| {
        std::str::from_utf8(s).unwrap().parse::<u64>()
    })(input)?;
    let (input, _) = space1(input)?;
    let (input, generation) = map_res(take_while1(|c: u8| c.is_ascii_digit()), |s: &[u8]| {
        std::str::from_utf8(s).unwrap().parse::<u16>()
    })(input)?;
    let (input, _) = space1(input)?;
    let (input, status) = alt((char('n'), char('f')))(input)?;
    let (input, _) = multispace0(input)?;

    let entry = match status {
        'n' => XRefEntry::InUse { offset, generation },
        'f' => XRefEntry::Free {
            next_free_object: offset as u32,
            generation,
        },
        _ => unreachable!(),
    };

    Ok((input, entry))
}

/// Parse XRef Stream (PDF 1.5+)
/// XRef streams are compressed streams that contain the cross-reference information
pub fn parse_xref_stream(stream: &PdfStream) -> Result<Vec<(ObjectId, XRefEntry)>, String> {
    let dict = &stream.dict;

    // Get W array - widths of the three fields in each entry
    let w_array = dict
        .get("W")
        .and_then(|v| v.as_array())
        .ok_or("Missing W array in XRef stream")?;

    if w_array.len() != 3 {
        return Err("W array must have exactly 3 elements".to_string());
    }

    let w1 = w_array[0].as_integer().unwrap_or(0) as usize; // Type field width
    let w2 = w_array[1].as_integer().unwrap_or(0) as usize; // Field 2 width
    let w3 = w_array[2].as_integer().unwrap_or(0) as usize; // Field 3 width

    if w1 + w2 + w3 == 0 {
        return Err("Invalid W array - all widths are zero".to_string());
    }

    // Get Index array (or default to [0, Size])
    let index_array = dict
        .get("Index")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_integer())
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| {
            let size = dict.get("Size").and_then(|v| v.as_integer()).unwrap_or(0);
            vec![0, size]
        });

    // Decode the stream data
    let filters = stream.get_filters();
    let raw_data = match &stream.data {
        crate::types::StreamData::Raw(data) => data,
        crate::types::StreamData::Decoded(data) => data,
        crate::types::StreamData::Lazy(_) => {
            return Err("Cannot decode lazy stream data".to_string());
        }
    };

    let decoded_data = if filters.is_empty() {
        raw_data.clone()
    } else {
        decode_stream(raw_data, &filters)
            .map_err(|e| format!("Failed to decode XRef stream: {}", e))?
    };

    let mut entries = Vec::new();
    let mut data_offset = 0;
    let entry_size = w1 + w2 + w3;

    // Process each subsection defined in Index array
    for chunk in index_array.chunks(2) {
        if chunk.len() != 2 {
            continue;
        }

        let start = chunk[0] as u32;
        let count = chunk[1] as u32;

        for i in 0..count {
            if data_offset + entry_size > decoded_data.len() {
                break; // Not enough data for another entry
            }

            let entry_data = &decoded_data[data_offset..data_offset + entry_size];

            let entry = parse_xref_stream_entry(entry_data, w1, w2, w3)?;
            let obj_id = ObjectId::new(start + i, entry.generation());

            entries.push((obj_id, entry));
            data_offset += entry_size;
        }
    }

    Ok(entries)
}

/// Parse a single entry from an XRef stream
fn parse_xref_stream_entry(
    data: &[u8],
    w1: usize,
    w2: usize,
    w3: usize,
) -> Result<XRefEntry, String> {
    let mut offset = 0;

    // Field 1: Type (0 = free, 1 = normal, 2 = compressed)
    let type_field = if w1 > 0 {
        read_int_field(&data[offset..offset + w1])
    } else {
        1 // Default type is 1 (normal object)
    };
    offset += w1;

    // Field 2: Object number or offset
    let field2 = if w2 > 0 {
        read_int_field(&data[offset..offset + w2])
    } else {
        0
    };
    offset += w2;

    // Field 3: Generation or index
    let field3 = if w3 > 0 {
        read_int_field(&data[offset..offset + w3])
    } else {
        0
    };

    match type_field {
        0 => {
            // Free object entry
            Ok(XRefEntry::Free {
                next_free_object: field2 as u32,
                generation: field3 as u16,
            })
        }
        1 => {
            // Normal object entry
            Ok(XRefEntry::InUse {
                offset: field2,
                generation: field3 as u16,
            })
        }
        2 => {
            // Compressed object entry
            Ok(XRefEntry::Compressed {
                stream_object: field2 as u32,
                index: field3 as u32,
            })
        }
        _ => Err(format!("Invalid XRef entry type: {}", type_field)),
    }
}

/// Read an integer field from bytes (big-endian)
fn read_int_field(data: &[u8]) -> u64 {
    let mut result = 0u64;
    for &byte in data {
        result = (result << 8) | (byte as u64);
    }
    result
}

/// Linearized PDF parsing support
/// Linearized PDFs are optimized for web viewing and have a special structure
pub fn parse_linearization_dict(stream: &PdfStream) -> Result<LinearizationInfo, String> {
    let dict = &stream.dict;

    // Linearized PDFs must have a /Linearized entry
    if !dict.contains_key("Linearized") {
        return Err("Not a linearized PDF - missing /Linearized entry".to_string());
    }

    let linearized_version = dict
        .get("Linearized")
        .and_then(|v| v.as_real())
        .unwrap_or(1.0);

    let length = dict
        .get("L")
        .and_then(|v| v.as_integer())
        .ok_or("Missing /L (file length) in linearization dict")?;

    let hint_offset = dict
        .get("H")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.get(0))
        .and_then(|v| v.as_integer())
        .ok_or("Missing /H (hint stream offset) in linearization dict")?;

    let hint_length = dict
        .get("H")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.get(1))
        .and_then(|v| v.as_integer());

    let object_count = dict
        .get("N")
        .and_then(|v| v.as_integer())
        .ok_or("Missing /N (object count) in linearization dict")?;

    let first_page_offset = dict
        .get("O")
        .and_then(|v| v.as_integer())
        .ok_or("Missing /O (first page offset) in linearization dict")?;

    let first_page_end = dict
        .get("E")
        .and_then(|v| v.as_integer())
        .ok_or("Missing /E (first page end) in linearization dict")?;

    let main_xref_entries = dict
        .get("T")
        .and_then(|v| v.as_integer())
        .ok_or("Missing /T (main xref table entries) in linearization dict")?;

    Ok(LinearizationInfo {
        version: linearized_version,
        file_length: length as u64,
        hint_stream_offset: hint_offset as u64,
        hint_stream_length: hint_length.map(|l| l as u64),
        object_count: object_count as u32,
        first_page_object_number: first_page_offset as u32,
        first_page_end_offset: first_page_end as u64,
        main_xref_table_entries: main_xref_entries as u32,
    })
}

/// Parse hybrid XRef table/stream
/// Some PDFs use both traditional xref tables and xref streams
pub fn parse_hybrid_xref(input: &[u8]) -> XRefParseResult<'_> {
    // Try to parse traditional xref table first
    if let Ok((remaining, table_entries)) = parse_xref_table(input) {
        // Check if there's an xref stream following
        let (remaining, xref_stream) = opt(parse_xref_stream_object)(remaining)?;
        return Ok((remaining, (table_entries, xref_stream)));
    }

    // If no traditional table, try xref stream
    let (remaining, xref_stream) = parse_xref_stream_object(input)?;
    Ok((remaining, (Vec::new(), Some(xref_stream))))
}

/// Parse an XRef stream object
fn parse_xref_stream_object(input: &[u8]) -> IResult<&[u8], PdfStream> {
    // This is a simplified implementation - in practice, you'd use the full object parser
    use crate::parser::object_parser::parse_indirect_object;

    let (input, (_obj_id, value)) = parse_indirect_object(input)?;

    if let PdfValue::Stream(stream) = value {
        // Verify it's an XRef stream by checking for required entries
        if stream
            .dict
            .get("Type")
            .and_then(|v| v.as_name())
            .map(|n| n.as_str())
            == Some("/XRef")
        {
            Ok((input, stream))
        } else {
            Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Tag,
            )))
        }
    } else {
        Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        )))
    }
}

impl XRefEntry {
    pub fn generation(&self) -> u16 {
        match self {
            XRefEntry::InUse { generation, .. } | XRefEntry::Free { generation, .. } => *generation,
            XRefEntry::Compressed { .. } => 0,
        }
    }

    /// Check if this entry represents an object in use
    pub fn is_in_use(&self) -> bool {
        matches!(self, XRefEntry::InUse { .. } | XRefEntry::Compressed { .. })
    }

    /// Check if this entry represents a free object
    pub fn is_free(&self) -> bool {
        matches!(self, XRefEntry::Free { .. })
    }

    /// Get the offset for in-use objects
    pub fn offset(&self) -> Option<u64> {
        match self {
            XRefEntry::InUse { offset, .. } => Some(*offset),
            _ => None,
        }
    }
}
