//! `PhaseView` — 5 値の閉集合 (upstream 01 §2.1)。リードモデルが載せるフェーズ名の写し。

use super::unknown_value::UnknownValue;

/// ワークフローの 5 フェーズ。宣言順 = 派生 `Ord` 順。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PhaseView {
    /// ブートストラップ専用 — record ツリーのスキャフォールドとワークスペースの分類。
    Initialization,
    /// ソリューション化に先立つ問題フレーミング。
    Ideation,
    /// 仕様化・コンポーネントモデル設計・Unit of Work 分解・配送計画。
    Inception,
    /// Unit ごとの設計とコード生成。
    Construction,
    /// 出荷・観測・対応と、NFR に照らした検証。
    Operation,
}

impl PhaseView {
    /// 宣言順の全値 (フェーズ走査の唯一の正本)。
    pub const ALL: [PhaseView; 5] = [
        PhaseView::Initialization,
        PhaseView::Ideation,
        PhaseView::Inception,
        PhaseView::Construction,
        PhaseView::Operation,
    ];

    /// # Errors
    ///
    /// 5 値以外は [`UnknownValue`] で拒否する (既定経路へフォールスルーさせない)。
    pub fn parse(s: &str) -> Result<PhaseView, UnknownValue> {
        Ok(match s {
            "initialization" => PhaseView::Initialization,
            "ideation" => PhaseView::Ideation,
            "inception" => PhaseView::Inception,
            "construction" => PhaseView::Construction,
            "operation" => PhaseView::Operation,
            other => return Err(UnknownValue::new(other)),
        })
    }

    /// `stage-graph.json` 上の語 (`parse` の逆写像)。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            PhaseView::Initialization => "initialization",
            PhaseView::Ideation => "ideation",
            PhaseView::Inception => "inception",
            PhaseView::Construction => "construction",
            PhaseView::Operation => "operation",
        }
    }

    /// 転置の特例 — initialization の全ステージは全スコープ列で EXECUTE (01 §5.1)。
    #[must_use]
    pub const fn is_always_in_plan(self) -> bool {
        matches!(self, PhaseView::Initialization)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_five_phases_round_trip_and_unknown_is_rejected() {
        for p in PhaseView::ALL {
            assert_eq!(PhaseView::parse(p.as_str()).unwrap(), p);
        }
        let rejected = PhaseView::parse("Initialization").unwrap_err();
        assert_eq!(rejected.as_str(), "Initialization");
        assert!(PhaseView::parse("delivery").is_err());
    }

    #[test]
    fn only_initialization_is_unconditionally_in_plan() {
        assert!(PhaseView::Initialization.is_always_in_plan());
        for p in [
            PhaseView::Ideation,
            PhaseView::Inception,
            PhaseView::Construction,
            PhaseView::Operation,
        ] {
            assert!(!p.is_always_in_plan());
        }
    }
}
