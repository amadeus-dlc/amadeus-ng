//! `DefineWorkflowError` — `DefineWorkflowUseCase` の失敗。

use std::fmt;

use core_command_domain::workflow_definition::{
    CompiledDefinitionId, LineageMismatch, RedefineError, WorkflowDefinitionId,
};

use super::port::RepositoryError;

/// `DefineWorkflowUseCase` の失敗 (材料のみ — 逐語文言は出す側が組む)。
///
/// 4 変種はいずれも**そのまま伝播させるための封筒**である。ユースケースはポートや集約の
/// 拒否を握り潰さないし言い換えもしない (`coding-rules/error-handling.md`)。失敗の位置は
/// 復旧手順を変えるので変種で分かる必要がある — コンパイル済み定義 (配布束) が読めないのは
/// ハーネス配置の問題、ジャーナル側の失敗は永続化の問題である (集約ごとに自前の ID 型を
/// 持つので、変種はエラーの ID 型でも区別される)。
///
/// **「内容が変わっていない」はここに現れない。** それは失敗ではなく取込が冪等であること
/// の帰結であり、ユースケースが `Ok` へ畳む ([`DefineWorkflowUseCase`] の doc を参照)。
///
/// `Clone` / `PartialEq` は実装しない — `Corrupt` の `source` (原因連鎖) が比較・複製不能で
/// ある (裁定 6 で受容済み)。テストは `matches!` で判定する。
///
/// [`DefineWorkflowUseCase`]: super::define_workflow_use_case::DefineWorkflowUseCase
#[derive(Debug)]
pub enum DefineWorkflowError {
    /// コンパイル済み定義 (配布束) の取得の失敗 (ポートからそのまま伝播)。
    CompiledDefinitionRepository(RepositoryError<CompiledDefinitionId>),
    /// ジャーナル側の定義の取得ないし永続化の失敗 (ポートからそのまま伝播)。
    DefinitionRepository(RepositoryError<WorkflowDefinitionId>),
    /// 集約が確立を拒否した (そのまま伝播 — 配布束の系譜が違う)。
    Define(LineageMismatch),
    /// 集約が改訂を拒否した (そのまま伝播 — 系譜違い・通番の枯渇)。
    Redefine(RedefineError),
}

impl fmt::Display for DefineWorkflowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DefineWorkflowError::CompiledDefinitionRepository(error) => {
                write!(f, "compiled definition repository: {error}")
            }
            DefineWorkflowError::DefinitionRepository(error) => {
                write!(f, "definition repository: {error}")
            }
            DefineWorkflowError::Define(error) => write!(f, "define: {error}"),
            DefineWorkflowError::Redefine(error) => write!(f, "redefine: {error}"),
        }
    }
}

impl std::error::Error for DefineWorkflowError {
    /// 内包した失敗へ連鎖する。
    ///
    /// **封筒は連鎖を切ってはならない。**
    /// `RepositoryError::Corrupt` は「壊れていた」としか `Display` に書かず、どのファイルが
    /// どう壊れていたかという実材料は `Error::source` の連鎖に載せる (裁定 6 — エラーは契約の
    /// 一部であり、内部実装がバレる分類を契約に含めない)。ここで `None` を返すと、その材料は
    /// この型で行き止まりになり、診断には分類だけが残る。
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DefineWorkflowError::CompiledDefinitionRepository(error) => Some(error),
            DefineWorkflowError::DefinitionRepository(error) => Some(error),
            DefineWorkflowError::Define(error) => Some(error),
            DefineWorkflowError::Redefine(error) => Some(error),
        }
    }
}

// `RepositoryError<WorkflowDefinitionId>` からの `From` は DefinitionRepository へだけ
// 写す — 同じ型を運ぶ変種が 2 つある (CompiledDefinitionRepository / DefinitionRepository)
// ため、配布束側は `execute` が呼出の場で明示的に包む。

impl From<RepositoryError<WorkflowDefinitionId>> for DefineWorkflowError {
    fn from(error: RepositoryError<WorkflowDefinitionId>) -> DefineWorkflowError {
        DefineWorkflowError::DefinitionRepository(error)
    }
}

impl From<LineageMismatch> for DefineWorkflowError {
    fn from(error: LineageMismatch) -> DefineWorkflowError {
        DefineWorkflowError::Define(error)
    }
}

impl From<RedefineError> for DefineWorkflowError {
    fn from(error: RedefineError) -> DefineWorkflowError {
        DefineWorkflowError::Redefine(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::error::Error as _;

    use core_command_domain::workflow_definition::DefinitionRevision;

    fn definition_id() -> WorkflowDefinitionId {
        WorkflowDefinitionId::parse("claude").expect("フィクスチャの定義 id")
    }

    fn compiled_definition_id() -> CompiledDefinitionId {
        CompiledDefinitionId::parse("claude").expect("フィクスチャの配布束 id")
    }

    fn boxed(message: &str) -> Box<dyn std::error::Error + Send + Sync> {
        Box::new(std::io::Error::other(message.to_string()))
    }

    /// 連鎖の末端の文言 (診断が最後に見せるべき実材料)。
    fn terminal(error: &dyn std::error::Error) -> String {
        let mut last = error.to_string();
        let mut source = error.source();
        while let Some(cause) = source {
            last = cause.to_string();
            source = cause.source();
        }
        last
    }

    #[test]
    fn the_envelope_chains_through_to_the_material_at_the_end() {
        // 封筒が `source` を返さないと、`Corrupt` が連鎖に載せた実材料 (どのファイルが
        // どう壊れていたか) がこの型で行き止まりになる。診断はそこから先を辿れなくなり、
        // 利用者には分類 (「壊れていた」) しか届かない。
        let compiled =
            DefineWorkflowError::CompiledDefinitionRepository(RepositoryError::Corrupt {
                id: compiled_definition_id(),
                seq_nr: None,
                source: boxed(
                    "stage graph at /w/stage-graph.json is not valid JSON: expected value",
                ),
            });
        assert_eq!(
            terminal(&compiled),
            "stage graph at /w/stage-graph.json is not valid JSON: expected value"
        );
        // 1 段目は包んだポートのエラーそのものである (構造を偽らない — 末端へ飛ばさない)。
        assert!(
            compiled
                .source()
                .expect("配布束の取得の失敗へ連鎖する")
                .to_string()
                .starts_with("corrupt"),
        );

        let repository = DefineWorkflowError::DefinitionRepository(RepositoryError::Corrupt {
            id: definition_id(),
            seq_nr: Some(2),
            source: boxed("undecodable payload"),
        });
        assert_eq!(terminal(&repository), "undecodable payload");
    }

    #[test]
    fn a_lineage_refusal_converts_into_the_define_variant_and_chains_to_it() {
        let error: DefineWorkflowError =
            LineageMismatch::new(definition_id(), compiled_definition_id()).into();
        assert!(matches!(error, DefineWorkflowError::Define(_)));
        assert_eq!(
            error.to_string(),
            "define: lineage mismatch: definition claude was handed the bundle claude"
        );
        assert!(
            error
                .source()
                .expect("拒否そのものへ連鎖する")
                .to_string()
                .starts_with("lineage mismatch")
        );
    }

    #[test]
    fn a_redefine_refusal_converts_into_its_own_variant() {
        // `?` で伝播させる経路 — 変種の取り違えが起きないことを固定する。
        let error: DefineWorkflowError = RedefineError::SequenceExhausted.into();
        assert!(matches!(
            error,
            DefineWorkflowError::Redefine(RedefineError::SequenceExhausted)
        ));
    }

    #[test]
    fn a_rejection_carries_its_material_in_its_own_wording() {
        // 集約の拒否は材料を自分の `Display` に持つ — その先に連鎖は無い。辿る側が
        // 末端で止まれることを固定する。
        let revision =
            DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).expect("revision");
        let rejected = DefineWorkflowError::Redefine(RedefineError::Unchanged { revision });

        let inner = rejected.source().expect("拒否そのものへ連鎖する");
        assert!(inner.to_string().starts_with("definition unchanged at "));
        assert!(inner.source().is_none(), "その先は無い");
    }
}
