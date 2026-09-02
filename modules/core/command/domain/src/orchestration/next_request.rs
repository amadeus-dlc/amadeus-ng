//! `NextRequest` — `next` の状態依存判断に要る観測 (BR3.1 の入力)。
//!
//! 状態**非依存**の分岐 (read-only フラグ、名詞トークン、scope 検証、compose、`--single` 等) は
//! 要求の形だけで決まるコントローラのルーティングであり、集約の判断には来ない (BR3.2)。
//! ここに来るのは「集約の状態を見なければ決まらない」観測だけである。

/// `next_decision` への入力のうち、ワークフロー状態の判断に要る観測。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NextRequest {
    resume: bool,
    reentry: bool,
    free_text: bool,
}

/// 何も観測していない素の要求 (通常のループ 1 周)。
impl Default for NextRequest {
    fn default() -> NextRequest {
        NextRequest::new(false, false, false)
    }
}

impl NextRequest {
    /// 3 観測を束ねる。
    ///
    /// `resume` = `--resume` 指定、`reentry` = `--stage` / `--phase` / `--review` / `--new-intent`
    /// のいずれか (park ガードを外す再入フラグ)、`free_text` = 稼働中に自由記述 prose が来た。
    #[must_use]
    pub const fn new(resume: bool, reentry: bool, free_text: bool) -> NextRequest {
        NextRequest {
            resume,
            reentry,
            free_text,
        }
    }

    /// `--resume` 指定があったか。
    #[must_use]
    pub const fn is_resume(self) -> bool {
        self.resume
    }

    /// 再入フラグがあったか (park ガードを外す)。
    #[must_use]
    pub const fn is_reentry(self) -> bool {
        self.reentry
    }

    /// 稼働中に自由記述が来たか。
    #[must_use]
    pub const fn is_free_text(self) -> bool {
        self.free_text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_request_carries_the_three_state_relevant_observations() {
        let request = NextRequest::new(true, false, true);
        assert!(request.is_resume());
        assert!(!request.is_reentry());
        assert!(request.is_free_text());
    }

    #[test]
    fn a_plain_request_observes_nothing() {
        let request = NextRequest::default();
        assert!(!request.is_resume());
        assert!(!request.is_reentry());
        assert!(!request.is_free_text());
    }
}
