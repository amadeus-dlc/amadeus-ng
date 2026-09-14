//! 自己診断の 1 行 — ID・成否・ラベル・修復案または原因。

use super::DoctorCheckId;

/// 描画される 1 行。成功行の `fix` は表示されないが、材料として保持する。
///
/// ラベルと修復案は本家 (固定コミット `a277af21` の `handleDoctor`) の分岐と同じ綴りか、
/// 独自診断の固定ラベル + 原因である。どの綴りを選ぶかは判断であり、集約の側にある。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorCheck {
    id: DoctorCheckId,
    passed: bool,
    label: String,
    fix: Option<String>,
}

impl DoctorCheck {
    /// 行を束ねる (**唯一の構築経路**)。
    #[must_use]
    pub const fn new(id: DoctorCheckId, passed: bool, label: String, fix: Option<String>) -> Self {
        Self {
            id,
            passed,
            label,
            fix,
        }
    }

    /// 成功行 (修復案なし)。
    #[must_use]
    pub const fn passed(id: DoctorCheckId, label: String) -> Self {
        Self::new(id, true, label, None)
    }

    /// 必須失敗行 (修復案または原因つき)。
    #[must_use]
    pub const fn failed(id: DoctorCheckId, label: String, fix: String) -> Self {
        Self::new(id, false, label, Some(fix))
    }

    /// C7 の行 ID。
    #[must_use]
    pub const fn id(&self) -> DoctorCheckId {
        self.id
    }

    /// 成功 (評価済み助言を含む) か。
    #[must_use]
    pub const fn is_passed(&self) -> bool {
        self.passed
    }

    /// 表示ラベル。
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// 修復案または原因 (失敗行だけが描画する)。
    #[must_use]
    pub fn fix(&self) -> Option<&str> {
        self.fix.as_deref()
    }
}
