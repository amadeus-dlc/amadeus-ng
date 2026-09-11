//! set_*関連関数・メソッドの禁止。可視性・テスト配置による除外やallow抑制をしない。
use crate::check::Finding;
use syn::visit::Visit;
/// 安定したルール識別子。
pub(crate) const RULE: &str = "setter-method";
/// impl/traitのソース上のメソッドを検査する。macro展開は行わない。
pub(crate) fn check(file: &syn::File) -> Vec<Finding> {
    let mut visitor = SetterVisitor {
        findings: Vec::new(),
    };
    visitor.visit_file(file);
    visitor.findings
}
const HELP: &str = "Repair: 1. 完成型は完全コンストラクタで初期化する。2. 実行時変更は不変条件を守るドメインの振る舞い・保存済みイベント適用にし、setterを単に改名しない。with_*はファクトリとして維持する。 Scope: 指摘されたimpl/traitの定義・呼出し・対応テスト。private/関連関数/trait/testも対象。自由関数・文字列・未展開macroは対象外。 Rationale: 初期化漏れと、DDDの文脈を無視した任意の状態設定を防ぐ。";
struct SetterVisitor {
    findings: Vec<Finding>,
}
impl SetterVisitor {
    fn inspect(&mut self, signature: &syn::Signature) {
        let name = signature.ident.to_string();
        let name = name.strip_prefix("r#").unwrap_or(&name);
        if name.starts_with("set_") {
            self.findings.push(Finding::new(RULE,signature.ident.span().start().line,format!("Problem: impl/traitの `{name}` は禁止されたset_*メソッド名。通常のallowコメントでは抑制しない。"),HELP));
        }
    }
}
impl<'ast> Visit<'ast> for SetterVisitor {
    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        self.inspect(&node.sig);
        syn::visit::visit_impl_item_fn(self, node);
    }
    fn visit_trait_item_fn(&mut self, node: &'ast syn::TraitItemFn) {
        self.inspect(&node.sig);
        syn::visit::visit_trait_item_fn(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::RULE;
    fn findings(path: &str, source: &str) -> Vec<crate::check::Finding> {
        crate::check::check_source(path, source)
            .unwrap()
            .into_iter()
            .filter(|finding| finding.rule == RULE)
            .collect()
    }
    #[test]
    fn detects_impl_methods_regardless_of_visibility_receiver_or_modifiers() {
        let source = r#"
impl Example {
    fn set_private(&mut self, value: u8) {}
    pub fn set_public(self, value: u8) -> Self { self }
    pub(crate) const fn set_associated() -> u8 { 0 }
    async fn set_async(&self) {}
    fn r#set_state(&mut self) {}
}
"#;
        let found = findings("modules/app/example.rs", source);
        assert_eq!(found.len(), 5);
        assert_eq!(
            found.iter().map(|finding| finding.line).collect::<Vec<_>>(),
            vec![3, 4, 5, 6, 7]
        );
        assert!(found.last().unwrap().message.contains("`set_state`"));
    }
    #[test]
    fn detects_trait_declarations_defaults_and_implementations() {
        let source = r#"
trait Example {
    fn set_required(&mut self);
    fn set_default(&mut self) {}
}
impl Example for Target {
    fn set_required(&mut self) {}
}
"#;
        assert_eq!(
            findings("modules/core/command/domain/src/example.rs", source).len(),
            3
        );
    }
    #[test]
    fn does_not_exempt_test_paths_or_cfg_test_methods() {
        let source = r#"
#[cfg(test)]
mod tests {
    impl Example {
        #[cfg(test)]
        fn set_state(&mut self) {}
    }
}
"#;
        for path in [
            "modules/app/tests/fixture.rs",
            "modules/app/src/example.rs",
            "modules\\app\\tests\\fixture.rs",
        ] {
            assert_eq!(findings(path, source).len(), 1, "{path}");
        }
    }
    #[test]
    fn distinguishes_factories_functions_strings_and_unexpanded_macros() {
        let source = r##"
fn set_global() {}
#[test] fn set_fixture_name() {}
impl Example {
    fn with_value(mut self, value:u8)->Self { self.value=value;self }
    fn setup(&mut self) {}
    fn asset_value(&mut self) {}
}
const TEXT:&str = "impl Example { fn set_value(&mut self) {} }";
// impl Example { fn set_comment(&mut self) {} }
macro_rules! emitted { () => { impl Example { fn set_generated(&mut self) {} } }; }
"##;
        assert!(findings("modules/app/src/example.rs", source).is_empty());
    }
    #[test]
    fn suppression_comments_do_not_disable_this_rule() {
        let source = r#"
impl Example {
    // amadeus-lint: allow(setter-method) — test helper
    fn set_state(&mut self) {}
}
"#;
        let found = findings("modules/app/src/example.rs", source);
        assert_eq!(found.len(), 1);
        assert!(found[0].message.contains("Problem:"));
        for label in ["Repair:", "Scope:", "Rationale:"] {
            assert!(found[0].help.contains(label));
        }
        assert!(found[0].help.contains("with_*"));
    }
}
