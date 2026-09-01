//! `AuditFieldKey` — 監査ブロックのフィールドキー (11-workspace §2.2 / W9)。
//!
//! 監査ブロックの**描画**は投影の責務であり、ここには無い (§2.3)。ここに置くのは
//! 「行を偽造できない」ことを型で保証する Always Valid のキーである — 描き手がどれだけ雑に
//! 書いても第二の `**Event**:` 行が出せない、という性質はドメインに残る
//! (§2.3「行終端エスケープによる行偽造不能性」)。

use std::fmt;

use super::audit_field_key_error::AuditFieldKeyError;

/// フィールドキーの文法 (upstream `AUDIT_FIELD_KEY_PATTERN` — 先頭は英字、以降は英数と
/// ` ._()/-` の 7 記号)。正規表現は使わずコードポイント走査で判定する。
const KEY_TAIL_SYMBOLS: [char; 7] = [' ', '.', '_', '(', ')', '/', '-'];

/// 描き手が自分で書く 2 つのキーの片方 (upstream `EMITTER_OWNED_FIELD_KEYS`)。もう一方の
/// `Timestamp` は `AuditFields` が破棄するので、その定数はコレクション側が持つ。
const EVENT_KEY: &str = "Event";

/// 監査ブロックのフィールドキー (Always Valid — `parse` 以外では作れない)。
///
/// `Event` を拒むのは行偽造の防止である。呼出側供給の `Event` は**第二の**`**Event**:` 行
/// として着地し、監査ブロックのどの行にも一致する読み手 (`findAllEvents` の複数行正規表現)
/// から見ると、無害なイベント型に密輸したイベントが全クエリで本物として登録されてしまう。
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AuditFieldKey(String);

impl AuditFieldKey {
    /// 文法検査つきの唯一の構成関数。
    ///
    /// # Errors
    ///
    /// 文法外 (`Malformed`)、描き手が所有するキー `Event` (`EmitterOwned`) を返す。
    pub fn parse(raw: &str) -> Result<AuditFieldKey, AuditFieldKeyError> {
        if raw == EVENT_KEY {
            return Err(AuditFieldKeyError::EmitterOwned {
                key: raw.to_string(),
            });
        }
        let mut chars = raw.chars();
        let head_is_letter = chars.next().is_some_and(|c| c.is_ascii_alphabetic());
        let tail_is_legal =
            chars.all(|c| c.is_ascii_alphanumeric() || KEY_TAIL_SYMBOLS.contains(&c));
        if head_is_letter && tail_is_legal {
            Ok(AuditFieldKey(raw.to_string()))
        } else {
            Err(AuditFieldKeyError::Malformed {
                key: raw.to_string(),
            })
        }
    }

    /// `**<key>**:` に書かれる綴り。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AuditFieldKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_key_grammar_accepts_letters_digits_and_the_seven_symbols() {
        for raw in [
            "Stage",
            "Revision count",
            "User Input",
            "Fire id",
            "a",
            "A1._()/-",
            "Some.Key (with) slash/and-dash",
        ] {
            assert!(AuditFieldKey::parse(raw).is_ok(), "受理されるべき: {raw}");
        }
    }

    #[test]
    fn the_key_grammar_refuses_anything_outside_it() {
        for raw in [
            "",
            "1Stage",
            " Stage",
            "-Stage",
            "Stage:",
            "Stage*",
            "ステージ",
        ] {
            assert_eq!(
                AuditFieldKey::parse(raw),
                Err(AuditFieldKeyError::Malformed {
                    key: raw.to_string()
                }),
                "拒否されるべき: {raw}"
            );
        }
    }

    #[test]
    fn the_event_key_is_refused_because_a_second_event_line_forges_events() {
        assert_eq!(
            AuditFieldKey::parse("Event"),
            Err(AuditFieldKeyError::EmitterOwned {
                key: "Event".to_string()
            })
        );
        // 綴りが違えば別のキーである (前方一致では拒まない)。
        assert!(AuditFieldKey::parse("Event Kind").is_ok());
    }

    #[test]
    fn the_key_rejections_carry_material_not_wording() {
        assert_eq!(
            AuditFieldKey::parse("1x").unwrap_err().to_string(),
            "malformed audit field key: 1x"
        );
        assert_eq!(
            AuditFieldKey::parse("Event").unwrap_err().to_string(),
            "emitter-owned audit field key: Event"
        );
        let boxed: Box<dyn std::error::Error> =
            Box::new(AuditFieldKey::parse("Event").unwrap_err());
        assert_eq!(boxed.to_string(), "emitter-owned audit field key: Event");
    }
}
