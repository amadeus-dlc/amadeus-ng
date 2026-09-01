//! `CreateIntentError` — `CreateIntentUseCase` の失敗。

use std::fmt;

use core_command_domain::orchestration::{IntentError, IntentExecutionId, IntentId};
use core_command_domain::workflow_definition::WorkflowDefinitionId;

use super::port::RepositoryError;

/// `CreateIntentUseCase` の失敗（材料のみ — 逐語文言は出す側が組む）。
///
/// 4 変種はいずれも**そのまま伝播させるための封筒**である。ユースケースはポートや集約の
/// 拒否を握り潰さないし言い換えもしない（`coding-rules/error-handling.md`）。この動詞は
/// 3 つのポートを順に叩くので、どのポートで倒れたかが変種で分かる必要がある — 失敗の
/// 位置は復旧手順を変えるからである（定義が読めないのはワークスペースの問題、intent の
/// 重複は再実行の問題、実行の書込失敗は intent だけが着地した中途半端な状態）。
// `Clone` / `PartialEq` は実装しない — `Corrupt` の `source`（原因連鎖）が比較・複製不能で
// ある（裁定 6 で受容済み）。テストは `matches!` で判定する。
#[derive(Debug)]
pub enum CreateIntentError {
    /// 定義の取得の失敗（ポートからそのまま伝播）。
    DefinitionRepository(RepositoryError<WorkflowDefinitionId>),
    /// 集約 `Intent` の genesis が拒否した（そのまま伝播 — 未知スコープなど）。
    Intent(IntentError),
    /// intent の永続化の失敗（ポートからそのまま伝播）。
    IntentRepository(RepositoryError<IntentId>),
    /// 実行の永続化の失敗（ポートの失敗 + 孤児 intent の識別子）。
    ///
    /// この変種だけは **intent が既に着地したあと**の失敗である。合成ルートは同じ
    /// `intent_id` で再試行できない（intent の genesis が `Conflict` になる）ので、
    /// 出す側は「intent は作られたが実行が始まっていない」ことを言う必要がある。
    /// そのために**孤児になった intent の識別子を材料として運ぶ**（issue #77 の先行改善 —
    /// 恒久対応は doctor の検出・修復。オーナー裁定 2026-09-01）。
    ExecutionRepository {
        /// 着地済みで、実行の書込が失敗として報告された intent（孤児）。
        ///
        /// ポート契約は Err ⇒ 未永続化を**約束しない**ので、実行行の存否そのものは
        /// ここでは断定できない — 存否の確認と修復は doctor の仕事である（issue #77）。
        orphan: IntentId,
        /// ポートからそのまま伝播した失敗。
        error: RepositoryError<IntentExecutionId>,
    },
}

impl fmt::Display for CreateIntentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CreateIntentError::DefinitionRepository(error) => {
                write!(f, "definition repository: {error}")
            }
            CreateIntentError::Intent(error) => write!(f, "intent: {error}"),
            CreateIntentError::IntentRepository(error) => write!(f, "intent repository: {error}"),
            CreateIntentError::ExecutionRepository { orphan, error } => {
                // 孤児 id は**前置**に置く — 出す側の `chained` は「表示が原因の文言で
                // **終わる**とき」だけ `caused by` の重複を抑止する (ends_with 判定) ので、
                // 接尾辞にすると内側の失敗が二重描画される (PR #87 Bugbot 指摘で実証)。
                write!(f, "execution repository (orphan intent {orphan}): {error}")
            }
        }
    }
}

impl std::error::Error for CreateIntentError {
    /// 内包した失敗へ連鎖する。
    ///
    /// **封筒は連鎖を切ってはならない。** `RepositoryError::Corrupt` は「壊れていた」としか
    /// `Display` に書かず、どの行がどう壊れていたかという実材料は `Error::source` の連鎖に
    /// 載せる（裁定 6 — エラーは契約の一部であり、内部実装がバレる分類を契約に含めない）。
    /// ここで `None` を返すと、その材料はこの型で行き止まりになり、診断には分類だけが残る。
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CreateIntentError::DefinitionRepository(error) => Some(error),
            CreateIntentError::Intent(error) => Some(error),
            CreateIntentError::IntentRepository(error) => Some(error),
            CreateIntentError::ExecutionRepository { error, .. } => Some(error),
        }
    }
}

impl From<RepositoryError<WorkflowDefinitionId>> for CreateIntentError {
    fn from(error: RepositoryError<WorkflowDefinitionId>) -> CreateIntentError {
        CreateIntentError::DefinitionRepository(error)
    }
}

impl From<IntentError> for CreateIntentError {
    fn from(error: IntentError) -> CreateIntentError {
        CreateIntentError::Intent(error)
    }
}

impl From<RepositoryError<IntentId>> for CreateIntentError {
    fn from(error: RepositoryError<IntentId>) -> CreateIntentError {
        CreateIntentError::IntentRepository(error)
    }
}

// `RepositoryError<IntentExecutionId>` からの `From` は**意図的に無い** — この変種は
// 孤児 intent の識別子という文脈材料を要し、ポートの失敗だけからは組めない
// （`CreateIntentUseCase::execute` が store 呼出の場で明示的に包む）。

#[cfg(test)]
mod tests {
    use super::*;

    fn definition_id() -> WorkflowDefinitionId {
        WorkflowDefinitionId::parse("claude").expect("フィクスチャの定義 id")
    }

    #[test]
    fn the_envelope_chains_to_the_material_the_port_hid_in_its_source() {
        // `RepositoryError::Corrupt` は分類 (「壊れていた」) しか `Display` に書かない
        // (裁定 6)。どの行がどう壊れていたかは `source` の連鎖に載るので、封筒がそこで
        // 連鎖を切ると診断には分類しか残らない。
        let error = CreateIntentError::DefinitionRepository(RepositoryError::Corrupt {
            id: definition_id(),
            seq_nr: Some(1),
            source: Box::new(std::io::Error::other("undecodable payload")),
        });

        let port = std::error::Error::source(&error).expect("ポートの失敗へ連鎖する");
        assert_eq!(
            std::error::Error::source(port)
                .expect("ポートは原因へ連鎖する")
                .to_string(),
            "undecodable payload"
        );
    }

    #[test]
    fn a_rejected_genesis_chains_to_the_aggregate_rejection() {
        // 集約の拒否は材料を自分の `Display` に持つ — 連鎖は 1 段で終わる。
        let error = CreateIntentError::Intent(IntentError::Empty);

        let inner = std::error::Error::source(&error).expect("拒否そのものへ連鎖する");
        assert!(!inner.to_string().is_empty(), "材料を語る");
        assert!(std::error::Error::source(inner).is_none(), "その先は無い");
    }
}
