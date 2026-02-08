use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolyglotHit {
    pub format: String,
    pub offset: usize,
}

#[derive(Debug, Clone)]
struct Signature {
    name: &'static str,
    magic: &'static [u8],
}

const SIGNATURES: &[Signature] = &[
    Signature {
        name: "ZIP",
        magic: b"PK\x03\x04",
    },
    Signature {
        name: "ZIP_EOCD",
        magic: b"PK\x05\x06",
    },
    Signature {
        name: "RAR",
        magic: b"Rar!\x1A\x07\x00",
    },
    Signature {
        name: "7Z",
        magic: b"7z\xBC\xAF\x27\x1C",
    },
    Signature {
        name: "ELF",
        magic: b"\x7FELF",
    },
    Signature {
        name: "MZ",
        magic: b"MZ",
    },
    Signature {
        name: "OLE",
        magic: b"\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1",
    },
    Signature {
        name: "PNG",
        magic: b"\x89PNG\r\n\x1A\n",
    },
    Signature {
        name: "JPEG",
        magic: b"\xFF\xD8\xFF",
    },
    Signature {
        name: "GIF87a",
        magic: b"GIF87a",
    },
    Signature {
        name: "GIF89a",
        magic: b"GIF89a",
    },
    Signature {
        name: "MP4",
        magic: b"ftyp",
    },
];

pub fn scan_signature_hits(buffer: &[u8], base_offset: u64) -> Vec<PolyglotHit> {
    let mut hits = Vec::new();

    for sig in SIGNATURES {
        if sig.magic.is_empty() || buffer.len() < sig.magic.len() {
            continue;
        }

        for i in 0..=buffer.len() - sig.magic.len() {
            if &buffer[i..i + sig.magic.len()] == sig.magic {
                let offset = base_offset as usize + i;
                hits.push(PolyglotHit {
                    format: sig.name.to_string(),
                    offset,
                });
            }
        }
    }

    hits
}

pub fn detect_polyglot_hits(
    head: &[u8],
    head_offset: u64,
    tail: &[u8],
    tail_offset: u64,
) -> Vec<PolyglotHit> {
    let mut hits = scan_signature_hits(head, head_offset);
    hits.extend(scan_signature_hits(tail, tail_offset));

    let mut unique: HashMap<(String, usize), PolyglotHit> = HashMap::new();
    for hit in hits {
        unique
            .entry((hit.format.clone(), hit.offset))
            .or_insert(hit);
    }

    unique.into_values().collect()
}

pub fn detect_trailing_data(buffer: &[u8], base_offset: u64) -> Option<usize> {
    if let Some(eof_pos) = find_last(buffer, b"%%EOF") {
        let after = &buffer[eof_pos + 5..];
        for (idx, b) in after.iter().enumerate() {
            if !b.is_ascii_whitespace() {
                return Some(base_offset as usize + eof_pos + 5 + idx);
            }
        }
    }
    None
}

pub fn count_eof_markers(buffer: &[u8]) -> usize {
    let mut count = 0;
    let mut pos = 0;
    while let Some(rel) = find_first(&buffer[pos..], b"%%EOF") {
        count += 1;
        pos += rel + 5;
    }
    count
}

fn find_first(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return None;
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn find_last(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return None;
    }
    haystack.windows(needle.len()).rposition(|w| w == needle)
}
