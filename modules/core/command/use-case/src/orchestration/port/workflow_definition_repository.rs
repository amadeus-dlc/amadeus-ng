//! `WorkflowDefinitionRepository` ポート — 集約 `WorkflowDefinition` (12-workflow-definition
//! §2.1) の Repository。10-orchestration §3 のポート表に対応する。
//!
//! **コマンド側に残る通常の Repository である** (オーナー裁定 2026-08-30 追補 —
//! `coding-rules/cqrs-boundaries.md`「リポジトリの使い方」)。`find_by_id` で集約を取得し、
//! ビジネスロジックを実行し、更新された集約を `store` で保存する、というのが正しい使い方で
//! ある。両動詞が揃ったのは 2026-08-31 で、追補裁定「`store` は定義を変更する最初の
//! ユースケースと同じ Bolt で書く」の条件が `DefineWorkflowUseCase` の登場で満たされた。
//!
//! # 格納先はイベントストアである (2026-08-31 オーナー裁定)
//!
//! 「リポジトリの実装は `EventStoreForSqlite` を使わないといけない」。**このポートの実装が
//! dist の 3 入力をファイルから読んで集約を組み立てることは無い** — それは
//! `coding-rules/cqrs-boundaries.md` 規則 4 (コマンド側の最新状態は常に集約から。リードモデルは
//! 遅延するので物理的に読めない) への正面違反であり、2026-08-31 に破棄された。
//!
//! 3 入力を読むのは配布束の Repository ([`CompiledDefinitionRepository`]) であり、読んだ内容から定義を
//! 確立・改訂して**このポートへ書く**のが `DefineWorkflowUseCase` である。以後の読取は
//! 常にジャーナル + スナップショットからの再構成になる。
//!
//! **読むだけの用途はこのポートの仕事ではない。** `next` / `continue` のように定義を読んで
//! 何も書かない動詞はクエリ側が担い、クエリ側は Published Language を**自分の
//! リードモデル読取実装**で読む (`core_query_interface_adapter::WorkflowDefinitionDaoImpl` —
//! オーナー裁定 2026-08-31、b27)。両者は側ごと専用の別実装であり、一方が他方の読取結果を
//! 受け取ることはない (同規則 6)。
//!
//! 名前は「集約名＋Repository」規則に従う (aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md)。格納形式
//! (イベントストアであること) は Repository **実装**の内部詳細であり、ポート名に現れない。
//!
//! # 失敗はジェネリック 1 本 (オーナー裁定 2026-08-31)
//!
//! ポート専用のエラー型は持たない。失敗は [`RepositoryError<WorkflowDefinitionId>`] で表し、
//! **リポジトリにビジネスロジックのエラーを扱わせない** (`coding-rules/error-handling.md`)。
//! 何がどう壊れていたかという文脈はアダプタ私有の型を `Error::source` の連鎖で運び、契約は
//! 「壊れていた」としか約束しない (裁定 6)。
//!
//! 実装は `core-command-interface-adapter`
//! (`orchestration::WorkflowDefinitionRepositoryImpl` — SQLite / memory のどちらのストアを
//! 内包しても手順は同一である)。
//!
//! [`CompiledDefinitionRepository`]: super::compiled_definition_repository::CompiledDefinitionRepository

use core_command_domain::orchestration::Intent;
use core_command_domain::workflow_definition::{
    WorkflowDefinition, WorkflowDefinitionEvent, WorkflowDefinitionId,
};

use super::repository_error::RepositoryError;

/// 集約 `WorkflowDefinition` の Repository (イベントソーシング形 — ADR-010)。
///
/// 自集約の ID による取得に加え、intent が参照する定義の取得を提供する。
/// 関連 ID の解決はアダプタが担い、再構成・永続化する対象は常に定義だけである。
/// 取得後のレビュー方針などの業務判断はドメインが担う。
///
/// レシーバは CQS に従う (`coding-rules/command-query-separation.md`) — 読取は `&self`、
/// 永続化は `&mut self`。
#[allow(
    async_fn_in_trait,
    reason = "Send 境界を意図的に要求しない設計 (C3 / Q3 = A — tokio current_thread)。\
              `IntentRepository` / `IntentExecutionRepository` と同じ方針である。"
)]
pub trait WorkflowDefinitionRepository {
    /// 定義を再構成して返す。
    ///
    /// 1 つのハーネスが提供する定義は 1 つだけだが、それは**ストアに何が書かれているか**で
    /// 決まる — 要求 id のストリームが無ければ `NotFound` である (BR2.6 / ADR-008)。
    ///
    /// 返るのは Always Valid な [`WorkflowDefinition`] である — 実装は復号したあと必ず
    /// 検査付き再構成経路 (誕生記録の変換 + [`WorkflowDefinition::replay`]) を通す
    /// (`coding-rules/domain-persistence-neutrality.md`)。
    ///
    /// # Errors
    ///
    /// 定義がまだ確立されていない (`NotFound`)、ストア I/O (`Io`)、ストアの記録の破損
    /// (`Corrupt` — 原因は `source` 連鎖) を返す。
    async fn find_by_id(
        &self,
        id: &WorkflowDefinitionId,
    ) -> Result<WorkflowDefinition, RepositoryError<WorkflowDefinitionId>>;

    /// intent が参照する系譜の最新定義を、その定義のストリームから再構成して返す。
    ///
    /// 関連 ID の読取はアダプタが担い、既存の [`Self::find_by_id`] へ委譲する。
    /// intent の作成時点の内容版への巻戻しや、別の読取モデルへの参照は行わない。
    ///
    /// # Errors
    ///
    /// [`Self::find_by_id`] と同じ失敗を返す。`NotFound` / `Corrupt` の ID は参照先の
    /// [`WorkflowDefinitionId`] である。
    async fn find_for_intent(
        &self,
        intent: &Intent,
    ) -> Result<WorkflowDefinition, RepositoryError<WorkflowDefinitionId>>;

    /// イベントを 1 件と、適用後の集約を永続化する。
    ///
    /// 呼出側は [`WorkflowDefinition::define`] が返す対、または
    /// [`WorkflowDefinition::redefine`] が返したイベントと改訂後の集約をそのまま渡す —
    /// いつスナップショットを書くかは実装の内部政策である (オーナー裁定 2026-08-30)。
    ///
    /// 発生時刻は**引数で受けない**。集約が `last_updated_at` として運んでくるので、封筒の
    /// `occurred_at` はそこから組む
    /// ([`IntentExecutionRepository::store`](super::intent_execution_repository::IntentExecutionRepository::store)
    /// と同じ形 — オーナー裁定 2026-08-31「手本と対にせよ」)。時刻そのものは `define` /
    /// `redefine` の引数として合成ルートの clock から集約へ入る。
    ///
    /// # Errors
    ///
    /// 楽観 version の不一致 (`Conflict` — 別プロセスが先に改訂した)、ストア I/O (`Io`)、
    /// 書込契約の違反 (`Corrupt`) を返す。
    async fn store(
        &mut self,
        event: &WorkflowDefinitionEvent,
        definition: &WorkflowDefinition,
    ) -> Result<(), RepositoryError<WorkflowDefinitionId>>;
}
