//! `WriteTargets` — 1 回のツール呼出しが書こうとしている宛先の列。
use core_infrastructure::collections::FirstClassCollection;

use crate::workflow_definition::{ReviewPolicy, StageNode};

use super::{ReviewFreezeBlock, WriteTarget};

/// 1 回のツール呼出しが名指した書込み先 (入力順・重複そのまま)。
///
/// **素通しの列**である — 並べ替えも重複除去もしない。どの宛先が先に凍結へ当たるかは
/// ハーネスが並べた順で決まるので、順序を触ると拒否が名指す対象が変わる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteTargets {
    items: Vec<WriteTarget>,
}

impl Default for WriteTargets {
    fn default() -> Self {
        Self::of_items(Default::default())
    }
}
impl WriteTargets {
    // 検査済みの列とその部分列は、この構築口で全状態を初期化する。
    const fn of_items(items: Vec<WriteTarget>) -> Self {
        Self { items }
    }

    /// 書込み先を 1 つも名指さない呼出しの列 (読取り専用ツールなど)。
    #[must_use]
    pub const fn empty() -> WriteTargets {
        WriteTargets::of_items(Vec::new())
    }

    /// 与えられた順序と重複のまま列にする (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(items: Vec<WriteTarget>) -> WriteTargets {
        WriteTargets::of_items(items)
    }

    /// 宛先の件数。
    #[must_use]
    pub const fn len(&self) -> usize {
        self.items.len()
    }

    /// 宛先が 1 つも無いか。
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// 入力順の添字参照。範囲外は `None` (panic しない)。
    #[must_use]
    pub fn at(&self, index: usize) -> Option<&WriteTarget> {
        self.items.get(index)
    }

    /// 入力順に左から畳み込む。空なら初期値を返す。
    pub fn fold_left<'a, A>(&'a self, initial: A, fold: impl FnMut(A, &'a WriteTarget) -> A) -> A {
        self.items.iter().fold(initial, fold)
    }

    /// 条件に一致する宛先を入力順のまま残す。結果は空になり得る。
    #[must_use]
    pub fn filter(&self, mut predicate: impl FnMut(&WriteTarget) -> bool) -> WriteTargets {
        WriteTargets::of_items(
            self.items
                .iter()
                .filter(|target| predicate(target))
                .cloned()
                .collect(),
        )
    }

    /// この列のうち、そのステージの**現在の受領証が覆う**最初の宛先。
    ///
    /// 凍結が名指す材料そのものを返す — 呼出側が宛先と Unit を組み直すと、別の宛先の
    /// Unit を載せた拒否行が書ける。射程の判断は [`ReviewPolicy::receipt_covers`] が持つ
    /// （per-unit ステージの受領証は Unit ごとである）。
    ///
    /// 覆わない宛先で**切り上げない** — upstream は宛先を 1 つずつ判定して最初に拒否へ
    /// 倒れたものを拒否行に載せるので、per-unit ステージではステージ水準の宛先を読み飛ばし、
    /// 後ろに並んだ Unit 宛先で凍結する。覆う宛先が 1 つも無ければ `None`。
    #[must_use]
    pub fn first_frozen_by(
        &self,
        node: &StageNode,
        policy: &ReviewPolicy,
    ) -> Option<ReviewFreezeBlock> {
        self.items.iter().find_map(|target| {
            let artifact = node.reviewed_artifact_target(target);
            policy.receipt_covers(&artifact).then(|| {
                ReviewFreezeBlock::new(
                    target.clone(),
                    node.slug().clone(),
                    artifact.unit().cloned(),
                )
            })
        })
    }
}

impl FirstClassCollection for WriteTargets {
    type Item<'a> = &'a WriteTarget;
    type Filtered = Self;
    fn len(&self) -> usize {
        Self::len(self)
    }
    fn at(&self, index: usize) -> Option<Self::Item<'_>> {
        Self::at(self, index)
    }
    fn fold_left<'a, A>(&'a self, initial: A, fold: impl FnMut(A, Self::Item<'a>) -> A) -> A {
        Self::fold_left(self, initial, fold)
    }
    fn filter(&self, predicate: impl FnMut(Self::Item<'_>) -> bool) -> Self {
        Self::filter(self, predicate)
    }
}
