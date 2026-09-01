//! `CompiledDefinitionRepository` ポート — コンパイル済み定義 (配布束) の永続化契約。
//!
//! [`CompiledDefinition`] は同一システムのドメインモデルであり、その読取は Repository の
//! 仕事である (オーナー裁定 2026-09-02 — かつての「外部システムクライアント」分類は棄却済み。
//! #79 §5-g / #80)。媒体 (ハーネス配布の 3 ファイル) は**実装の内部詳細**で、ポート名にも
//! シグネチャにも現れない (`coding-rules/gateway-taxonomy.md` §2)。
//!
//! # 両動詞を持ち、`store` は他リポジトリと同じ (イベント, 集約) の対を受ける
//!
//! Repository は自集約の読み書き両方を所有し (オーナー裁定 2026-09-02 —
//! 「書き込めないと読み込めない」)、`store` の契約は他のリポジトリと同形
//! (`store(&event, &aggregate)`) である。現スコープの実行時の書き手はまだ居ない
//! (compile コンテキスト = slice 2 がこの `store` の呼出側になる) が、読める形は同じ
//! Repository が書ける形でなければならない。
//!
//! # 失敗態度の非対称は実装の挙動 (12 §4)
//!
//! 3 入力で失敗態度が非対称なことは観測可能な契約として実装が維持する (「より厳格にする」
//! 方向の改変も逸脱になる) が、**ポート契約には載せない** — 契約が語るのは
//! [`RepositoryError`] の分類だけである (`coding-rules/error-handling.md`
//! 「Repository エラーはジェネリック 1 本」):
//!
//! - `harness.json` / `stage-graph.json` が読めない・不正 = fatal (`Io` / `Corrupt`)。
//! - `scope-grid.json` の欠損・不正は fatal にしない — グラフの `scopes[]` からの転置導出へ
//!   フォールバックするので、グリッド欠損では失敗しない。
//! - identity ファイルとグリッド列の不一致は双方向とも正当であり、どちらもエラーにしない。

use core_command_domain::workflow_definition::{
    CompiledDefinition, CompiledDefinitionEvent, CompiledDefinitionId,
};

use super::repository_error::RepositoryError;

/// コンパイル済み定義 (配布束) の Repository。
#[allow(
    async_fn_in_trait,
    reason = "Send 境界を意図的に要求しない設計 (C3 / Q3 = A — tokio current_thread)。\
              他の Repository ポートと同じ方針である。"
)]
pub trait CompiledDefinitionRepository {
    /// 識別子でコンパイル済み定義を取得する。
    ///
    /// 呼出側 (合成ルート) は `harness.json` から識別子を先に鋳造しているので、
    /// 鶏と卵にはならない。配布束が別の ID を名乗っていれば `NotFound` である。
    ///
    /// # Errors
    ///
    /// 見つからない・別 ID を名乗っている (`NotFound`)、OS 由来の読取失敗 (`Io`)、
    /// 読めたが内容が壊れている (`Corrupt`)。
    async fn find_by_id(
        &self,
        id: &CompiledDefinitionId,
    ) -> Result<CompiledDefinition, RepositoryError<CompiledDefinitionId>>;

    /// イベントを 1 件と、適用後の集約を永続化する。
    ///
    /// 呼出側は genesis ([`CompiledDefinition::compile`]) が返す対をそのまま渡す —
    /// 他のリポジトリと同じ契約である (`coding-rules/aggregate-commands.md`)。
    ///
    /// # Errors
    ///
    /// イベントと集約の対が食い違う書込契約違反 (`Corrupt`)、OS 由来の書込失敗 (`Io`)、
    /// 内容を永続化表現へ写せない (`Corrupt`) を返す。
    async fn store(
        &mut self,
        event: &CompiledDefinitionEvent,
        compiled_definition: &CompiledDefinition,
    ) -> Result<(), RepositoryError<CompiledDefinitionId>>;
}
