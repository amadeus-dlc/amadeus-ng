//! `JumpView` — `read_next_jump` 1 行の写し (ジャンプ先 1 つの受理判定)。

/// `read_next_jump` の 1 行 (自然キー `execution_id` × `target_index` は UNIQUE 索引)。
///
/// 拒否も 1 つの答えである — `outcome` が方向 (`forward` / `backward` / `redo`) を言うか
/// 拒否を言うかで、プレゼンタが描き分ける。跳べるかどうかの判定はクエリ側に無い。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JumpView {
    target_index: u32,
    target_slug: String,
    outcome: String,
    refusal: Option<String>,
}

impl JumpView {
    /// 4 列をそのまま束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(
        target_index: u32,
        target_slug: String,
        outcome: String,
        refusal: Option<String>,
    ) -> JumpView {
        JumpView {
            target_index,
            target_slug,
            outcome,
            refusal,
        }
    }

    /// ジャンプ先の位置。
    #[must_use]
    pub const fn target_index(&self) -> u32 {
        self.target_index
    }

    /// ジャンプ先の slug。
    #[must_use]
    pub fn target_slug(&self) -> &str {
        &self.target_slug
    }

    /// 受理の答え (方向の綴り、または拒否)。
    #[must_use]
    pub fn outcome(&self) -> &str {
        &self.outcome
    }

    /// 拒否のときだけ在る理由の綴り。
    #[must_use]
    pub fn refusal(&self) -> Option<&str> {
        self.refusal.as_deref()
    }
}
