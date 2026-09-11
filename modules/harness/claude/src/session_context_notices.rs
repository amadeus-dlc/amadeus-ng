//! SessionStartへ渡す判断済みの補足表示材料。
/// SessionStartへ渡す判断済みの補足表示材料。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionContextNotices {
    rebind_offer: String,
    unit_line: String,
    recovery_present: bool,
    uncompiled_stages: Vec<String>,
}
impl SessionContextNotices {
    /// 判断済みの全表示材料から構築する。
    #[must_use]
    pub const fn new(
        rebind_offer: String,
        unit_line: String,
        recovery_present: bool,
        uncompiled_stages: Vec<String>,
    ) -> Self {
        Self {
            rebind_offer,
            unit_line,
            recovery_present,
            uncompiled_stages,
        }
    }
    /// 表示すると決めた再選択案内。
    #[must_use]
    pub fn rebind_offer(&self) -> &str {
        &self.rebind_offer
    }
    /// 表示すると決めたUnit行。
    #[must_use]
    pub fn unit_line(&self) -> &str {
        &self.unit_line
    }
    /// 復旧記録の存在。
    #[must_use]
    pub const fn recovery_present(&self) -> bool {
        self.recovery_present
    }
    /// 未コンパイル工程の表示順一覧。
    #[must_use]
    pub fn uncompiled_stages(&self) -> &[String] {
        &self.uncompiled_stages
    }
}
