//! UUIDv7 正準表記の形式検査 — 識別子 Domain Primitive が共有する 1 つの検査点。
//!
//! `IntentId`（intent の識別子）と `IntentExecutionId`（実行の識別子）は同じ形を持つ別々の
//! Domain Primitive である。形の規則 (BR4.1) の正本をこの 1 か所に置き、それぞれの型は
//! 自分の語彙のエラーに写して返す。

/// 正準形の文字数 (`8-4-4-4-12` + ハイフン 4)。
pub(super) const CANONICAL_LEN: usize = 36;
/// `-` が来る 0 始まり位置。
const HYPHEN_POSITIONS: [usize; 4] = [8, 13, 18, 23];
/// version nibble の 0 始まり位置 (16 進 13 桁目)。
const VERSION_POSITION: usize = 14;
/// variant nibble の 0 始まり位置 (16 進 17 桁目)。
const VARIANT_POSITION: usize = 19;
/// UUIDv7 の version nibble。
pub(super) const VERSION_NIBBLE: char = '7';

/// 正準表記に合わない理由 (材料のみ — 利用者向け文言は各識別子型とアダプタ層)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum MalformedUuidV7 {
    /// 前後の空白を除くと空になる。
    Empty,
    /// 正準形の 36 字でない。
    Length {
        /// 実際の文字数 (前後の空白を除いたもの)。
        actual: usize,
    },
    /// ハイフン位置か 16 進小文字の並びが正準形に合わない。位置は 0 始まりの文字位置。
    Format {
        /// 最初に形式へ合わなかった文字の 0 始まり位置。
        position: usize,
    },
    /// version nibble が `7` でない (UUIDv7 以外)。
    Version {
        /// 実際に置かれていた nibble。
        found: char,
    },
    /// variant nibble が RFC の `10xx` (`8` / `9` / `a` / `b`) でない。
    Variant {
        /// 実際に置かれていた nibble。
        found: char,
    },
}

/// 16 進の小文字桁 (`[0-9a-f]`)。大文字は受理しない。
const fn is_lower_hex(c: char) -> bool {
    c.is_ascii_digit() || matches!(c, 'a'..='f')
}

/// RFC の variant nibble (`10xx`)。
const fn is_variant_nibble(c: char) -> bool {
    matches!(c, '8' | '9' | 'a' | 'b')
}

/// 前後の空白を落としてから UUIDv7 の正準表記として検証し、trim 済みの綴りを返す。
///
/// 大文字・短縮形・他 version・記録ディレクトリ名の kebab 表記は受理しない (BR4.1)。
pub(super) fn parse_canonical(s: &str) -> Result<String, MalformedUuidV7> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err(MalformedUuidV7::Empty);
    }
    let actual = trimmed.chars().count();
    if actual != CANONICAL_LEN {
        return Err(MalformedUuidV7::Length { actual });
    }
    for (position, c) in trimmed.chars().enumerate() {
        if HYPHEN_POSITIONS.contains(&position) {
            if c != '-' {
                return Err(MalformedUuidV7::Format { position });
            }
            continue;
        }
        if !is_lower_hex(c) {
            return Err(MalformedUuidV7::Format { position });
        }
        if position == VERSION_POSITION && c != VERSION_NIBBLE {
            return Err(MalformedUuidV7::Version { found: c });
        }
        if position == VARIANT_POSITION && !is_variant_nibble(c) {
            return Err(MalformedUuidV7::Variant { found: c });
        }
    }
    Ok(trimmed.to_string())
}
