//! `StateBinding` — state 束縛 (`a` + `h` の畳み込み)。
//!
//! **不透明トークン**である: 等値比較だけが契約で、解釈も加工もしない。値の**計算**は所有する
//! 型の関連メソッドが持ち (`steering_digest` モジュール)、ここは値の型だけを持つ。

/// state 束縛 (`a` + `h` の畳み込み)。
///
/// 「state-aware なのにダイジェストが無い」という不正状態は `Option<StateBinding>` で
/// 表現不能になる — `Some` = 束縛あり (値つき)、`None` = 束縛なし。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateBinding(String);

impl StateBinding {
    /// 計算済みの値を包む。
    #[must_use]
    pub fn new(value: impl Into<String>) -> StateBinding {
        StateBinding(value.into())
    }

    /// 不透明な値 (ワイヤ・表示用)。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
