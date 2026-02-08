use crate::types::{PdfDictionary, PdfStream, PdfValue};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AudioInfo {
    pub format: Option<String>,
    pub byte_len: usize,
    pub channels: Option<i64>,
    pub sample_rate: Option<i64>,
    pub bits_per_sample: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VideoInfo {
    pub format: Option<String>,
    pub byte_len: usize,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub duration: Option<f64>,
}

pub fn extract_audio_info(annotation: &PdfDictionary) -> Option<AudioInfo> {
    let sound = annotation.get("Sound")?;
    let mut info = AudioInfo::default();

    match sound {
        PdfValue::Stream(stream) => {
            info.byte_len = stream
                .raw_data()
                .map(|d| d.len())
                .unwrap_or(stream.data.len());
            info.format = detect_audio_format(stream);
            read_sound_params(&stream.dict, &mut info);
        }
        PdfValue::Dictionary(dict) => {
            if let Some(stream) = extract_stream_from_dict(dict) {
                info.byte_len = stream
                    .raw_data()
                    .map(|d| d.len())
                    .unwrap_or(stream.data.len());
                info.format = detect_audio_format(stream);
            }
            read_sound_params(dict, &mut info);
        }
        _ => return None,
    }

    Some(info)
}

pub fn extract_video_info(annotation: &PdfDictionary) -> Option<VideoInfo> {
    let movie = annotation.get("Movie")?;
    let mut info = VideoInfo::default();

    if let PdfValue::Dictionary(movie_dict) = movie {
        if let Some(f) = movie_dict.get("F") {
            if let Some(fmt) = detect_filespec_format(f) {
                info.format = Some(fmt);
            }
        }
        if let Some(w) = movie_dict.get("W").and_then(|v| v.as_integer()) {
            info.width = Some(w);
        }
        if let Some(h) = movie_dict.get("H").and_then(|v| v.as_integer()) {
            info.height = Some(h);
        }
        if let Some(dur) = movie_dict.get("D").and_then(|v| v.as_real()) {
            info.duration = Some(dur);
        }
    }

    Some(info)
}

fn read_sound_params(dict: &PdfDictionary, info: &mut AudioInfo) {
    if let Some(ch) = dict.get("C").and_then(|v| v.as_integer()) {
        info.channels = Some(ch);
    }
    if let Some(rate) = dict.get("R").and_then(|v| v.as_integer()) {
        info.sample_rate = Some(rate);
    }
    if let Some(bits) = dict.get("B").and_then(|v| v.as_integer()) {
        info.bits_per_sample = Some(bits);
    }
}

fn detect_audio_format(stream: &PdfStream) -> Option<String> {
    if let Some(subtype) = stream
        .dict
        .get("Subtype")
        .and_then(|v| v.as_name())
        .map(|n| n.without_slash().to_string())
    {
        return Some(subtype);
    }

    let data = stream.raw_data()?;
    if data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WAVE" {
        return Some("WAV".to_string());
    }
    if data.len() >= 3 && &data[0..3] == b"ID3" {
        return Some("MP3".to_string());
    }
    if data.len() >= 4 && &data[0..4] == b"OggS" {
        return Some("OGG".to_string());
    }
    if data.len() >= 4 && &data[0..4] == b"fLaC" {
        return Some("FLAC".to_string());
    }
    None
}

fn detect_filespec_format(value: &PdfValue) -> Option<String> {
    match value {
        PdfValue::String(s) => guess_format_from_name(&s.decode_pdf_encoding()),
        PdfValue::Name(n) => guess_format_from_name(n.without_slash()),
        PdfValue::Dictionary(dict) => {
            if let Some(PdfValue::String(s)) = dict.get("UF").or_else(|| dict.get("F")) {
                return guess_format_from_name(&s.decode_pdf_encoding());
            }
            if let Some(PdfValue::Name(n)) = dict.get("UF").or_else(|| dict.get("F")) {
                return guess_format_from_name(n.without_slash());
            }
            None
        }
        _ => None,
    }
}

fn guess_format_from_name(name: &str) -> Option<String> {
    let lower = name.to_lowercase();
    if lower.ends_with(".mp4") {
        return Some("MP4".to_string());
    }
    if lower.ends_with(".mov") {
        return Some("MOV".to_string());
    }
    if lower.ends_with(".mkv") {
        return Some("MKV".to_string());
    }
    if lower.ends_with(".avi") {
        return Some("AVI".to_string());
    }
    if lower.ends_with(".mp3") {
        return Some("MP3".to_string());
    }
    if lower.ends_with(".wav") {
        return Some("WAV".to_string());
    }
    if lower.ends_with(".ogg") {
        return Some("OGG".to_string());
    }
    None
}

fn extract_stream_from_dict(dict: &PdfDictionary) -> Option<&PdfStream> {
    match dict.get("Stream")? {
        PdfValue::Stream(s) => Some(s),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PdfString;

    #[test]
    fn detect_audio_formats() {
        let stream = PdfStream::new(PdfDictionary::new(), b"RIFFxxxxWAVE".to_vec());
        assert_eq!(detect_audio_format(&stream), Some("WAV".to_string()));
    }

    #[test]
    fn detect_video_format_from_filespec() {
        let mut movie = PdfDictionary::new();
        movie.insert("F", PdfValue::String(PdfString::new_literal(b"clip.mp4")));

        let mut annot = PdfDictionary::new();
        annot.insert("Movie", PdfValue::Dictionary(movie));

        let info = extract_video_info(&annot).unwrap();
        assert_eq!(info.format, Some("MP4".to_string()));
    }
}
