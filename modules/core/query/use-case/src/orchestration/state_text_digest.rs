//! 入力境界が計算した公開状態本文のSHA-256。Queryは計算しない。
/// 封緘トークンが持つ本文の照合子。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateTextDigest(String);
impl StateTextDigest {
    /// 小文字64桁の表記だけを受け取る。
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        (value.len() == 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)))
        .then(|| Self(value.to_string()))
    }
    /// 入力境界へ渡す照合値。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
