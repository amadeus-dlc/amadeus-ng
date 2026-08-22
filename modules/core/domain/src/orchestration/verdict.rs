//! `Verdict` — `report --result` の受理 10 語 (02 §7.1)。`approved` / `completed` /
//! `complete` / `done` は**相互交換可能な同義語** (コミットするサブコマンドは呼出側でなく
//! エンジンが選ぶ)。`resume` / `resumed` も同義。未知語は Err (逐語拒否は Presenter 側)。

/// 正規化済み verdict (パースの結果としてのみ得られる)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Verdict {
    /// approved / completed / complete / done の正規化先。
    Forward,
    /// ゲートを開く報告。in-progress のステージだけが開ける
    /// (「only an in-progress stage can open a gate」)。
    AwaitingApproval,
    /// ゲートでの差し戻し。非空のフィードバックを伴い、ステージを revising へ送る。
    Rejected,
    /// 差し戻し後の再入報告。revising のステージだけが再入できる
    /// (「only a revising stage can re-enter its gate」)。
    Revised,
    /// resume / resumed の正規化先。
    Resume,
    /// ルーティングされたライフサイクル結末であって完了ではない
    /// (「a routed lifecycle outcome, not a completion」)。全 completion ガードより先に
    /// 判定される (I13)。
    Skipped,
}

/// 受理 10 語以外の生値 — `parse` の拒否経路 (未知語を既定 verdict へ丸めない)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownVerdict(String);

impl UnknownVerdict {
    /// 拒否された生値をそのまま包む (トリム・大小文字の正規化はしない)。
    #[must_use]
    pub fn new(value: impl Into<String>) -> UnknownVerdict {
        UnknownVerdict(value.into())
    }

    /// 拒否された生値を逐語で持ち帰る (文言化は Presenter 側の責務)。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// 受理語の正準列挙 (エラーメッセージ描画用 — 順序も upstream の提示順)。
pub const ACCEPTED_RESULTS: &[&str] = &[
    "approved",
    "completed",
    "complete",
    "done",
    "awaiting-approval",
    "rejected",
    "revised",
    "resume",
    "resumed",
    "skipped",
];

impl Verdict {
    /// # Errors
    ///
    /// 受理 10 語以外は `UnknownVerdict` で拒否する (逐語の拒否文言は Presenter 側)。
    pub fn parse(s: &str) -> Result<Verdict, UnknownVerdict> {
        Ok(match s {
            "approved" | "completed" | "complete" | "done" => Verdict::Forward,
            "awaiting-approval" => Verdict::AwaitingApproval,
            "rejected" => Verdict::Rejected,
            "revised" => Verdict::Revised,
            "resume" | "resumed" => Verdict::Resume,
            "skipped" => Verdict::Skipped,
            other => return Err(UnknownVerdict::new(other)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_four_forward_synonyms_normalize_to_one_verdict() {
        for s in ["approved", "completed", "complete", "done"] {
            assert_eq!(Verdict::parse(s), Ok(Verdict::Forward));
        }
    }

    #[test]
    fn all_ten_accepted_words_parse_and_unknown_is_rejected() {
        assert_eq!(ACCEPTED_RESULTS.len(), 10);
        for s in ACCEPTED_RESULTS {
            assert!(Verdict::parse(s).is_ok());
        }
        // 受理語以外は生値を逐語で持ち帰る (逐語の拒否文言は Presenter が組む)
        let rejected = Verdict::parse("ok").unwrap_err();
        assert_eq!(rejected.as_str(), "ok");
        assert_eq!(rejected, UnknownVerdict::new("ok"));
    }
}
