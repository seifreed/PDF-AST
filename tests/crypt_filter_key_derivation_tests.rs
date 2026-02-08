use pdf_ast::filters::crypt::CryptFilter;

#[test]
fn test_object_key_derivation_salt_differs_for_aes() {
    let filter = CryptFilter::new();
    let base_key = b"base_key_16bytes";
    let object_number = 12;
    let generation = 0;

    let key_std = filter.derive_object_key(base_key, object_number, generation);
    let key_aes = filter.derive_object_key_aes(base_key, object_number, generation);

    assert_eq!(key_std.len(), 16);
    assert_eq!(key_aes.len(), 16);
    assert_ne!(key_std, key_aes);
}
