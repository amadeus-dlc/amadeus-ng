//! `PhaseId` — 5 値の閉集合 (upstream 01 §2.1)。`initialization = 0 … operation = 4` の
//! インデックスは `StageNumber` の `<phaseIndex>` と同じ番号体系。

/// ワークフローの 5 フェーズ。宣言順 = `index()` 順 = 派生 `Ord` 順。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PhaseId {
    Initialization,
    Ideation,
    Inception,
    Construction,
    Operation,
}

/// 閉集合外の値。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownPhase(String);

impl UnknownPhase {
    #[must_use]
    pub fn new(value: impl Into<String>) -> UnknownPhase {
        UnknownPhase(value.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl PhaseId {
    /// 宣言順の全値 (フェーズ走査の唯一の正本)。
    pub const ALL: [PhaseId; 5] = [
        PhaseId::Initialization,
        PhaseId::Ideation,
        PhaseId::Inception,
        PhaseId::Construction,
        PhaseId::Operation,
    ];

    /// # Errors
    ///
    /// 5 値以外は `UnknownPhase` で拒否する (既定経路へフォールスルーさせない)。
    pub fn parse(s: &str) -> Result<PhaseId, UnknownPhase> {
        Ok(match s {
            "initialization" => PhaseId::Initialization,
            "ideation" => PhaseId::Ideation,
            "inception" => PhaseId::Inception,
            "construction" => PhaseId::Construction,
            "operation" => PhaseId::Operation,
            other => return Err(UnknownPhase::new(other)),
        })
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            PhaseId::Initialization => "initialization",
            PhaseId::Ideation => "ideation",
            PhaseId::Inception => "inception",
            PhaseId::Construction => "construction",
            PhaseId::Operation => "operation",
        }
    }

    /// `StageNumber` の `<phaseIndex>` に対応する番号。
    #[must_use]
    pub const fn index(self) -> u32 {
        match self {
            PhaseId::Initialization => 0,
            PhaseId::Ideation => 1,
            PhaseId::Inception => 2,
            PhaseId::Construction => 3,
            PhaseId::Operation => 4,
        }
    }

    #[must_use]
    pub const fn from_index(index: u32) -> Option<PhaseId> {
        Some(match index {
            0 => PhaseId::Initialization,
            1 => PhaseId::Ideation,
            2 => PhaseId::Inception,
            3 => PhaseId::Construction,
            4 => PhaseId::Operation,
            _ => return None,
        })
    }

    /// 転置の特例 — initialization の全ステージは全スコープ列で EXECUTE (01 §5.1)。
    #[must_use]
    pub fn is_always_in_plan(self) -> bool {
        self == PhaseId::Initialization
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn the_five_phases_round_trip_and_unknown_is_rejected() {
        for p in PhaseId::ALL {
            assert_eq!(PhaseId::parse(p.as_str()).unwrap(), p);
        }
        // 拒否された生値は逐語で持ち帰る (文言化は Presenter 側の責務)
        let rejected = PhaseId::parse("Initialization").unwrap_err();
        assert_eq!(rejected.as_str(), "Initialization");
        assert_eq!(rejected, UnknownPhase::new("Initialization"));
        assert!(PhaseId::parse("delivery").is_err());
    }

    #[test]
    fn only_initialization_is_unconditionally_in_plan() {
        assert!(PhaseId::Initialization.is_always_in_plan());
        for p in [
            PhaseId::Ideation,
            PhaseId::Inception,
            PhaseId::Construction,
            PhaseId::Operation,
        ] {
            assert!(!p.is_always_in_plan());
        }
    }

    proptest! {
        /// `index` と `from_index` は全域で往復し、`Ord` は index 順と一致する。
        #[test]
        fn index_round_trips_and_orders_like_the_declaration(i in 0u32..5, j in 0u32..5) {
            let a = PhaseId::from_index(i).unwrap();
            let b = PhaseId::from_index(j).unwrap();
            prop_assert_eq!(a.index(), i);
            prop_assert_eq!(a.cmp(&b), i.cmp(&j));
        }

        /// 5 値以外はすべて拒否 (未知値が既定経路に落ちない)。
        #[test]
        fn rejects_anything_outside_the_closed_set(s in "[a-z]{1,12}") {
            let known = PhaseId::ALL.iter().any(|p| p.as_str() == s);
            prop_assert_eq!(PhaseId::parse(&s).is_ok(), known);
        }

        #[test]
        fn from_index_is_none_outside_the_five(i in 5u32..1000) {
            prop_assert!(PhaseId::from_index(i).is_none());
        }
    }
}
