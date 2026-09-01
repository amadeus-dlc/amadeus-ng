//! `RunStageDirectiveBuilder` — [`RunStageDirective`] の唯一の組み立て経路。
//!
//! 本型の `build()` だけが [`RunStageDirective`] の構造体リテラルを書く
//! (`coding-rules/factory-naming.md`)。`mut self` を取って `Self` を返すので `with_*` は
//! setter ではなくファクトリメソッドである。
//!
//! 対象型の**子モジュール**に置くのは、private フィールドが「定義モジュールとその子孫」まで
//! 見えるからである — 兄弟ファイルへ出すと、21 フィールドを位置引数で受け渡す基本
//! コンストラクタを別に立てることになり、取り違えを型で防げなくなる。

use super::super::gate_field::GateField;
use super::super::unit_ref::UnitRef;
use super::RunStageDirective;
use crate::orchestration::{PhaseView, ReviewClassView, StageModeView, StageSlugView};

/// [`RunStageDirective`] の組み立て器 — 必須 6 点を基本コンストラクタ相当で受け、残りは
/// `with_*` で伴わせる。`build()` だけが構造体リテラルを書く (factory-naming.md)。
#[derive(Debug, Clone)]
pub struct RunStageDirectiveBuilder {
    stage: StageSlugView,
    phase: PhaseView,
    lead_agent: String,
    mode: StageModeView,
    gate: GateField,
    stage_file: String,
    memory_path: String,
    support_agents: Vec<String>,
    inline_context_paths: Vec<String>,
    consumes: Vec<String>,
    produces: Vec<String>,
    sensors_applicable: Vec<String>,
    next_stage: Option<String>,
    reviewer: Option<String>,
    review_class: Option<ReviewClassView>,
    reviewer_max_iterations: Option<u32>,
    protocol_modules: Vec<String>,
    narration: Option<String>,
    single: bool,
    unit: Option<UnitRef>,
    rules_in_context: Vec<String>,
}

impl RunStageDirectiveBuilder {
    /// 必須材料 (ステージ・フェーズ・リード・モード・ゲート・本体ファイル・日誌) を束ねる。
    #[must_use]
    pub fn new(
        stage: StageSlugView,
        phase: PhaseView,
        lead_agent: impl Into<String>,
        mode: StageModeView,
        gate: GateField,
        stage_file: impl Into<String>,
        memory_path: impl Into<String>,
    ) -> RunStageDirectiveBuilder {
        RunStageDirectiveBuilder {
            stage,
            phase,
            lead_agent: lead_agent.into(),
            mode,
            gate,
            stage_file: stage_file.into(),
            memory_path: memory_path.into(),
            support_agents: Vec::new(),
            inline_context_paths: Vec::new(),
            consumes: Vec::new(),
            produces: Vec::new(),
            sensors_applicable: Vec::new(),
            next_stage: None,
            reviewer: None,
            review_class: None,
            reviewer_max_iterations: None,
            protocol_modules: Vec::new(),
            narration: None,
            single: false,
            unit: None,
            rules_in_context: Vec::new(),
        }
    }

    /// 支援エージェント列を伴う。
    #[must_use]
    pub fn with_support_agents(mut self, agents: Vec<String>) -> RunStageDirectiveBuilder {
        self.support_agents = agents;
        self
    }

    /// 読み込み必須のコンテキストパス列を伴う。
    #[must_use]
    pub fn with_inline_context_paths(mut self, paths: Vec<String>) -> RunStageDirectiveBuilder {
        self.inline_context_paths = paths;
        self
    }

    /// 上流成果物 (consumes) を伴う。
    #[must_use]
    pub fn with_consumes(mut self, consumes: Vec<String>) -> RunStageDirectiveBuilder {
        self.consumes = consumes;
        self
    }

    /// 産出物 (produces) を伴う。
    #[must_use]
    pub fn with_produces(mut self, produces: Vec<String>) -> RunStageDirectiveBuilder {
        self.produces = produces;
        self
    }

    /// 発火センサー列を伴う。
    #[must_use]
    pub fn with_sensors(mut self, sensors: Vec<String>) -> RunStageDirectiveBuilder {
        self.sensors_applicable = sensors;
        self
    }

    /// 次ステージの表示名を伴う。
    #[must_use]
    pub fn with_next_stage(mut self, next_stage: impl Into<String>) -> RunStageDirectiveBuilder {
        self.next_stage = Some(next_stage.into());
        self
    }

    /// レビュアー構成 (名前・クラス・最大反復) を伴う。
    #[must_use]
    pub fn with_reviewer(
        mut self,
        reviewer: impl Into<String>,
        class: ReviewClassView,
        max_iterations: u32,
    ) -> RunStageDirectiveBuilder {
        self.reviewer = Some(reviewer.into());
        self.review_class = Some(class);
        self.reviewer_max_iterations = Some(max_iterations);
        self
    }

    /// プロトコルモジュールのヒント列を伴う。
    #[must_use]
    pub fn with_protocol_modules(mut self, modules: Vec<String>) -> RunStageDirectiveBuilder {
        self.protocol_modules = modules;
        self
    }

    /// ユーザ向けのひとことを伴う。
    #[must_use]
    pub fn with_narration(mut self, narration: impl Into<String>) -> RunStageDirectiveBuilder {
        self.narration = Some(narration.into());
        self
    }

    /// 単一ステージ隔離モード (`--single`) を伴う。
    #[must_use]
    pub const fn with_single(mut self) -> RunStageDirectiveBuilder {
        self.single = true;
        self
    }

    /// per-unit 反復の unit を伴う。
    #[must_use]
    pub fn with_unit(mut self, unit: UnitRef) -> RunStageDirectiveBuilder {
        self.unit = Some(unit);
        self
    }

    /// 配信済みルール束のパス台帳 (`rules_in_context`) を伴う。
    #[must_use]
    pub fn with_rules_in_context(mut self, paths: Vec<String>) -> RunStageDirectiveBuilder {
        self.rules_in_context = paths;
        self
    }

    /// 組み上げる (構造体リテラルはここだけ)。
    #[must_use]
    pub fn build(self) -> RunStageDirective {
        RunStageDirective {
            stage: self.stage,
            phase: self.phase,
            lead_agent: self.lead_agent,
            support_agents: self.support_agents,
            mode: self.mode,
            gate: self.gate,
            stage_file: self.stage_file,
            memory_path: self.memory_path,
            inline_context_paths: self.inline_context_paths,
            consumes: self.consumes,
            produces: self.produces,
            sensors_applicable: self.sensors_applicable,
            next_stage: self.next_stage,
            reviewer: self.reviewer,
            review_class: self.review_class,
            reviewer_max_iterations: self.reviewer_max_iterations,
            protocol_modules: self.protocol_modules,
            narration: self.narration,
            single: self.single,
            unit: self.unit,
            rules_in_context: self.rules_in_context,
        }
    }
}
