//! 計画承認の契約解決に使う、階層ごとの原文。
/// コメントを含む入力原文。ファイルの読み取りは入力境界が担う。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestingSections {
    org: String,
    team: String,
    project: String,
}
impl TestingSections {
    /// 三つの階層のTesting Posture節を保持する。
    #[must_use]
    pub const fn new(org: String, team: String, project: String) -> Self {
        Self { org, team, project }
    }
    /// 各規則文書からTesting Postureの節を取り出す。
    #[must_use]
    pub fn from_documents(org: &str, team: &str, project: &str) -> Self {
        Self::new(
            super::testing_posture::extract_section(org),
            super::testing_posture::extract_section(team),
            super::testing_posture::extract_section(project),
        )
    }
    /// 組織の原文。
    #[must_use]
    pub fn org(&self) -> &str {
        &self.org
    }
    /// チームの原文。
    #[must_use]
    pub fn team(&self) -> &str {
        &self.team
    }
    /// プロジェクトの原文。
    #[must_use]
    pub fn project(&self) -> &str {
        &self.project
    }
}
