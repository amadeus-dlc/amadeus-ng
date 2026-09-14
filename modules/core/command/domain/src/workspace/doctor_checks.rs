//! 自己診断の行の一級コレクション — 表示順の行と、その集計 (passed / failed / 終了コード)。
//!
//! 判断 (D1.a〜D5.b を観測からどう評価するか) は [`DoctorChecks::evaluate`] が所有する。
//! 本家 (固定コミット `a277af21` の `aidlc-utility.ts handleDoctor`) に対応する行は
//! ラベル・fix を本家の分岐と同じ綴りで出し、独自の必須診断は固定ラベル + ` — <原因>` で
//! 出す。対象外の検査は行を出さない。表示順は C7 の表の順 (D1.a から) で、フック別の行は
//! 名前順である。

use core_infrastructure::collections::FirstClassCollection;

use super::{DoctorCheck, DoctorObservation};

mod definition_checks;
mod environment_checks;
mod heartbeat_check;
mod hook_checks;
mod record_checks;

/// 表示順に並んだ検査行の集まり (集計はここ 1 箇所)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorChecks {
    checks: Vec<DoctorCheck>,
}

impl DoctorChecks {
    /// 表示順の行から組む (**唯一の構築経路**)。
    #[must_use]
    pub const fn new(checks: Vec<DoctorCheck>) -> Self {
        Self { checks }
    }

    /// 観測を C7 の表の順に評価する。
    ///
    /// 初回状態 (作業記録が無い) では D4 / D5 を非適用として行を出さない
    /// (C7「初回状態・評価不能・警告の区別」)。
    #[must_use]
    pub fn evaluate(observation: &DoctorObservation) -> Self {
        let mut checks: Vec<DoctorCheck> = Vec::new();
        checks.extend(environment_checks::evaluate(observation));
        checks.extend(hook_checks::evaluate(observation.hook_wiring()));
        checks.push(heartbeat_check::evaluate(observation.heartbeat()));
        checks.extend(definition_checks::evaluate(
            observation.shell(),
            observation.definition(),
        ));
        if let Some(record) = observation.record() {
            checks.extend(record_checks::evaluate(record));
        }
        Self::new(checks)
    }

    /// 成功行の数 (実際に出した成功/助言行だけ — C7「出力と終了コード」)。
    #[must_use]
    pub fn passed(&self) -> u64 {
        self.checks.iter().filter(|check| check.is_passed()).count() as u64
    }

    /// 必須失敗行の数。
    #[must_use]
    pub fn failed(&self) -> u64 {
        self.checks
            .iter()
            .filter(|check| !check.is_passed())
            .count() as u64
    }

    /// `failed = 0` なら 0、それ以外は 1。
    #[must_use]
    pub fn exit_code(&self) -> u8 {
        u8::from(self.failed() > 0)
    }

    /// 表示順の行。
    ///
    /// 永続化・投影・表示の境界が行を 1 つずつ写すために列を借用する (一級コレクションの
    /// 基本操作で表せない境界処理 — `coding-rules/first-class-collections.md` の例外)。
    #[must_use]
    pub fn as_slice(&self) -> &[DoctorCheck] {
        &self.checks
    }
}

impl FirstClassCollection for DoctorChecks {
    type Item<'a>
        = &'a DoctorCheck
    where
        Self: 'a;
    type Filtered = Self;

    fn len(&self) -> usize {
        self.checks.len()
    }

    fn at(&self, index: usize) -> Option<&DoctorCheck> {
        self.checks.get(index)
    }

    fn fold_left<'a, A>(&'a self, initial: A, fold: impl FnMut(A, &'a DoctorCheck) -> A) -> A {
        self.checks.iter().fold(initial, fold)
    }

    fn filter(&self, mut predicate: impl FnMut(&DoctorCheck) -> bool) -> Self {
        Self::new(
            self.checks
                .iter()
                .filter(|check| predicate(check))
                .cloned()
                .collect(),
        )
    }
}
