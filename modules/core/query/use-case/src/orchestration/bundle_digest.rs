//! `BundleDigest` — steering 連鎖の束縛ダイジェストの 1 本 (02 §4.4)。
//!
//! **不透明トークン**である: 等値比較だけが契約で、解釈も加工もしない。4 本を別型の newtype に
//! するのは、相互代入・取り違え比較をコンパイルエラーにするためである。値の**計算**は所有する
//! 型の関連メソッドが持ち (`steering_digest` モジュール)、ここは値の型だけを持つ。

/// ルール束ダイジェスト (`b`)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleDigest(String);

impl BundleDigest {
    /// 計算済みの値を包む。
    #[must_use]
    pub fn new(value: impl Into<String>) -> BundleDigest {
        BundleDigest(value.into())
    }

    /// 不透明な値 (ワイヤ・表示用)。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
