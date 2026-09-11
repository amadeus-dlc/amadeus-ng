//! 保存した移動結果の読取り値。
#[derive(Debug, Clone, PartialEq, Eq)]
/// 移動結果の読取り境界。
pub struct JumpResultView {
    payload: String,
}
impl JumpResultView {
    /// 行のJSON列を束ねる。
    #[must_use]
    pub const fn new(payload: String) -> Self {
        Self { payload }
    }
    /// 公開結果のJSON列。
    #[must_use]
    pub fn payload(&self) -> &str {
        &self.payload
    }
}
