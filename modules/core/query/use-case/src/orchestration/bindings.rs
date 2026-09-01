//! `Bindings` — steering 連鎖の束縛語彙の対 (4 ダイジェスト — 02 §4.4)。
//!
//! 4 本を別型の newtype で受けるのは、相互代入・取り違え比較をコンパイルエラーにするため
//! である (同型プリミティブの隣接は取り違えの温床)。値の**計算**は所有する型の関連メソッドが
//! 持ち (`steering_digest` モジュール)、ここは対の形だけを持つ。

use super::bundle_digest::BundleDigest;
use super::directive_digest::DirectiveDigest;
use super::route_digest::RouteDigest;
use super::state_binding::StateBinding;

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
