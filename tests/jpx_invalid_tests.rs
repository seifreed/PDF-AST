use pdf_ast::filters::jpx::decode_jpx_to_codestream;

#[test]
fn reject_missing_signature() {
    let data = vec![0x00, 0x00, 0x00, 0x08, b'j', b'p', b'2', b'c'];
    assert!(decode_jpx_to_codestream(&data).is_err());
}

#[test]
fn reject_no_codestream() {
    let data = vec![
        0x00, 0x00, 0x00, 0x0C, b'j', b'P', b' ', b' ', 0x0D, 0x0A, 0x87, 0x0A,
    ];
    assert!(decode_jpx_to_codestream(&data).is_err());
}
