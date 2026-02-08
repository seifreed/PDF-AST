use pdf_ast::filters::ccitt::CcittDecoder;

fn pack_bits_lsb(bits: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut current = 0u8;
    let mut pos = 0u8;
    for &bit in bits {
        current |= (bit & 1) << pos;
        pos += 1;
        if pos == 8 {
            out.push(current.reverse_bits());
            current = 0;
            pos = 0;
        }
    }
    if pos != 0 {
        out.push(current.reverse_bits());
    }
    out
}

#[test]
fn decode_group3_1d_single_row_white() {
    let decoder = CcittDecoder::new(8, 1).with_black_is_1(true);
    let white8_bits = [1, 0, 0, 1, 1];
    let data = pack_bits_lsb(&white8_bits);
    let result = decoder.decode_group3_1d(&data).unwrap();
    assert_eq!(result, vec![0x00]);
}
