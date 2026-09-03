//! `JumpPhaseView` — `read_next_jump_phase` 1 行の写し (フェーズごとの目的地)。

/// `read_next_jump_phase` の 1 行 (自然キー `execution_id` × `phase` は UNIQUE 索引)。
///
/// 目的地は**実効プラン**で決まる (recompose のオーバレイが静的グリッドに勝つ) ので、
/// 定義側のフェーズ入口 (`read_definition_scope_phase_entry`) とは答えが違いうる。
/// 目的地を持たないフェーズには行が無いので、引当が `None` になる。
///
/// **受理判定はこの行に無い。** 跳べるかどうかは `read_next_jump` の行が言うので、
/// [`JumpPhaseView::target_index`] を鍵にしてそちらを引く (オーナー裁定 2026-09-03 —
/// 関連行は表ごとに引き、たどるのはユースケースの仕事)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JumpPhaseView {
    target_index: u32,
    target_slug: Option<String>,
}

impl JumpPhaseView {
    /// 2 列をそのまま束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(target_index: u32, target_slug: Option<String>) -> JumpPhaseView {
        JumpPhaseView {
            target_index,
            target_slug,
        }
    }

    /// そのフェーズで最初に実行される in-scope ステージの位置 (受理判定を引く鍵)。
    #[must_use]
    pub const fn target_index(&self) -> u32 {
        self.target_index
    }

    /// 目的地の slug (添字帳を引けなかったときは `None`)。
    #[must_use]
    pub fn target_slug(&self) -> Option<&str> {
        self.target_slug.as_deref()
    }
}
