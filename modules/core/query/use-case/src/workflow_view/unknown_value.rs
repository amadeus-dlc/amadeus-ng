//! `UnknownValue` — 閉集合の語彙が受け付けなかった生値。
//!
//! ビューではなく**拒否**である。どの語彙が拒否したかは呼出側 (アダプタの parse) が
//! 文言に載せるので、ここは生値だけを逐語で運ぶ (`coding-rules/error-handling.md` —
//! エラーは材料のみを持ち、利用者向け文言は出す側が組む)。

use std::fmt;

/// 閉集合外の値。トリム・大文字小文字の正規化はしない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownValue(String);

impl UnknownValue {
    /// 拒否された生値をそのまま包む。
    #[must_use]
    pub fn new(value: impl Into<String>) -> UnknownValue {
        UnknownValue(value.into())
    }

    /// 拒否された生値を逐語で持ち帰る (文言化は出す側の責務)。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for UnknownValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown value {:?}", self.0)
    }
}

impl std::error::Error for UnknownValue {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rejected_value_is_carried_verbatim() {
        let rejected = UnknownValue::new("Initialization");
        assert_eq!(rejected.as_str(), "Initialization");
        assert_eq!(rejected, UnknownValue::new("Initialization"));
        assert_ne!(rejected, UnknownValue::new("initialization"));
    }

    #[test]
    fn the_rendering_carries_material_not_wording() {
        assert_eq!(
            UnknownValue::new("swarm").to_string(),
            "unknown value \"swarm\""
        );
        let boxed: Box<dyn std::error::Error> = Box::new(UnknownValue::new("swarm"));
        assert_eq!(boxed.to_string(), "unknown value \"swarm\"");
    }
}
