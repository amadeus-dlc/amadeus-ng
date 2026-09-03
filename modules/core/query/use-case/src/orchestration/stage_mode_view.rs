//! `StageModeView` — ステージのトポロジ。5 値の閉集合 (upstream 01 §3.2)。
//!
//! `agent-team` は**予約値**で出荷グラフには現れない。読み手は明示的に扱い、既定経路へ
//! フォールスルーさせてはならない。

use super::unknown_value::UnknownValue;

/// ステージ実行トポロジ。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StageModeView {
    /// conductor が全ての声を担う。ディスパッチも貢献ファイルも無い。
    Inline,
    /// ハブアンドスポーク — lead が起草し、support が各々貢献し、lead が統合する。
    Subagent,
    /// 連鎖 — 各リンクは上流の全作業を見たうえで成果物を直接進める。
    Pipeline,
    /// 相互に発言しあうメッシュ — 記録された異論を伴う。
    Mob,
    /// 予約 — 未実装。`is_reserved()` が真を返す唯一の値。
    AgentTeam,
}

impl StageModeView {
    /// 宣言順の全値。
    pub const ALL: [StageModeView; 5] = [
        StageModeView::Inline,
        StageModeView::Subagent,
        StageModeView::Pipeline,
        StageModeView::Mob,
        StageModeView::AgentTeam,
    ];

    /// # Errors
    ///
    /// 5 値以外は [`UnknownValue`] で拒否する。
    pub fn parse(s: &str) -> Result<StageModeView, UnknownValue> {
        Ok(match s {
            "inline" => StageModeView::Inline,
            "subagent" => StageModeView::Subagent,
            "pipeline" => StageModeView::Pipeline,
            "mob" => StageModeView::Mob,
            "agent-team" => StageModeView::AgentTeam,
            other => return Err(UnknownValue::new(other)),
        })
    }

    /// `stage-graph.json` 上の語 (`parse` の逆写像)。`AgentTeam` は `agent-team` —
    /// `_` 区切りは閉集合外である。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            StageModeView::Inline => "inline",
            StageModeView::Subagent => "subagent",
            StageModeView::Pipeline => "pipeline",
            StageModeView::Mob => "mob",
            StageModeView::AgentTeam => "agent-team",
        }
    }

    /// 予約値か。真なら**ディスパッチしてはならない** — 呼出側は明示的に拒否する。
    #[must_use]
    pub const fn is_reserved(self) -> bool {
        matches!(self, StageModeView::AgentTeam)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_five_modes_round_trip_and_the_spelling_is_exact() {
        for m in StageModeView::ALL {
            assert_eq!(StageModeView::parse(m.as_str()).unwrap(), m);
        }
        assert_eq!(
            StageModeView::parse("agent-team").unwrap(),
            StageModeView::AgentTeam
        );
        // `-` ではなく `_` の綴りは閉集合外。
        let rejected = StageModeView::parse("agent_team").unwrap_err();
        assert_eq!(rejected.as_str(), "agent_team");
        assert!(StageModeView::parse("swarm").is_err());
    }

    #[test]
    fn agent_team_is_the_only_reserved_mode() {
        assert!(StageModeView::AgentTeam.is_reserved());
        for m in [
            StageModeView::Inline,
            StageModeView::Subagent,
            StageModeView::Pipeline,
            StageModeView::Mob,
        ] {
            assert!(!m.is_reserved());
        }
    }
}
