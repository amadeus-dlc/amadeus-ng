//! **構造化投影核** — 読取コマンドが 1 回の引当で答えを得るための `read_*` 表 (系統 (2))。
//!
//! # 何をする層か
//!
//! ジャーナルの全履歴からコマンド側の集約を `replay` で起こし、**集約のクエリメソッドを
//! 呼んだ答えをそのまま行に写す**。判断はすべて集約に在り、この層が持つのは「どのキーで
//! どのクエリを呼ぶか」の列挙だけである (`coding-rules/cqrs-boundaries.md` 規則 3 の
//! 2026-09-02 追記 — 投影核は集約を `replay` で起こしてクエリメソッドを呼ぶ)。
//!
//! これは Markdown 面 (系統 (1) — [`crate::workspace`] の `aidlc-state.md` と監査シャード)
//! とは別の面である。系統 (1) は人と upstream ツールがそのまま読むファイルで、系統 (2) は
//! CLI の読取コマンド (`next` / `continue` / 将来の `--status` / `doctor`) が読む
//! **非正規化リードモデル**である。クエリ側が系統 (1) を逆パースして自分で計算することは
//! 禁じられている (b26 / b27 の誤りの是正)。
//!
//! # 全再計算・全差し替え
//!
//! 投影は差分ではなく**全履歴からの再計算**であり、書込は全行の差し替えである (裁定 §5)。
//! 壁時計を読まないので、同じ履歴からは何度走らせても同じ行になる。ジャーナルは 1
//! ワークスペース分で小さく、定義イベントも数版しか無いので、増分化は必要になった時点で
//! 行う。
//!
//! # 純粋である
//!
//! [`ReadTables::project`] はストレージを知らない — 接続もチェックポイントも引数に
//! 現れない。行を SQLite へ落とすのは取得ループの仕事であり、行の差し替えとチェック
//! ポイントの前進は 1 トランザクションで行う (裁定 §3)。二層を潰してはならない。
//!
//! 型ファイルの mod は private。公開 API は以下の `pub use` が唯一の宣言であり、
//! 消費側のパスは `core_read_model_updater::read_tables::<型>` で安定する
//! (aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/module-visibility.md)。

use std::collections::BTreeMap;

use core_command_domain::orchestration::{IntentExecution, IntentExecutionEvent};
use core_command_domain::workflow_definition::{
    PhaseId, PlanAction, WorkflowDefinition, WorkflowDefinitionEvent,
};

use crate::orchestration::{DefinitionEntry, GlobalSeqNr, JournalBatch, JournalEntry};

mod definition_row;
mod definition_scope_keyword_row;
mod definition_scope_phase_entry_row;
mod definition_scope_row;
mod definition_scope_stage_row;
mod definition_stage_row;
mod execution_row;
mod execution_stage_row;
mod intent_row;
mod intent_stage_row;
mod json_column;
mod next_answer_row;
mod next_jump_phase_row;
mod next_jump_row;
mod read_tables_error;
mod request_kind;
mod spelling;
mod sql;
mod stage_lookup;

pub use definition_row::DefinitionRow;
pub use definition_scope_keyword_row::DefinitionScopeKeywordRow;
pub use definition_scope_phase_entry_row::DefinitionScopePhaseEntryRow;
pub use definition_scope_row::DefinitionScopeRow;
pub use definition_scope_stage_row::DefinitionScopeStageRow;
pub use definition_stage_row::DefinitionStageRow;
pub use execution_row::ExecutionRow;
pub use execution_stage_row::ExecutionStageRow;
pub use intent_row::IntentRow;
pub use intent_stage_row::IntentStageRow;
pub use next_answer_row::NextAnswerRow;
pub use next_jump_phase_row::NextJumpPhaseRow;
pub use next_jump_row::NextJumpRow;
pub use read_tables_error::ReadTablesError;
pub use request_kind::RequestKind;

// 表の DDL と全差し替えは取得ループ (`JournalReaderImpl`) だけが呼ぶ内部の口である。
pub(crate) use sql::{ensure_tables, replace_all};

/// 1 回の投影で作った `read_*` 表の全行。
///
/// フィールドは private。行の並びは決定的である — 定義と実行は識別子の辞書順、その中は
/// 文書順・キー順であり、同じ履歴からは同じ順序の行が出る。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadTables {
    definitions: Vec<DefinitionRow>,
    definition_stages: Vec<DefinitionStageRow>,
    definition_scopes: Vec<DefinitionScopeRow>,
    definition_scope_keywords: Vec<DefinitionScopeKeywordRow>,
    definition_scope_stages: Vec<DefinitionScopeStageRow>,
    definition_scope_phase_entries: Vec<DefinitionScopePhaseEntryRow>,
    intents: Vec<IntentRow>,
    intent_stages: Vec<IntentStageRow>,
    executions: Vec<ExecutionRow>,
    execution_stages: Vec<ExecutionStageRow>,
    next_answers: Vec<NextAnswerRow>,
    next_jumps: Vec<NextJumpRow>,
    next_jump_phases: Vec<NextJumpPhaseRow>,
    as_of: Option<GlobalSeqNr>,
}

impl ReadTables {
    /// 全履歴を `read_*` 表の行へ投影する (**この型の唯一の構築経路**)。
    ///
    /// 手順は 4 段である。
    ///
    /// 1. 定義の行を系譜 ID ごとに束ね、誕生記録 (`Defined`) を種に `replay` する。
    /// 2. intent は誕生記録から再構成済みの集約がバッチに載っているのでそのまま使う。
    /// 3. 実行の行を実行 ID ごとに束ね、誕生記録 (`Started`) を種に `replay` する。
    /// 4. 起こした集約のクエリを呼んで行を作る。
    ///
    /// # Errors
    ///
    /// ストリームの先頭が誕生記録でない (`MissingGenesis`)、実行が指す intent が履歴に
    /// 無い (`IntentUnavailable`)。どちらも歴史が切り落とされた兆候であり、部分的な行を
    /// 書かずに止める。
    ///
    /// # Panics
    ///
    /// 集約の再構成が壊れた歴史を踏んだとき (通番の飛び・不変条件違反)。再構成は失敗を
    /// 返さずクラッシュするのがキャノンである (オーナー裁定 2026-08-30)。
    pub fn project(history: &JournalBatch) -> Result<ReadTables, ReadTablesError> {
        let mut definitions = Vec::new();
        let mut definition_stages = Vec::new();
        let mut definition_scopes = Vec::new();
        let mut definition_scope_keywords = Vec::new();
        let mut definition_scope_stages = Vec::new();
        let mut definition_scope_phase_entries = Vec::new();

        for definition in replay_definitions(history)? {
            let id = definition.id();
            definitions.push(DefinitionRow::of(&definition));
            for (position, node) in definition.graph().nodes().iter().enumerate() {
                definition_stages.push(DefinitionStageRow::of(id, position, node));
            }
            // 語 → スコープの逆引き。`scopes()` は辞書順なので `or_insert` が「辞書順の
            // 先着」になる (選択ではなく決定的な畳み込み)。
            let mut first_scope_of_keyword: BTreeMap<&str, &str> = BTreeMap::new();
            for (scope, metadata) in definition.scopes() {
                definition_scopes.push(DefinitionScopeRow::of(&definition, scope, metadata));
                for keyword in metadata.keywords() {
                    first_scope_of_keyword
                        .entry(keyword.as_str())
                        .or_insert(scope.as_str());
                }
                let mut in_scope_order = 0_usize;
                for (slug, _, action) in definition.stages_in_scope(scope) {
                    let order = if action == Some(PlanAction::Execute) {
                        let current = in_scope_order;
                        in_scope_order += 1;
                        Some(current)
                    } else {
                        None
                    };
                    definition_scope_stages
                        .push(DefinitionScopeStageRow::of(id, scope, slug, action, order));
                }
                for phase in phases() {
                    if let Some(first) = definition.first_in_scope_stage_of_phase(phase, scope) {
                        definition_scope_phase_entries
                            .push(DefinitionScopePhaseEntryRow::of(id, scope, phase, first));
                    }
                }
            }
            for (keyword, scope) in first_scope_of_keyword {
                definition_scope_keywords.push(DefinitionScopeKeywordRow::of(id, keyword, scope));
            }
        }

        let mut intents = Vec::new();
        let mut intent_stages = Vec::new();
        for intent in history.intents() {
            intents.push(IntentRow::of(intent));
            for (index, entry) in intent.stages().iter().enumerate() {
                intent_stages.push(IntentStageRow::of(intent.id(), index, entry));
            }
        }

        let mut executions = Vec::new();
        let mut execution_stages = Vec::new();
        let mut next_answers = Vec::new();
        let mut next_jumps = Vec::new();
        let mut next_jump_phases = Vec::new();
        for execution in replay_executions(history)? {
            let intent = history
                .intents()
                .iter()
                .find(|intent| intent.id() == execution.intent_id())
                .ok_or_else(|| ReadTablesError::IntentUnavailable {
                    execution_id: execution.id().as_str().to_string(),
                    intent_id: execution.intent_id().as_str().to_string(),
                })?;
            executions.push(ExecutionRow::of(&execution));
            for (index, key) in execution.stage_keys().iter().enumerate() {
                let Some(stage) = execution.stage_index(index) else {
                    continue;
                };
                execution_stages.push(ExecutionStageRow::of(&execution, intent, stage, key));
                next_jumps.push(NextJumpRow::of(&execution, intent, stage, key));
            }
            for kind in RequestKind::ALL {
                next_answers.push(NextAnswerRow::of(&execution, kind));
            }
            for phase in phases() {
                if let Some(target) = execution.first_in_scope_of_phase(phase) {
                    next_jump_phases.push(NextJumpPhaseRow::of(&execution, phase, target));
                }
            }
        }

        Ok(ReadTables {
            definitions,
            definition_stages,
            definition_scopes,
            definition_scope_keywords,
            definition_scope_stages,
            definition_scope_phase_entries,
            intents,
            intent_stages,
            executions,
            execution_stages,
            next_answers,
            next_jumps,
            next_jump_phases,
            as_of: history.scanned_to(),
        })
    }

    /// 走査済み最終位置 (どこまでの歴史を映した行か)。`None` = 1 行も無かった履歴。
    #[must_use]
    pub const fn as_of(&self) -> Option<GlobalSeqNr> {
        self.as_of
    }

    /// `read_definition` の行。
    #[must_use]
    pub fn definitions(&self) -> &[DefinitionRow] {
        &self.definitions
    }

    /// `read_definition_stage` の行。
    #[must_use]
    pub fn definition_stages(&self) -> &[DefinitionStageRow] {
        &self.definition_stages
    }

    /// `read_definition_scope` の行。
    #[must_use]
    pub fn definition_scopes(&self) -> &[DefinitionScopeRow] {
        &self.definition_scopes
    }

    /// `read_definition_scope_keyword` の行。
    #[must_use]
    pub fn definition_scope_keywords(&self) -> &[DefinitionScopeKeywordRow] {
        &self.definition_scope_keywords
    }

    /// `read_definition_scope_stage` の行。
    #[must_use]
    pub fn definition_scope_stages(&self) -> &[DefinitionScopeStageRow] {
        &self.definition_scope_stages
    }

    /// `read_definition_scope_phase_entry` の行。
    #[must_use]
    pub fn definition_scope_phase_entries(&self) -> &[DefinitionScopePhaseEntryRow] {
        &self.definition_scope_phase_entries
    }

    /// `read_intent` の行。
    #[must_use]
    pub fn intents(&self) -> &[IntentRow] {
        &self.intents
    }

    /// `read_intent_stage` の行。
    #[must_use]
    pub fn intent_stages(&self) -> &[IntentStageRow] {
        &self.intent_stages
    }

    /// `read_execution` の行。
    #[must_use]
    pub fn executions(&self) -> &[ExecutionRow] {
        &self.executions
    }

    /// `read_execution_stage` の行。
    #[must_use]
    pub fn execution_stages(&self) -> &[ExecutionStageRow] {
        &self.execution_stages
    }

    /// `read_next_answer` の行。
    #[must_use]
    pub fn next_answers(&self) -> &[NextAnswerRow] {
        &self.next_answers
    }

    /// `read_next_jump` の行。
    #[must_use]
    pub fn next_jumps(&self) -> &[NextJumpRow] {
        &self.next_jumps
    }

    /// `read_next_jump_phase` の行。
    #[must_use]
    pub fn next_jump_phases(&self) -> &[NextJumpPhaseRow] {
        &self.next_jump_phases
    }
}

/// フェーズの全列挙 (番号順)。フェーズ横断の表はこの順で行を並べる。
fn phases() -> impl Iterator<Item = PhaseId> {
    (0..=4_u32).filter_map(PhaseId::from_index)
}

/// 定義ストリームを系譜 ID ごとに束ねて再生する (系譜 ID の辞書順)。
fn replay_definitions(history: &JournalBatch) -> Result<Vec<WorkflowDefinition>, ReadTablesError> {
    let mut streams: BTreeMap<&str, Vec<&DefinitionEntry>> = BTreeMap::new();
    for entry in history.definitions() {
        streams
            .entry(entry.definition_id().as_str())
            .or_default()
            .push(entry);
    }
    let mut replayed = Vec::new();
    for (id, entries) in streams {
        let Some((genesis, rest)) = entries.split_first() else {
            continue;
        };
        let WorkflowDefinitionEvent::Defined(defined) = genesis.event() else {
            return Err(ReadTablesError::MissingGenesis {
                aggregate_id: id.to_string(),
            });
        };
        let snapshot = WorkflowDefinition::from((defined.clone(), *genesis.occurred_at()));
        replayed.push(WorkflowDefinition::replay(
            snapshot,
            rest.iter()
                .map(|entry| (entry.seq_nr(), *entry.occurred_at(), entry.event().clone())),
        ));
    }
    Ok(replayed)
}

/// 実行ストリームを実行 ID ごとに束ねて再生する (実行 ID の辞書順)。
fn replay_executions(history: &JournalBatch) -> Result<Vec<IntentExecution>, ReadTablesError> {
    let mut streams: BTreeMap<&str, Vec<&JournalEntry>> = BTreeMap::new();
    for entry in history.executions() {
        streams
            .entry(entry.execution_id().as_str())
            .or_default()
            .push(entry);
    }
    let mut replayed = Vec::new();
    for (id, entries) in streams {
        let Some((genesis, rest)) = entries.split_first() else {
            continue;
        };
        let IntentExecutionEvent::Started(started) = genesis.event() else {
            return Err(ReadTablesError::MissingGenesis {
                aggregate_id: id.to_string(),
            });
        };
        let snapshot = IntentExecution::from((started.clone(), *genesis.occurred_at()));
        replayed.push(IntentExecution::replay(
            snapshot,
            rest.iter()
                .map(|entry| (entry.seq_nr(), *entry.occurred_at(), entry.event().clone())),
        ));
    }
    Ok(replayed)
}
