//! UUIDv7 正準表記の形式検査 — 識別子 Domain Primitive が共有する 1 つの検査点。
//!
//! 構文解析は uuid クレートに任せる (オーナー裁定 2026-08-30 — UUID の機械的な解析を自作
//! しない)。この層に残る契約は BR4.1 の**正準綴りの逐語検査**だけである — `Uuid::try_parse`
//! は寛容で、大文字・`{braced}`・URN・短縮形も受理するため、再直列化した正準表記と入力の
//! 逐語一致で「正規化せず拒否」を実現する。採番 (生成) はインフラストラクチャ層の責務で、
//! ここには置かない (U7 で composition root にだけ v7 feature を足す)。
//!
//! `IntentId`（intent の識別子）と `IntentExecutionId`（実行の識別子）は同じ形を持つ別々の
//! Domain Primitive である。形の規則 (BR4.1) の正本をこの 1 か所に置き、それぞれの型は
//! 自分の語彙のエラーに写して返す。

use uuid::Uuid;

/// 正準形の文字数 (`8-4-4-4-12` + ハイフン 4)。
pub(super) const CANONICAL_LEN: usize = 36;
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
    /// uuid として解析できない、または解析できても正準綴り (小文字 `8-4-4-4-12`) でない
    /// (大文字・短縮形・`{braced}` など)。
    NotCanonical,
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

/// 指定位置の文字 (長さ検査済みの前提だが、範囲外は `?` で防御)。
fn nibble_at(s: &str, position: usize) -> char {
    s.chars().nth(position).unwrap_or('?')
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
    let Ok(uuid) = Uuid::try_parse(trimmed) else {
        return Err(MalformedUuidV7::NotCanonical);
    };
    // 逐語一致 — 解析器の寛容さ (大文字等) をここで打ち消し、正規化せず拒否する。
    if uuid.as_hyphenated().to_string() != trimmed {
        return Err(MalformedUuidV7::NotCanonical);
    }
    if uuid.get_version_num() != 7 {
        return Err(MalformedUuidV7::Version {
            found: nibble_at(trimmed, VERSION_POSITION),
        });
    }
    if uuid.get_variant() != uuid::Variant::RFC4122 {
        return Err(MalformedUuidV7::Variant {
            found: nibble_at(trimmed, VARIANT_POSITION),
        });
    }
    Ok(trimmed.to_string())
}
