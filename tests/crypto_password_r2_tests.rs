use pdf_ast::crypto::encryption::{
    md5, rc4_encrypt, EncryptionAlgorithm, EncryptionInfo, PasswordValidator,
};

const PDF_PADDING: [u8; 32] = [
    0x28, 0xBF, 0x4E, 0x5E, 0x4E, 0x75, 0x8A, 0x41, 0x64, 0x00, 0x4E, 0x56, 0xFF, 0xFA, 0x01, 0x08,
    0x2E, 0x2E, 0x00, 0xB6, 0xD0, 0x68, 0x3E, 0x80, 0x2F, 0x0C, 0xA9, 0xFE, 0x64, 0x53, 0x69, 0x7A,
];

fn pad_password(password: &[u8]) -> [u8; 32] {
    let mut padded = PDF_PADDING;
    let len = password.len().min(32);
    padded[..len].copy_from_slice(&password[..len]);
    padded
}

fn compute_encryption_key_r2(
    owner_key: &[u8],
    permissions: u32,
    file_id: &[u8],
    user_password: &[u8],
    key_length: u32,
) -> Vec<u8> {
    let padded_user = pad_password(user_password);
    let mut input = Vec::new();
    input.extend_from_slice(&padded_user);
    input.extend_from_slice(owner_key);
    input.extend_from_slice(&permissions.to_le_bytes());
    input.extend_from_slice(file_id);
    let digest = md5(&input);
    let key_len = (key_length as usize / 8).min(16);
    digest[..key_len].to_vec()
}

fn compute_owner_key_r2(owner_password: &[u8], user_password: &[u8], key_length: u32) -> Vec<u8> {
    let padded_owner = pad_password(owner_password);
    let digest = md5(&padded_owner);
    let key_len = (key_length as usize / 8).min(16);
    let key = &digest[..key_len];
    let padded_user = pad_password(user_password);
    rc4_encrypt(&padded_user, key)
}

#[test]
fn test_password_validation_r2() {
    let user_password = b"user";
    let owner_password = b"owner";
    let permissions = 0xFFFFF0C0;
    let file_id = b"fileid-r2";
    let key_length = 40;

    let owner_key = compute_owner_key_r2(owner_password, user_password, key_length);
    let encryption_key =
        compute_encryption_key_r2(&owner_key, permissions, file_id, user_password, key_length);
    let user_key = rc4_encrypt(&pad_password(user_password), &encryption_key);

    let info = EncryptionInfo {
        algorithm: EncryptionAlgorithm::RC4,
        version: 1,
        revision: 2,
        key_length,
        permissions,
        owner_key,
        user_key,
        filter: "Standard".to_string(),
        file_id: Some(file_id.to_vec()),
    };

    let validator = PasswordValidator;
    assert!(validator.validate_user_password("user", &info));
    assert!(!validator.validate_user_password("wrong", &info));
    assert!(validator.validate_owner_password("owner", &info));
    assert!(!validator.validate_owner_password("bad", &info));
}
