//! 自己診断の 1 行の DTO — `read_doctor_check` の行の写し。
//!
//! 成否もラベルも修復案も**集約が決めた答え**であり、この型は運ぶだけである。数える・
//! 並べ替える・文言を組む口は持たない (`coding-rules/cqrs-boundaries.md` 規則 6)。

/// `read_doctor_check` の 1 行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorCheck {
    check_id: String,
    passed: bool,
    label: String,
    fix: Option<String>,
}

impl DoctorCheck {
    /// 行を束ねる (**唯一の構築経路**)。
    #[must_use]
    pub const fn new(check_id: String, passed: bool, label: String, fix: Option<String>) -> Self {
        Self {
            check_id,
            passed,
            label,
            fix,
        }
    }

    /// C7 の表の行 ID の綴り (`D1.a` …)。
    #[must_use]
    pub fn check_id(&self) -> &str {
        &self.check_id
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
