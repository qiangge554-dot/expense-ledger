//! 本地加密模块（T2 依赖）：Argon2id 口令派生 + AES-256-GCM 字段加密
//! 用于「口令锁」打开数据库前的密钥派生，以及敏感字段（如凭证路径）加密存储。
//!
//! 说明：derive_key / random_salt / encrypt / decrypt 目前尚未被命令层调用，
//! 是 T2「敏感字段级加密」的预留 API，故整模块豁免 dead_code 警告。
#![allow(dead_code)]
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::password_hash::{PasswordHash, PasswordHasher, SaltString};
use argon2::PasswordVerifier;
use rand::rngs::OsRng;
use rand::RngCore;

const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;

/// 用口令 + 盐派生 32 字节密钥（Argon2id，直接写入输出缓冲，免 SaltString 转换）
pub fn derive_key(password: &str, salt: &[u8]) -> [u8; KEY_LEN] {
    let argon2 = argon2::Argon2::default();
    let mut out = [0u8; KEY_LEN];
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut out)
        .expect("口令派生密钥失败");
    out
}

/// 生成 16 字节随机盐
pub fn random_salt() -> [u8; 16] {
    let mut s = [0u8; 16];
    OsRng.fill_bytes(&mut s);
    s
}

/// 加密：输出 = nonce(12B) || ciphertext
pub fn encrypt(key: &[u8; KEY_LEN], plaintext: &[u8]) -> Vec<u8> {
    let cipher = Aes256Gcm::new_from_slice(key).expect("密钥长度错误");
    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let mut ct = cipher.encrypt(nonce, plaintext).expect("加密失败");
    let mut out = nonce_bytes.to_vec();
    out.append(&mut ct);
    out
}

/// 解密：输入 = nonce(12B) || ciphertext
pub fn decrypt(key: &[u8; KEY_LEN], data: &[u8]) -> Vec<u8> {
    let cipher = Aes256Gcm::new_from_slice(key).expect("密钥长度错误");
    let (nonce, ct) = data.split_at(NONCE_LEN);
    let nonce = Nonce::from_slice(nonce);
    cipher.decrypt(nonce, ct).expect("解密失败")
}

/// 对口令做 Argon2id 哈希，返回 PHC 格式字符串（可直接存库）
pub fn hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = argon2::Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .expect("口令哈希失败")
        .to_string()
}

/// 验证口令与已存 PHC 哈希是否匹配
pub fn verify_password(password: &str, phc: &str) -> bool {
    match PasswordHash::new(phc) {
        Ok(parsed) => argon2::Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}
