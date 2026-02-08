use pdf_ast::multimedia::av::{extract_audio_info, extract_video_info};
use pdf_ast::types::{PdfDictionary, PdfStream, PdfString, PdfValue};

#[test]
fn parse_sound_annotation() {
    let stream = PdfStream::new(PdfDictionary::new(), b"RIFFxxxxWAVE".to_vec());
    let mut annot = PdfDictionary::new();
    annot.insert("Subtype", PdfValue::Name("Sound".into()));
    annot.insert("Sound", PdfValue::Stream(stream));

    let info = extract_audio_info(&annot).unwrap();
    assert_eq!(info.format.as_deref(), Some("WAV"));
}

#[test]
fn parse_movie_annotation() {
    let mut movie = PdfDictionary::new();
    movie.insert("F", PdfValue::String(PdfString::new_literal(b"clip.mp4")));
    movie.insert("W", PdfValue::Integer(640));
    movie.insert("H", PdfValue::Integer(480));
    movie.insert("D", PdfValue::Real(2.5));

    let mut annot = PdfDictionary::new();
    annot.insert("Subtype", PdfValue::Name("Movie".into()));
    annot.insert("Movie", PdfValue::Dictionary(movie));

    let info = extract_video_info(&annot).unwrap();
    assert_eq!(info.format.as_deref(), Some("MP4"));
    assert_eq!(info.width, Some(640));
    assert_eq!(info.height, Some(480));
    assert_eq!(info.duration, Some(2.5));
}
