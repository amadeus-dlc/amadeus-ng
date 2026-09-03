//! `FindJumpUseCase` — ジャンプ先ごとの受理判定を引く。

use crate::orchestration::{JumpDao, JumpPhaseDao, JumpView, ReadModelReadError};

/// ジャンプ先ごとの受理判定を引く。
///
/// slug 指定は `read_next_jump` の 1 引当で足りる。フェーズ指定は 2 引当である —
/// フェーズ表が目的地の**位置**を言い、その位置で受理判定の表を引く (オーナー裁定
/// 2026-09-03 — 関連行は表ごとに引き、FK をたどるのはユースケースの仕事)。
///
/// バインディングはスタティックが既定なので DAO は型パラメータで保持する
/// (`coding-rules/use-case-rules.md` §2)。実装 (`XxxDaoImpl`) には依存しない — 結線は
/// 合成ルートだけが行う (同 §1 の DIP)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindJumpUseCase<J: JumpDao, H: JumpPhaseDao> {
    jumps: J,
    phases: H,
}

impl<J: JumpDao, H: JumpPhaseDao> FindJumpUseCase<J, H> {
    /// 2 つの引当の口を注入する (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(jumps: J, phases: H) -> FindJumpUseCase<J, H> {
        FindJumpUseCase { jumps, phases }
    }

    /// 実行 × ジャンプ先 slug で受理判定を引く。拒否も 1 つの答えとして戻る。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。
    pub fn execute(
        &self,
        execution_id: &str,
        target_slug: &str,
    ) -> Result<Option<JumpView>, ReadModelReadError> {
        self.jumps.find(execution_id, target_slug)
    }

    /// 実行 × フェーズで、実効プランが決めた目的地の受理判定を引く。
    ///
    /// 目的地を持たないフェーズには行が無いので `Ok(None)` になる。目的地が在るのに
    /// その位置の受理判定が引けないのは壊れた投影である — どちらの表もジャンプ 1 回ぶんの
    /// 投影として同じトランザクションで差し替わる。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない、または目的地の受理判定が宙に浮いている
    /// ([`ReadModelReadError`])。
    pub fn execute_phase(
        &self,
        execution_id: &str,
        phase: &str,
    ) -> Result<Option<JumpView>, ReadModelReadError> {
        let Some(target) = self.phases.find(execution_id, phase)? else {
            return Ok(None);
        };
        self.jumps
            .find_by_target(execution_id, target.target_index())?
            .ok_or_else(ReadModelReadError::broken_projection)
            .map(Some)
    }
}
