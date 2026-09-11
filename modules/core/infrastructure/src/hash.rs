//! バイト列に対する汎用ハッシュ計算。ドメインの語彙や保存形式を知らない。
use sha2::{Digest as _, Sha256};
const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

/// バイト列の sha256 を 64 桁の小文字 16 進で返す。
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex_of(hasher.finalize())
}

/// 読取りを一定サイズに保って、ストリームのSHA-256を計算する。
/// # Errors
/// 読取り元が返したI/Oエラー。
pub fn sha256_read(reader: &mut impl std::io::Read) -> std::io::Result<String> {
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(buffer.split_at(count).0);
    }
    Ok(hex_of(hasher.finalize()))
}

fn hex_of(bytes: impl IntoIterator<Item = u8>) -> String {
    let mut hex = String::with_capacity(64);
    for byte in bytes {
        // ニブル (0..16) の変換なので添字は必ず範囲内 — `unwrap_or` の既定分岐には到達しない。
        hex.push(
            HEX_DIGITS
                .get(usize::from(byte >> 4))
                .copied()
                .unwrap_or(b'0') as char,
        );
        hex.push(
            HEX_DIGITS
                .get(usize::from(byte & 0x0f))
                .copied()
                .unwrap_or(b'0') as char,
        );
    }
    hex
}
