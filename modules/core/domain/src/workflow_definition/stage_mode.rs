//! `StageMode` — ステージのトポロジ。5 値の閉集合 (upstream 01 §3.2)。
//!
//! `agent-team` は**予約値**で出荷グラフには現れない。読み手は明示的に扱い、既定経路へ
//! フォールスルーさせてはならない (レポート §6.1-14 — [S] `02:154`, [S] `04:98`)。

/// ステージ実行トポロジ。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StageMode {
    /// conductor が全ての声を担う。ディスパッチも貢献ファイルも無い (04 §2.3)。
    Inline,
    /// ハブアンドスポーク — lead が起草し、相互に見えない support が各々貢献し、lead が統合する。
    Subagent,
    /// 連鎖 — 各リンクは上流の全作業を見たうえで成果物を直接進める。
    Pipeline,
    /// 相互に発言しあうメッシュ — 記録された異論を伴う。
    Mob,
    /// 予約 — 未実装。`is_reserved()` が真を返す唯一の値。
    AgentTeam,
}

/// 閉集合外の値。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownStageMode(String);

impl UnknownStageMode {
    /// 拒否された生値をそのまま包む (トリム・区切り文字の補正などの正規化はしない)。
    #[must_use]
    pub fn new(value: impl Into<String>) -> UnknownStageMode {
        UnknownStageMode(value.into())
    }

    /// 拒否された生値を逐語で持ち帰る (文言化は Presenter 側の責務)。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl StageMode {
    /// 宣言順の全値。
    pub const ALL: [StageMode; 5] = [
        StageMode::Inline,
        StageMode::Subagent,
        StageMode::Pipeline,
        StageMode::Mob,
        StageMode::AgentTeam,
    ];

    /// # Errors
    ///
    /// 5 値以外は `UnknownStageMode` で拒否する。
    pub fn parse(s: &str) -> Result<StageMode, UnknownStageMode> {
        Ok(match s {
            "inline" => StageMode::Inline,
            "subagent" => StageMode::Subagent,
            "pipeline" => StageMode::Pipeline,
            "mob" => StageMode::Mob,
            "agent-team" => StageMode::AgentTeam,
            other => return Err(UnknownStageMode::new(other)),
        })
    }

    /// ステージ frontmatter / `stage-graph.json` 上の語 (`parse` の逆写像)。
    /// `AgentTeam` は `agent-team` — `_` 区切りは閉集合外である。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            StageMode::Inline => "inline",
            StageMode::Subagent => "subagent",
            StageMode::Pipeline => "pipeline",
            StageMode::Mob => "mob",
            StageMode::AgentTeam => "agent-team",
        }
    }

    /// 予約値か。真なら**ディスパッチしてはならない** — 呼出側は明示的に拒否する
    /// (upstream 最低要件: `throw "mode agent-team not yet implemented"`)。
    #[must_use]
    pub fn is_reserved(self) -> bool {
        self == StageMode::AgentTeam
    }

    /// `support_agents` 非空が compile 時不変条件となるモードか (01 §3.2)。
    /// 読取経路では再検査しない (upstream もロード時は検証しない)。
    #[must_use]
    pub const fn requires_support_agents(self) -> bool {
        matches!(self, StageMode::Pipeline | StageMode::Mob)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn the_five_modes_round_trip_and_unknown_is_rejected() {
        for m in StageMode::ALL {
            assert_eq!(StageMode::parse(m.as_str()).unwrap(), m);
        }
        assert_eq!(
            StageMode::parse("agent-team").unwrap(),
            StageMode::AgentTeam
        );
        // 綴り違い (`-` ではなく `_`) は閉集合外。生値を逐語で持ち帰る
        let rejected = StageMode::parse("agent_team").unwrap_err();
        assert_eq!(rejected.as_str(), "agent_team");
        assert_eq!(rejected, UnknownStageMode::new("agent_team"));
        assert!(StageMode::parse("swarm").is_err());
    }

    #[test]
    fn agent_team_is_the_only_reserved_mode() {
        assert!(StageMode::AgentTeam.is_reserved());
        for m in [
            StageMode::Inline,
            StageMode::Subagent,
            StageMode::Pipeline,
            StageMode::Mob,
        ] {
            assert!(!m.is_reserved());
        }
    }

    #[test]
    fn pipeline_and_mob_are_the_support_agent_modes() {
        assert!(StageMode::Pipeline.requires_support_agents());
        assert!(StageMode::Mob.requires_support_agents());
        assert!(!StageMode::Inline.requires_support_agents());
        assert!(!StageMode::Subagent.requires_support_agents());
        assert!(!StageMode::AgentTeam.requires_support_agents());
    }

    proptest! {
        /// 閉集合外はすべて拒否 — 未知モードが既定経路 (inline) に落ちない。
        #[test]
        fn rejects_anything_outside_the_closed_set(s in "[a-z-]{1,12}") {
            let known = StageMode::ALL.iter().any(|m| m.as_str() == s);
            prop_assert_eq!(StageMode::parse(&s).is_ok(), known);
        }
    }
}
