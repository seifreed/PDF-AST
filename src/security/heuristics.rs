use crate::ast::PdfDocument;
use crate::security::{ValidationResult, ValidationStatus};
use crate::types::ObjectId;
use std::io::{Read, Seek, SeekFrom};

use super::polyglot;

const MAX_CHUNK_SIZE: usize = 256 * 1024;
const MAX_FILE_SIZE_FOR_FULL_SCAN: u64 = 20 * 1024 * 1024;
const MAX_OBJECTS_TO_SCAN: usize = 50_000;

pub fn analyze_document_heuristics<R: Read + Seek>(
    document: &PdfDocument,
    reader: &mut R,
) -> Result<Vec<ValidationResult>, String> {
    let current_pos = reader
        .seek(SeekFrom::Current(0))
        .map_err(|e| e.to_string())?;
    let file_size = reader.seek(SeekFrom::End(0)).map_err(|e| e.to_string())?;
    reader.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;

    let (head, head_offset, tail, tail_offset) = read_head_tail(reader, file_size, MAX_CHUNK_SIZE)?;

    let mut results = Vec::new();

    check_trailing_data(&tail, tail_offset, &mut results);
    check_eof_markers(&tail, &mut results);
    check_startxref_markers(&tail, document, &mut results);
    check_polyglot_signatures(&head, head_offset, &tail, tail_offset, &mut results);
    check_xref_offsets(document, file_size, &mut results);
    check_trailer_size(document, &mut results);
    check_missing_endobj(reader, file_size, &mut results)?;

    reader
        .seek(SeekFrom::Start(current_pos))
        .map_err(|e| e.to_string())?;

    Ok(results)
}

fn check_trailing_data(tail: &[u8], tail_offset: u64, results: &mut Vec<ValidationResult>) {
    if let Some(offset) = polyglot::detect_trailing_data(tail, tail_offset) {
        results.push(ValidationResult {
            check_type: "Heuristics:TrailingData".to_string(),
            status: ValidationStatus::Warning,
            message: format!("Data found after %%EOF at offset {}", offset),
        });
    }
}

fn check_eof_markers(tail: &[u8], results: &mut Vec<ValidationResult>) {
    let eof_count = polyglot::count_eof_markers(tail);
    if eof_count > 1 {
        results.push(ValidationResult {
            check_type: "Heuristics:MultipleEOF".to_string(),
            status: ValidationStatus::Warning,
            message: format!("Multiple EOF markers detected in tail: {}", eof_count),
        });
    }
}

fn check_startxref_markers(
    tail: &[u8],
    document: &PdfDocument,
    results: &mut Vec<ValidationResult>,
) {
    let startxref_count = count_occurrences(tail, b"startxref");
    if document.revisions.len() <= 1 && startxref_count > 1 {
        results.push(ValidationResult {
            check_type: "Heuristics:MultipleStartXref".to_string(),
            status: ValidationStatus::Warning,
            message: format!(
                "Multiple startxref markers detected without revisions: {}",
                startxref_count
            ),
        });
    }
}

fn check_polyglot_signatures(
    head: &[u8],
    head_offset: u64,
    tail: &[u8],
    tail_offset: u64,
    results: &mut Vec<ValidationResult>,
) {
    let hits = polyglot::detect_polyglot_hits(head, head_offset, tail, tail_offset);
    for hit in hits {
        results.push(ValidationResult {
            check_type: "IOC:Polyglot".to_string(),
            status: ValidationStatus::Warning,
            message: format!("Signature {} detected at offset {}", hit.format, hit.offset),
        });
    }
}

fn check_xref_offsets(document: &PdfDocument, file_size: u64, results: &mut Vec<ValidationResult>) {
    let first_bad_offset = document.xref.entries.iter().find_map(|(obj_id, entry)| {
        if let crate::ast::document::XRefEntry::InUse { offset, .. } = entry {
            if *offset >= file_size {
                return Some((*obj_id, *offset));
            }
        }
        None
    });

    if let Some((obj_id, offset)) = first_bad_offset {
        results.push(ValidationResult {
            check_type: "Heuristics:XRefOffsetOutOfBounds".to_string(),
            status: ValidationStatus::Warning,
            message: format!(
                "XRef points past EOF (object {}, offset {}, size {})",
                format_object_id(obj_id),
                offset,
                file_size
            ),
        });
    }
}

fn check_trailer_size(document: &PdfDocument, results: &mut Vec<ValidationResult>) {
    let expected = document
        .trailer
        .get("Size")
        .and_then(|v| v.as_integer())
        .map(|size| size as usize);

    if let Some(expected) = expected {
        let actual = document.xref.entries.len();
        if actual < expected {
            results.push(ValidationResult {
                check_type: "Heuristics:TrailerSizeMismatch".to_string(),
                status: ValidationStatus::Warning,
                message: format!(
                    "Trailer /Size is {}, but xref has {} entries",
                    expected, actual
                ),
            });
        }
    }
}

fn check_missing_endobj<R: Read + Seek>(
    reader: &mut R,
    file_size: u64,
    results: &mut Vec<ValidationResult>,
) -> Result<(), String> {
    if file_size > MAX_FILE_SIZE_FOR_FULL_SCAN {
        return Ok(());
    }

    reader.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
    let mut full = vec![0u8; file_size as usize];
    reader.read_exact(&mut full).map_err(|e| e.to_string())?;

    let missing = detect_missing_endobj_count(&full);
    if missing > 0 {
        results.push(ValidationResult {
            check_type: "Heuristics:MissingEndObj".to_string(),
            status: ValidationStatus::Warning,
            message: format!("Objects without endobj marker: {}", missing),
        });
    }

    Ok(())
}

fn read_head_tail<R: Read + Seek>(
    reader: &mut R,
    file_size: u64,
    max_chunk: usize,
) -> Result<(Vec<u8>, u64, Vec<u8>, u64), String> {
    let head_len = std::cmp::min(file_size as usize, max_chunk);
    reader.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
    let mut head = vec![0u8; head_len];
    reader.read_exact(&mut head).map_err(|e| e.to_string())?;

    let tail_len = std::cmp::min(file_size as usize, max_chunk);
    let tail_offset = file_size.saturating_sub(tail_len as u64);
    reader
        .seek(SeekFrom::Start(tail_offset))
        .map_err(|e| e.to_string())?;
    let mut tail = vec![0u8; tail_len];
    reader.read_exact(&mut tail).map_err(|e| e.to_string())?;

    Ok((head, 0, tail, tail_offset))
}

fn format_object_id(id: ObjectId) -> String {
    format!("{} {}", id.number, id.generation)
}

pub fn detect_missing_endobj_count(buffer: &[u8]) -> usize {
    let obj_marker = b" obj";
    let endobj_marker = b"endobj";

    let object_positions: Vec<usize> = buffer
        .windows(obj_marker.len())
        .enumerate()
        .filter(|(_, window)| *window == obj_marker)
        .map(|(i, _)| i)
        .take(MAX_OBJECTS_TO_SCAN)
        .collect();

    object_positions
        .iter()
        .enumerate()
        .filter(|(idx, &obj_pos)| {
            let search_end = object_positions
                .get(idx + 1)
                .copied()
                .unwrap_or(buffer.len());
            !contains_sequence(&buffer[obj_pos..search_end], endobj_marker)
        })
        .count()
}

fn contains_sequence(buffer: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return false;
    }
    buffer.windows(needle.len()).any(|window| window == needle)
}

fn count_occurrences(buffer: &[u8], needle: &[u8]) -> usize {
    if needle.is_empty() || buffer.len() < needle.len() {
        return 0;
    }

    let mut count = 0;
    let mut pos = 0;

    while pos <= buffer.len() - needle.len() {
        if let Some(rel) = find_subsequence(&buffer[pos..], needle) {
            count += 1;
            pos += rel + needle.len();
        } else {
            break;
        }
    }

    count
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
