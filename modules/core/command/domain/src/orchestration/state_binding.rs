//! `StateBinding` — 「この実行状態はまだ動いていないか」の照合子 (continue_token の `h`)。
//!
//! **不透明トークン**である: 等値比較だけが契約で、解釈も加工もしない。値の計算は所有する
//! 集約のクエリ [`IntentExecution::state_binding`](super::IntentExecution::state_binding) が
//! 持ち、RMU がリードモデル (`read_execution.state_binding`) へ投影し、`continue` はトークンの
//! 値と行の値の等値引当だけを行う (再構築も照合ロジックもクエリ側には無い)。

/// 実行状態の束縛ダイジェスト。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StateBinding(String);

impl StateBinding {
    /// 計算済みの値を包む (計算は集約のクエリが行う)。
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
