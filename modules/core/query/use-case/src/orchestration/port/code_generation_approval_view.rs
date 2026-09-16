//! 対象ごとの「コード生成を開始できるか」の読取り表現。

/// 投影済みの開始可否。Query 側では判断し直さない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeGenerationApprovalView {
    ok: bool,
    reason: String,
    unit: Option<String>,
    contract_hash: Option<String>,
}

impl CodeGenerationApprovalView {
    /// リードモデルの行から組む (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(
        ok: bool,
        reason: String,
        unit: Option<String>,
        contract_hash: Option<String>,
    ) -> CodeGenerationApprovalView {
        CodeGenerationApprovalView {
            ok,
            reason,
            unit,
            contract_hash,
        }
    }

    /// 開始できるか。
    #[must_use]
    pub const fn ok(&self) -> bool {
        self.ok
    }

    /// 判断の理由。拒否だけでなく成功時の名乗りも運ぶ。
    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// 対象 Unit。段階全体は `None`。
    #[must_use]
    pub fn unit(&self) -> Option<&str> {
        self.unit.as_deref()
    }

    /// 承認が束ねたテスト契約の自己ハッシュ。判断が成立しなければ `None`。
    #[must_use]
    pub fn contract_hash(&self) -> Option<&str> {
        self.contract_hash.as_deref()
    }
}
