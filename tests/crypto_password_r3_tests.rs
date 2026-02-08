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

fn compute_encryption_key_r3(
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
    let mut digest = md5(&input);
    let key_len = (key_length as usize / 8).min(16);
    for _ in 0..50 {
        digest = md5(&digest[..key_len]);
    }
    digest[..key_len].to_vec()
}

fn compute_owner_key_r3(owner_password: &[u8], user_password: &[u8], key_length: u32) -> Vec<u8> {
    let padded_owner = pad_password(owner_password);
    let mut digest = md5(&padded_owner);
    for _ in 0..50 {
        digest = md5(&digest);
    }
    let key_len = (key_length as usize / 8).min(16);
    let key = &digest[..key_len];
    let mut data = pad_password(user_password).to_vec();
    for i in 0..20u8 {
        let mut k = key.to_vec();
        for b in &mut k {
            *b ^= i;
        }
        data = rc4_encrypt(&data, &k);
    }
    data
}

fn compute_user_key_r3(encryption_key: &[u8], file_id: &[u8]) -> Vec<u8> {
    let mut input = Vec::new();
    input.extend_from_slice(&PDF_PADDING);
    input.extend_from_slice(file_id);
    let mut data = md5(&input);
    data = rc4_encrypt(&data, encryption_key);
    for i in 1..20u8 {
        let mut k = encryption_key.to_vec();
        for b in &mut k {
            *b ^= i;
        }
        data = rc4_encrypt(&data, &k);
    }
    let mut result = Vec::with_capacity(32);
    result.extend_from_slice(&data);
    result.resize(32, 0u8);
    result
}

#[test]
fn test_password_validation_r3() {
    let user_password = b"user";
    let owner_password = b"owner";
    let permissions = 0xFFFFF0C0;
    let file_id = b"fileid-r3";
    let key_length = 128;

    let owner_key = compute_owner_key_r3(owner_password, user_password, key_length);
    let encryption_key =
        compute_encryption_key_r3(&owner_key, permissions, file_id, user_password, key_length);
    let user_key = compute_user_key_r3(&encryption_key, file_id);

    let info = EncryptionInfo {
        algorithm: EncryptionAlgorithm::RC4,
        version: 2,
        revision: 3,
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
