//! `IntentDirName` — 記録ディレクトリ名 (entities.md IntentDirName、11-workspace §2.2)。

use std::fmt;

/// 日付プレフィクス (`<YYMMDD>`) の桁数。
const DATE_PREFIX_LEN: usize = 6;
/// 日付プレフィクスと slug を隔てる区切り位置 (0 始まり)。
const SEPARATOR_POSITION: usize = DATE_PREFIX_LEN;
/// ディレクトリ名全体の上限文字数。
const MAX_LEN: usize = 64;

/// intent の記録ディレクトリ名 (Always Valid — 不正値はこの型に存在しない)。
///
/// 形は `<YYMMDD>-<slug>` の **kebab 表記** (`^[0-9]{6}-[a-z0-9]+(?:-[a-z0-9]+)*$`、全体 64 字
/// 以下)。衝突サフィックス (`-2` …) は slug の区間として自然に収まる。
///
/// `IntentId` (UUIDv7) とは**別の値**であり、リードモデルの投影先パス解決に使う
/// (11-workspace §2.2、オーナー裁定 2026-08-23)。
///
/// 正規化 (小文字化・空白除去・区切り置換) は一切しない — 受理か拒否のみ。生のまま
/// `join()` に到達してよいパスセグメントであることを型で保証するのが役目だから
/// (`SpaceName` と同じ方針)。
///
/// 予約ラベル (help / list / switch / create / archive / rename / show / birth) の拒否は
/// birth (intent 生成) の責務であり、本型は**形式だけ**を保証する (BR4.2)。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IntentDirName(String);

/// `IntentDirName::parse` が拒否する形 (材料のみ — 利用者向け文言はアダプタ層)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntentDirNameError {
    /// 空文字列。
    Empty,
    /// 64 字を超える。
    Length {
        /// 実際の文字数。
        actual: usize,
    },
    /// 日付プレフィクスの数字・区切り・slug の `[a-z0-9-]` の並びに合わない文字がある。
    Format {
        /// 最初に形式へ合わなかった文字の 0 始まり位置 (末端で尽きた場合はその位置)。
        position: usize,
    },
    /// `-` で区切った区間が空 (`--` の連続、末尾の `-`、slug 不在)。
    EmptySegment {
        /// 空だった区間の 0 始まり位置 (区間 0 は日付プレフィクス)。
        position: usize,
    },
}

impl IntentDirName {
    /// `<YYMMDD>-<slug>` の kebab 表記として検証する。正規化はしない。
    ///
    /// # Errors
    ///
    /// 空・64 字超過・日付プレフィクスや区切りや slug の文字種の違反・空区間
    /// (`--` の連続、末尾 `-`、slug 不在) を、それぞれ拒否する。
    pub fn parse(s: &str) -> Result<IntentDirName, IntentDirNameError> {
        if s.is_empty() {
            return Err(IntentDirNameError::Empty);
        }
        let actual = s.chars().count();
        if actual > MAX_LEN {
            return Err(IntentDirNameError::Length { actual });
        }
        for (position, c) in s.chars().enumerate() {
            let ok = if position < DATE_PREFIX_LEN {
                c.is_ascii_digit()
            } else if position == SEPARATOR_POSITION {
                c == '-'
            } else {
                c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'
            };
            if !ok {
                return Err(IntentDirNameError::Format { position });
            }
        }
        // 区切りまで届かずに尽きた (`260822` / `26082` 等)。
        if actual <= SEPARATOR_POSITION {
            return Err(IntentDirNameError::Format { position: actual });
        }
        for (position, segment) in s.split('-').enumerate() {
            if segment.is_empty() {
                return Err(IntentDirNameError::EmptySegment { position });
            }
        }
        Ok(IntentDirName(s.to_string()))
    }

    /// 検証済みのパスセグメント — `join()` に渡してよい唯一の形。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for IntentDirName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for IntentDirNameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IntentDirNameError::Empty => f.write_str("empty"),
            IntentDirNameError::Length { actual } => {
                write!(f, "length {actual} (maximum {MAX_LEN})")
            }
            IntentDirNameError::Format { position } => {
                write!(f, "invalid character at position {position}")
            }
            IntentDirNameError::EmptySegment { position } => {
                write!(f, "empty segment at position {position}")
            }
        }
    }
}

impl std::error::Error for IntentDirNameError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_accepts_the_record_directory_name() {
        for raw in [
            "260822-stage1-selfhost",
            "260822-a",
            // 衝突サフィックス。
            "260822-stage1-selfhost-2",
            "991231-x9",
        ] {
            let name = IntentDirName::parse(raw).unwrap();
            assert_eq!(name.as_str(), raw);
            assert_eq!(name.to_string(), raw);
        }
    }

    #[test]
    fn an_empty_value_cannot_be_constructed() {
        assert_eq!(IntentDirName::parse(""), Err(IntentDirNameError::Empty));
    }

    #[test]
    fn the_name_must_start_with_a_six_digit_date_and_a_separator() {
        assert_eq!(
            IntentDirName::parse("26082-stage1"),
            Err(IntentDirNameError::Format { position: 5 })
        );
        assert_eq!(
            IntentDirName::parse("abcdef-stage1"),
            Err(IntentDirNameError::Format { position: 0 })
        );
        assert_eq!(
            IntentDirName::parse("2608221-stage1"),
            Err(IntentDirNameError::Format { position: 6 })
        );
        assert_eq!(
            IntentDirName::parse("260822"),
            Err(IntentDirNameError::Format { position: 6 })
        );
    }

    #[test]
    fn a_slug_is_required_after_the_separator() {
        assert_eq!(
            IntentDirName::parse("260822-"),
            Err(IntentDirNameError::EmptySegment { position: 1 })
        );
    }

    #[test]
    fn uppercase_is_rejected() {
        assert_eq!(
            IntentDirName::parse("260822-Stage1"),
            Err(IntentDirNameError::Format { position: 7 })
        );
        assert_eq!(
            IntentDirName::parse("260822-stage1-Selfhost"),
            Err(IntentDirNameError::Format { position: 14 })
        );
    }

    #[test]
    fn other_path_hazards_are_rejected() {
        assert_eq!(
            IntentDirName::parse("260822-a_b"),
            Err(IntentDirNameError::Format { position: 8 })
        );
        assert_eq!(
            IntentDirName::parse("260822-a/b"),
            Err(IntentDirNameError::Format { position: 8 })
        );
        assert_eq!(
            IntentDirName::parse("260822-a b"),
            Err(IntentDirNameError::Format { position: 8 })
        );
    }

    #[test]
    fn a_segment_may_not_be_empty() {
        // 連続ハイフン (kebab の空区間)。
        assert_eq!(
            IntentDirName::parse("260822-a--b"),
            Err(IntentDirNameError::EmptySegment { position: 2 })
        );
        // 末尾ハイフン。
        assert_eq!(
            IntentDirName::parse("260822-a-"),
            Err(IntentDirNameError::EmptySegment { position: 2 })
        );
    }

    #[test]
    fn the_name_may_not_exceed_sixty_four_characters() {
        let slug = "a".repeat(57);
        let at_the_limit = format!("260822-{slug}");
        assert_eq!(at_the_limit.len(), 64);
        assert!(IntentDirName::parse(&at_the_limit).is_ok());

        let over_the_limit = format!("{at_the_limit}a");
        assert_eq!(
            IntentDirName::parse(&over_the_limit),
            Err(IntentDirNameError::Length { actual: 65 })
        );
    }

    #[test]
    fn the_rejection_carries_material_not_wording() {
        assert_eq!(IntentDirNameError::Empty.to_string(), "empty");
        assert_eq!(
            IntentDirNameError::Length { actual: 65 }.to_string(),
            "length 65 (maximum 64)"
        );
        assert_eq!(
            IntentDirNameError::Format { position: 7 }.to_string(),
            "invalid character at position 7"
        );
        assert_eq!(
            IntentDirNameError::EmptySegment { position: 2 }.to_string(),
            "empty segment at position 2"
        );
    }
}
