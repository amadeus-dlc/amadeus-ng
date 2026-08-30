//! `InMemoryIntentRepository` — `IntentRepository` の揮発実装（結線テスト用）。

use std::collections::HashMap;

use core_command_domain::orchestration::{Intent, IntentId};
use core_command_use_case::orchestration::{IntentRepository, RepositoryError};

/// 保持している intent を識別子で引いて返す揮発の `IntentRepository`。
///
/// **本物の Gateway 実装ではない**ので `Impl` 接尾辞を付けない
/// (`coding-rules/gateway-taxonomy.md`)。実物 — intent 自身のジャーナルから再構成する実装 —
/// は U7 の課題である。現時点で intent の完全な材料が永続化されているのは各実行のジャーナル
/// 先頭の `Started` だけなので、読み先の設計ごと U7 で決める（改訂 10 の申し送り）。
///
/// ここに置く理由は 1 つだけで、**合成ルートが書く結線の形を型として固定する**ことである
/// （`CommitVerdictUseCase` がポート 2 本の注入を要求するようになったため、結線テストが
/// `IntentRepository` の実装を 1 つ必要とする）。
#[derive(Debug, Clone)]
pub struct InMemoryIntentRepository {
    held: HashMap<IntentId, Intent>,
}

impl InMemoryIntentRepository {
    /// 基本コンストラクタ — 中身の写像 (識別子 → intent) をそのまま受け取る
    /// (単一スロット保持は手抜き — オーナー指摘 2026-08-30、issue #54)。
    #[must_use]
    pub const fn new(held: HashMap<IntentId, Intent>) -> InMemoryIntentRepository {
        InMemoryIntentRepository { held }
    }

    /// 1 つの intent を保持する (単発の結線テストの便宜)。
    #[must_use]
    pub fn holding(held: Intent) -> InMemoryIntentRepository {
        let mut map = HashMap::new();
        map.insert(held.id().clone(), held);
        InMemoryIntentRepository::new(map)
    }
}

impl IntentRepository for InMemoryIntentRepository {
    async fn find_by_id(&self, id: &IntentId) -> Result<Intent, RepositoryError<IntentId>> {
        self.held
            .get(id)
            .cloned()
            .ok_or_else(|| RepositoryError::NotFound { id: id.clone() })
    }
}

#[cfg(test)]
mod tests {
    use core_command_domain::orchestration::{
        Created, IntentId, StageDisplay, StageEntry, StartRequest, WorkspaceScan,
    };
    use core_command_domain::workflow_definition::{
        BrownfieldGreenfield, DefinitionRevision, PhaseId, PlanAction, StageNumber, StageSlug,
        WorkflowDefinitionId,
    };

    use super::*;

    fn held() -> Intent {
        Intent::from(Created::new(
            IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").unwrap(),
            WorkflowDefinitionId::parse("claude").unwrap(),
            DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).unwrap(),
            StartRequest::new("classic", "wiring"),
            vec![StageEntry::new(
                StageSlug::parse("state-init").unwrap(),
                PhaseId::Initialization,
                PlanAction::Execute,
                false,
                StageDisplay::new(StageNumber::parse("0.1").unwrap(), "Stage", "orchestrator")
                    .unwrap(),
            )],
            WorkspaceScan::new(
                BrownfieldGreenfield::Greenfield,
                "Unknown",
                "Unknown",
                "Unknown",
            )
            .unwrap(),
        ))
    }

    #[tokio::test]
    async fn the_held_intent_is_returned_by_its_identifier() {
        let repository = InMemoryIntentRepository::holding(held());
        let found = repository.find_by_id(held().id()).await.expect("保持中");
        assert_eq!(found, held());
    }

    #[tokio::test]
    async fn multiple_intents_are_looked_up_by_identifier() {
        // HashMap 内蔵の受入 — 複数保持でも識別子で正しい 1 件が返る (issue #54)。
        let second_id = IntentId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap();
        let second = Intent::from(Created::new(
            second_id.clone(),
            WorkflowDefinitionId::parse("claude").unwrap(),
            DefinitionRevision::parse(&format!("sha256:{}", "1".repeat(64))).unwrap(),
            StartRequest::new("classic", "second"),
            vec![StageEntry::new(
                StageSlug::parse("state-init").unwrap(),
                PhaseId::Initialization,
                PlanAction::Execute,
                false,
                StageDisplay::new(StageNumber::parse("0.1").unwrap(), "Stage", "orchestrator")
                    .unwrap(),
            )],
            WorkspaceScan::new(
                BrownfieldGreenfield::Greenfield,
                "Unknown",
                "Unknown",
                "Unknown",
            )
            .unwrap(),
        ));
        let mut map = std::collections::HashMap::new();
        map.insert(held().id().clone(), held());
        map.insert(second_id.clone(), second.clone());
        let repository = InMemoryIntentRepository::new(map);
        assert_eq!(repository.find_by_id(held().id()).await.unwrap(), held());
        assert_eq!(repository.find_by_id(&second_id).await.unwrap(), second);
    }

    #[tokio::test]
    async fn an_unknown_identifier_is_not_found() {
        let repository = InMemoryIntentRepository::holding(held());
        let absent = IntentId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap();
        let err = repository.find_by_id(&absent).await.unwrap_err();
        assert!(matches!(err, RepositoryError::NotFound { id } if id == absent));
    }
}
