//! `ContinueTokenBuilder` — [`ContinueToken`] の唯一の組み立て経路。
//!
//! 必須 5 点 (stage / scope / 部索引 / 束縛 / gate) を受け、残りは `with_*` で伴わせる。
//! `v` は常に現行版である。
//!
//! 対象型の**子モジュール**に置くのは、private フィールドが「定義モジュールとその子孫」まで
//! 見えるからである — 兄弟ファイルへ出すと、全フィールドを位置引数で受け渡す基本
//! コンストラクタを別に立てることになる。

use super::super::bindings::Bindings;
use super::super::gate_field::GateField;
use super::super::part_index::PartIndex;
use super::super::stage_name::StageName;
use super::super::token_version::TokenVersion;
use super::super::unit_ref::UnitRef;
use super::ContinueToken;
use crate::orchestration::{ScopeSlugView, StageSlugView};

/// [`ContinueToken`] の組み立て器 — 必須 5 点を受け、残りは `with_*` で伴わせる。
/// `v` は常に現行版。
#[derive(Debug, Clone)]
pub struct ContinueTokenBuilder {
    token: ContinueToken,
}

impl ContinueTokenBuilder {
    /// 必須材料 (stage / scope / 部索引 / 束縛 / gate) を束ねる。
    #[must_use]
    pub const fn new(
        stage: StageSlugView,
        scope: ScopeSlugView,
        next_part_index: PartIndex,
        bindings: Bindings,
        gate: GateField,
    ) -> ContinueTokenBuilder {
        ContinueTokenBuilder {
            token: ContinueToken {
                version: TokenVersion::CURRENT,
                stage,
                scope,
                next_part_index,
                bindings,
                gate,
                next_stage: None,
                unit: None,
                single: false,
            },
        }
    }

    /// per-unit 反復の unit を伴う (per-unit フラグは unit の有無から導出される)。
    #[must_use]
    pub fn with_unit(mut self, unit: UnitRef) -> ContinueTokenBuilder {
        self.token.unit = Some(unit);
        self
    }

    /// 次ステージの表示名を伴う。
    #[must_use]
    pub fn with_next_stage(mut self, next_stage: StageName) -> ContinueTokenBuilder {
        self.token.next_stage = Some(next_stage);
        self
    }

    /// 単一ステージ隔離モードを伴う。
    #[must_use]
    pub const fn with_single(mut self) -> ContinueTokenBuilder {
        self.token.single = true;
        self
    }

    /// 組み上げる。
    #[must_use]
    pub fn build(self) -> ContinueToken {
        self.token
    }
}
