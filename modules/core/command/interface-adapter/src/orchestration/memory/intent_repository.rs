//! `InMemoryIntentRepository` — `IntentRepository` の揮発実装（結線テスト用）。

use core_command_domain::orchestration::{Intent, IntentId};
use core_command_use_case::orchestration::{IntentRepository, IntentRepositoryError};

/// 保持している intent をそのまま返す揮発の `IntentRepository`。
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
    held: Intent,
}

impl InMemoryIntentRepository {
    /// 1 つの intent を保持する。
    #[must_use]
    pub const fn holding(held: Intent) -> InMemoryIntentRepository {
        InMemoryIntentRepository { held }
    }
}

impl IntentRepository for InMemoryIntentRepository {
    async fn find_by_id(&self, id: &IntentId) -> Result<Intent, IntentRepositoryError> {
        if self.held.id() == id {
            Ok(self.held.clone())
        } else {
            Err(IntentRepositoryError::NotFound {
                intent_id: id.clone(),
            })
        }
    }
}
