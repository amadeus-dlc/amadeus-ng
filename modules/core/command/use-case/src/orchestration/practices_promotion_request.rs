//! `PracticesPromotionRequest` — `PromotePracticesUseCase` の入力（正規化済み）。

use core_command_domain::workspace::PracticesPromotion;

/// `aidlc-state practices-promote` 1 回分の入力。
///
/// 構文段（フラグの有無・ドラフトと正本の存在・contributions の identity marker）と昇格内容の
/// 計算は合成ルートが済ませているので、ここに届くのは**計算済みの値**だけである
/// （`coding-rules/use-case-rules.md` — 入力は型付きの値で受ける）。`--target-dir` は本 build
/// では未配線なので運ばない（設計 §1 の繰延）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PracticesPromotionRequest {
    promotion: PracticesPromotion,
    affirming_user: String,
}

impl PracticesPromotionRequest {
    /// 昇格の内容と承認者を束ねる（**この型の唯一の構築経路**）。
    #[must_use]
    pub fn new(
        promotion: PracticesPromotion,
        affirming_user: impl Into<String>,
    ) -> PracticesPromotionRequest {
        PracticesPromotionRequest {
            promotion,
            affirming_user: affirming_user.into(),
        }
    }

    /// 書き写す内容（置換する節と印付きの規則行）。
    #[must_use]
    pub const fn promotion(&self) -> &PracticesPromotion {
        &self.promotion
    }

    /// 昇格を打った人（upstream の `--affirming-user`、既定は `unknown`）。
    #[must_use]
    pub fn affirming_user(&self) -> &str {
        &self.affirming_user
    }
}
