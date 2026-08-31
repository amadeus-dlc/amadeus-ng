//! steering 連鎖の束縛語彙 — 4 ダイジェストの型付き対 (02 §4.4)。
//!
//! ダイジェストは**不透明トークン**である: 等値比較だけが契約で、解釈も加工もしない。
//! 4 本を別型の newtype にするのは、相互代入・取り違え比較をコンパイルエラーにするため
//! (同型プリミティブの隣接は取り違えの温床)。値の**計算**は所有する型の関連メソッドが持ち
//! (`steering_digest` モジュール)、ここは値の型だけを持つ。

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

/// 届けようとしている run-stage のダイジェスト (`d`)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectiveDigest(String);

impl DirectiveDigest {
    /// 計算済みの値を包む。
    #[must_use]
    pub fn new(value: impl Into<String>) -> DirectiveDigest {
        DirectiveDigest(value.into())
    }

    /// 不透明な値 (ワイヤ・表示用)。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// グラフノードと scope メンバーシップの route ダイジェスト (`r`)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteDigest(String);

impl RouteDigest {
    /// 計算済みの値を包む。
    #[must_use]
    pub fn new(value: impl Into<String>) -> RouteDigest {
        RouteDigest(value.into())
    }

    /// 不透明な値 (ワイヤ・表示用)。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

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

/// 4 ダイジェスト束縛の対 — bundle / directive / route と、任意の state 束縛。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bindings {
    bundle: BundleDigest,
    directive: DirectiveDigest,
    route: RouteDigest,
    state: Option<StateBinding>,
}

impl Bindings {
    /// 束縛 4 点を束ねる (state なしは `None`)。
    #[must_use]
    pub const fn new(
        bundle: BundleDigest,
        directive: DirectiveDigest,
        route: RouteDigest,
        state: Option<StateBinding>,
    ) -> Bindings {
        Bindings {
            bundle,
            directive,
            route,
            state,
        }
    }

    /// ルール束ダイジェスト。
    #[must_use]
    pub const fn bundle(&self) -> &BundleDigest {
        &self.bundle
    }

    /// run-stage ダイジェスト。
    #[must_use]
    pub const fn directive(&self) -> &DirectiveDigest {
        &self.directive
    }

    /// route ダイジェスト。
    #[must_use]
    pub const fn route(&self) -> &RouteDigest {
        &self.route
    }

    /// state 束縛 (無ければ `None`)。
    #[must_use]
    pub const fn state(&self) -> Option<&StateBinding> {
        self.state.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bindings_carry_the_four_typed_digests() {
        let bindings = Bindings::new(
            BundleDigest::new("b"),
            DirectiveDigest::new("d"),
            RouteDigest::new("r"),
            Some(StateBinding::new("h")),
        );
        assert_eq!(bindings.bundle().as_str(), "b");
        assert_eq!(bindings.directive().as_str(), "d");
        assert_eq!(bindings.route().as_str(), "r");
        assert_eq!(bindings.state().map(StateBinding::as_str), Some("h"));
    }

    #[test]
    fn a_stateless_binding_is_represented_by_none() {
        let bindings = Bindings::new(
            BundleDigest::new("b"),
            DirectiveDigest::new("d"),
            RouteDigest::new("r"),
            None,
        );
        assert!(bindings.state().is_none());
        assert_ne!(
            bindings,
            Bindings::new(
                BundleDigest::new("b"),
                DirectiveDigest::new("d"),
                RouteDigest::new("r"),
                Some(StateBinding::new("h")),
            )
        );
    }
}
