//! `WorkflowDefinitionRepository` ポート — 集約 `WorkflowDefinition` (12-workflow-definition
//! §2.1) の Repository。10-orchestration §3 のポート表に対応し、規範は 12-workflow-definition
//! §4 / §5 が所有する。
//!
//! **コマンド側に残る通常の Repository である** (オーナー裁定 2026-08-30 追補 —
//! `coding-rules/cqrs-boundaries.md`「リポジトリの使い方」)。`find_by_id` で集約を取得し、
//! ビジネスロジックを実行し、更新された集約を `store` で保存する、というのが正しい使い方で
//! ある。ここで `find_by_id` だけを宣言しているのは**書込ユースケースの中の集約再構成**
//! (同規則 5) のためで、読取専用ポートだからではない。`store` は**定義を変更する最初の
//! ユースケースと同じ Bolt で書く** — 呼び手のいない口を先行実装しない
//! (`coding-rules/no-backward-compatibility.md` の同じ精神)。
//!
//! **読むだけの用途はこのポートの仕事ではない。** `next` / `continue` のように定義を読んで
//! 何も書かない動詞はクエリ側が担い、クエリ側は同じ Published Language を**自分の
//! リードモデル読取実装**で読む (`core_query_interface_adapter::WorkflowDefinitionDaoImpl` —
//! クエリ側は読取専用 DAO ポート経由で読む。オーナー裁定 2026-08-31、b27)。
//! 両者は側ごと専用の別実装であり、一方が他方の読取結果を受け取ることはない (同規則 6)。
//! upstream 逐語文言 (12 §4 / §6 の「Stage graph not readable at ...」等) の所有も
//! **クエリ側へ移った** (b26 段階 2。b27 でさらにアダプタからクエリ側ユースケースの
//! `wording` へ移り、ポートは材料だけを運ぶ)。
//!
//! Published Language の 3 入力
//! (`stage-graph.json` / `scope-grid.json` / `<harnessRoot>/scopes/aidlc-<name>.md`) を
//! **1 つの Repository で** 集約 `WorkflowDefinition` に束ねて供給する (compile が graph と
//! grid を lockstep で出すため、片方だけ新しい状態は upstream でも想定外)。
//!
//! 名前は「集約名＋Repository」規則に従う (aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md)。格納形式
//! (`stage-graph.json` というファイル名) は Repository **実装**の内部詳細であり、ポート名に
//! 現れてはならない。
//!
//! # 失敗はジェネリック 1 本 (オーナー裁定 2026-08-31)
//!
//! ポート専用のエラー型 (3 入力の失敗を分類していた 6 変種) は持たない。失敗は
//! [`RepositoryError<WorkflowDefinitionId>`] で表し、**リポジトリにビジネスロジックの
//! エラーを扱わせない** (`coding-rules/error-handling.md`「Repository エラーはジェネリック
//! 1 本」への収束)。どのファイルがどう壊れていたかという文脈はアダプタ私有の型を
//! `Error::source` の連鎖で運び、契約は「壊れていた」としか約束しない (裁定 6 —
//! エラーは契約の一部であり、内部実装がバレる情報を含めない)。
//!
//! **3 入力で失敗態度が非対称なことは実装の挙動として維持される** (12 §4。この非対称そのものが
//! 観測可能な契約で、「より厳格にする」方向の改変も逸脱になる):
//!
//! - `harness.json` が読めない / 不正 JSON / `name` 欠落 = **fatal**。定義 id の供給元であり、
//!   失われると集約に識別子を与えられない (ADR-008)。
//! - `stage-graph.json` が読めない / 不正 JSON = **fatal**。
//! - `scope-grid.json` が読めない / 不正 = **fatal にしない**。グラフの `scopes[]` からの
//!   転置導出へフォールバックする (#3)。したがって `find_by_id` はグリッド欠損では失敗しない。
//! - identity ファイルとグリッド列の不一致は**双方向とも正当** (#5 zero-EXECUTE な正当
//!   スコープ / #6 ランタイム不可視) であり、どちらもエラーにしない。
//! - いずれの失敗でも **stdout に何も書かない** (#10 — half-emitted directive を出さない)。
//!
//! ただしこの非対称の**分類はポート契約に載せない** — どの入力がどう壊れていたかは
//! `Corrupt` の原因連鎖にだけ現れる。
//!
//! 実装は `core-command-interface-adapter`
//! (`orchestration::WorkflowDefinitionRepositoryImpl` が実 I/O、
//! `orchestration::InMemoryWorkflowDefinitionRepository` がテストダブル)。パス解決と env
//! オーバライドの意味論は実装側に閉じる (12 §6)。

use core_command_domain::workflow_definition::{WorkflowDefinition, WorkflowDefinitionId};

use super::repository_error::RepositoryError;

/// 集約 `WorkflowDefinition` の Repository。
pub trait WorkflowDefinitionRepository {
    /// 定義 id で引き、3 入力を読んで集約 `WorkflowDefinition` を組み立てて返す。
    ///
    /// 1 つのハーネスが提供できる定義は 1 つだけなので、この Repository は「id で探す」
    /// のではなく「**要求された id が自分の id か**」を検査する (BR2.6 / ADR-008)。
    /// 一致すれば 3 入力を読み、`id` と内容版 `DefinitionRevision` を載せた集約を返す。
    ///
    /// # Errors
    ///
    /// 要求 id をこのハーネスが提供していない (`NotFound` — 運ぶのは**要求された id** だけ)、
    /// OS 由来の読取失敗 (`Io`)、読めたが内容が壊れている (`Corrupt` — 不正 JSON・identity の
    /// 内容不正・scope frontmatter の検証失敗・ドメイン型への写像失敗。原因は
    /// `Error::source` の連鎖が運ぶ)。
    /// **グリッドの欠損・不正はエラーにしない** — 転置導出へフォールバックする (12 §4 #3)。
    fn find_by_id(
        &self,
        id: &WorkflowDefinitionId,
    ) -> Result<WorkflowDefinition, RepositoryError<WorkflowDefinitionId>>;
}
