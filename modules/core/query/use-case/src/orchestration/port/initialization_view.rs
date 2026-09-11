//! 作業開始時に投影された事実。現在状態から計算しない。
/// 開始結果の読取りモデル。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitializationView {
    scope: String,
    depth: Option<String>,
    total_stages: u32,
    project_type: String,
    languages: String,
    frameworks: String,
    build_system: String,
    first_stage: Option<String>,
    first_phase: Option<String>,
}
impl InitializationView {
    /// 保存済みの開始結果を束ねる。
    #[expect(
        clippy::too_many_arguments,
        reason = "単一行の写しを完全コンストラクタで受ける"
    )]
    #[must_use]
    pub const fn new(
        scope: String,
        depth: Option<String>,
        total_stages: u32,
        project_type: String,
        languages: String,
        frameworks: String,
        build_system: String,
        first_stage: Option<String>,
        first_phase: Option<String>,
    ) -> Self {
        Self {
            scope,
            depth,
            total_stages,
            project_type,
            languages,
            frameworks,
            build_system,
            first_stage,
            first_phase,
        }
    }
    /// 投影されたscope。
    #[must_use]
    pub fn scope(&self) -> &str {
        &self.scope
    }
    /// 投影されたdepth。
    #[must_use]
    pub fn depth(&self) -> Option<&str> {
        self.depth.as_deref()
    }
    /// 投影されたtotal_stages。
    #[must_use]
    pub const fn total_stages(&self) -> u32 {
        self.total_stages
    }
    /// 投影されたproject_type。
    #[must_use]
    pub fn project_type(&self) -> &str {
        &self.project_type
    }
    /// 投影されたlanguages。
    #[must_use]
    pub fn languages(&self) -> &str {
        &self.languages
    }
    /// 投影されたframeworks。
    #[must_use]
    pub fn frameworks(&self) -> &str {
        &self.frameworks
    }
    /// 投影されたbuild_system。
    #[must_use]
    pub fn build_system(&self) -> &str {
        &self.build_system
    }
    /// 投影されたfirst_stage。
    #[must_use]
    pub fn first_stage(&self) -> Option<&str> {
        self.first_stage.as_deref()
    }
    /// 投影されたfirst_phase。
    #[must_use]
    pub fn first_phase(&self) -> Option<&str> {
        self.first_phase.as_deref()
    }
}
