//! directive プロトコル — `DirectiveKind` 10 種の閉集合。
//!
//! directive は `next` / `continue` が放出する**エンジンへの指示書**であり、その判別子は
//! 公開言語 (B14) である。読むだけの動詞が出す**出力モデル**なので、クエリ側が所有する
//! (`coding-rules/cqrs-boundaries.md` 規則 5)。ワイヤ上の綴りは 1 バイトも変えられない。
//! 出典: upstream `aidlc-directive.ts:419-430` (02 §4.1)。

/// 10 種の閉集合。`PresentGate` と `DispatchSubagent` は upstream の placeholder —
/// 「Do not implement those two placeholder behaviours speculatively.」(02 §4.1)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DirectiveKind {
    /// 有効ステージの決定論的ルール束を分割して届ける 1 部。コンダクタは `rules_content` を
    /// 配列順に適用し、ただちに `continue <continue_token>` を実行する。
    LoadSteering,
    /// ステージ本体の実行。ルール／エージェント／`consumes` をロードして本体を走らせ、
    /// `produces` を書き、`memory.md` を保つ。
    RunStage,
    /// **placeholder** — `run-stage` の各フィールドに `worker` を加え、名指しのワーカーへ
    /// `Task` として委譲する構想。エンジンは今日これを構築しない。
    DispatchSubagent,
    /// ビルドバッチのために N ワークツリーへ N 並列ワーカーをファンアウトする。
    InvokeSwarm,
    /// **placeholder** — learnings の儀式を走らせてから承認ゲートを描画する構想。ゲートは
    /// 実際には `run-stage` の 1 フィールドとして運ばれており、この kind は構築されない。
    PresentGate,
    /// 構造化された質問の提示。
    Ask,
    /// 逐語で印字して停止する (status / help / doctor / version)。
    Print,
    /// エラーで停止する。`message` はユーザへ逐語で見せる。
    Error,
    /// ループの停止 (ワークフロー完了、または単一ステージ完了)。
    Done,
    /// ワークフローが途中で意図的に park された。`done` とは別 — park されたワークフローには
    /// スコープ内の未実施ステージが残っている。
    Parked,
}

impl DirectiveKind {
    /// 10 種の全値。並びは upstream `VALID_KINDS` の「エンジン設計のカタログ順」に一致させる
    /// (判別子の allowlist としてこの順が観測可能な契約)。
    pub const ALL: &'static [DirectiveKind] = &[
        DirectiveKind::LoadSteering,
        DirectiveKind::RunStage,
        DirectiveKind::DispatchSubagent,
        DirectiveKind::InvokeSwarm,
        DirectiveKind::PresentGate,
        DirectiveKind::Ask,
        DirectiveKind::Print,
        DirectiveKind::Error,
        DirectiveKind::Done,
        DirectiveKind::Parked,
    ];

    /// ワイヤ上の正準綴り。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            DirectiveKind::LoadSteering => "load-steering",
            DirectiveKind::RunStage => "run-stage",
            DirectiveKind::DispatchSubagent => "dispatch-subagent",
            DirectiveKind::InvokeSwarm => "invoke-swarm",
            DirectiveKind::PresentGate => "present-gate",
            DirectiveKind::Ask => "ask",
            DirectiveKind::Print => "print",
            DirectiveKind::Error => "error",
            DirectiveKind::Done => "done",
            DirectiveKind::Parked => "parked",
        }
    }

    /// ワイヤ上の綴りから閉集合へ引き当てる。`None` は upstream の
    /// `unknown kind: "<k>"` に相当する拒否 (既定経路へフォールスルーさせない)。
    #[must_use]
    pub fn parse(s: &str) -> Option<DirectiveKind> {
        Self::ALL.iter().copied().find(|k| k.as_str() == s)
    }

    /// upstream が「投機的に実装するな」と明示した 2 種 (`PresentGate` /
    /// `DispatchSubagent`) か。エンジンが構築してよい kind の判定に使う。
    #[must_use]
    pub const fn is_placeholder(self) -> bool {
        matches!(
            self,
            DirectiveKind::PresentGate | DirectiveKind::DispatchSubagent
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ten_kinds_round_trip_and_unknown_is_rejected() {
        assert_eq!(DirectiveKind::ALL.len(), 10);
        for k in DirectiveKind::ALL {
            assert_eq!(DirectiveKind::parse(k.as_str()), Some(*k));
        }
        assert_eq!(DirectiveKind::parse("gate"), None);
    }

    #[test]
    fn exactly_the_two_placeholders_are_marked() {
        let placeholders: Vec<_> = DirectiveKind::ALL
            .iter()
            .filter(|k| k.is_placeholder())
            .collect();
        assert_eq!(placeholders.len(), 2);
    }
}
