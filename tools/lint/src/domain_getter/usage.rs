//! ローカル束縛と契約の型を追跡し、保持データの射影呼出しを報告する。

use super::index::{Index, Ty};
use super::infer::{Locals, call_owner, infer};
use super::resolve::Context;
use crate::check::{Finding, has_cfg_test, impl_item_attrs, item_attrs};
use syn::{
    Expr, FnArg, Item, Pat, Stmt,
    visit::{self, Visit},
};

const RULE: &str = "use-case-domain-getter";
const HELP: &str = "業務判断は状態の所有者へ委譲する。例外は型付きIDをRepositoryのfind_by_idへ直接渡す用途だけであり、IDの比較や他の状態取得は許可しない (coding-rules/tell-dont-ask.md)";

pub(super) fn check(index: &Index, path: &str, file: &syn::File) -> Vec<Finding> {
    let Some(context) = index.contexts.get(path) else {
        return Vec::new();
    };
    let mut visitor = Usage {
        index,
        context: context.clone(),
        locals: Locals::new(),
        findings: Vec::new(),
    };
    visitor.visit_file(file);
    visitor.findings
}

struct Usage<'a> {
    index: &'a Index,
    context: Context,
    locals: Locals,
    findings: Vec<Finding>,
}

impl Usage<'_> {
    fn repository_key(&self, receiver: &Ty, method: &str) -> Option<Ty> {
        if method != "find_by_id" {
            return None;
        }
        let definition = self.index.definitions.get(receiver.named()?)?;
        let method = definition.methods.get(method)?;
        let [key] = method.inputs.as_slice() else {
            return None;
        };
        let name = key.named()?;
        (definition.repository
            && name.ends_with("Id")
            && self.index.definitions.get(name).is_some_and(|d| d.domain))
        .then(|| key.clone())
    }

    fn visit_lookup_key(&mut self, expr: &Expr, key: &Ty) {
        match expr {
            Expr::Reference(reference) => self.visit_lookup_key(&reference.expr, key),
            Expr::Paren(paren) => self.visit_lookup_key(&paren.expr, key),
            Expr::Group(group) => self.visit_lookup_key(&group.expr, key),
            Expr::MethodCall(call)
                if call.args.is_empty()
                    && infer(self.index, &self.context, &self.locals, expr) == *key =>
            {
                if call.method == "clone" {
                    self.visit_lookup_key(&call.receiver, key);
                } else {
                    // 引数そのもののID取得だけを許可し、所有者へ至る式は通常どおり検査する。
                    self.visit_expr(&call.receiver);
                }
            }
            Expr::Call(call)
                if call.args.len() == 1
                    && infer(self.index, &self.context, &self.locals, expr) == *key =>
            {
                // UFCSで書かれたgetterも同じ扱いにする。引数内の別のgetterは許可しない。
                for arg in &call.args {
                    self.visit_expr(arg);
                }
            }
            _ => self.visit_expr(expr),
        }
    }

    fn bind(&mut self, pat: &Pat, ty: Ty) {
        match pat {
            Pat::Ident(i) => {
                self.locals.insert(i.ident.to_string(), ty);
            }
            Pat::Type(t) => {
                let ty = self.index.ty(&self.context, &t.ty);
                self.bind(&t.pat, ty);
            }
            Pat::Reference(r) => self.bind(&r.pat, ty),
            Pat::TupleStruct(p) if p.elems.len() == 1 => self.bind(&p.elems[0], ty.inner()),
            Pat::Tuple(t) => {
                let elements = if let Ty::Tuple(elements) = ty {
                    elements
                } else {
                    Vec::new()
                };
                for (i, p) in t.elems.iter().enumerate() {
                    self.bind(p, elements.get(i).cloned().unwrap_or_default());
                }
            }
            Pat::Struct(s) => {
                for f in &s.fields {
                    self.bind(&f.pat, Ty::Unknown);
                }
            }
            _ => {}
        }
    }
    fn function(&mut self, sig: &syn::Signature, block: &syn::Block) {
        let before = self.locals.clone();
        let ctx = self.context.clone();
        self.context = self.index.with_generics(&self.context, &sig.generics);
        for arg in &sig.inputs {
            match arg {
                FnArg::Typed(t) => self.bind(&t.pat, self.index.ty(&self.context, &t.ty)),
                FnArg::Receiver(_) => {
                    self.locals.insert(
                        "self".into(),
                        self.context
                            .generics
                            .get("Self")
                            .cloned()
                            .unwrap_or_default(),
                    );
                }
            }
        }
        self.visit_block(block);
        self.locals = before;
        self.context = ctx;
    }
    fn report(&mut self, receiver: &Ty, method: &str, line: usize) {
        let receiver = if matches!(receiver, Ty::Wrapper(w, _) if matches!(w.as_str(), "Box" | "Arc" | "Rc"))
        {
            receiver.inner()
        } else {
            receiver.clone()
        };
        let Some(owner) = receiver.named() else {
            return;
        };
        if self
            .index
            .definitions
            .get(owner)
            .is_some_and(|d| d.domain && d.methods.get(method).is_some_and(|m| m.getter))
        {
            self.findings.push(Finding::new(
                RULE,
                line,
                format!("use-case 層からドメイン getter `{owner}::{method}` を呼び出している"),
                HELP,
            ));
        }
    }
}

impl<'ast> Visit<'ast> for Usage<'_> {
    fn visit_item(&mut self, item: &'ast Item) {
        if has_cfg_test(item_attrs(item)) {
            return;
        }
        visit::visit_item(self, item);
    }
    fn visit_impl_item(&mut self, item: &'ast syn::ImplItem) {
        if has_cfg_test(impl_item_attrs(item)) {
            return;
        }
        visit::visit_impl_item(self, item);
    }
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if let Some((_, items)) = &item.content {
            let previous = self.context.clone();
            self.context.module = format!("{}::{}", self.context.module, item.ident);
            self.context = self.context.with_items(items);
            for item in items {
                self.visit_item(item);
            }
            self.context = previous;
        }
    }
    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        let previous = self.context.clone();
        self.context = self.index.with_generics(&self.context, &item.generics);
        let ty = self.index.ty(&self.context, &item.self_ty);
        self.context.generics.insert("Self".into(), ty);
        for item in &item.items {
            self.visit_impl_item(item);
        }
        self.context = previous;
    }
    fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
        if !has_cfg_test(&item.attrs)
            && let Some(body) = &item.default
        {
            self.function(&item.sig, body);
        }
    }
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        self.function(&item.sig, &item.block);
    }
    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        self.function(&item.sig, &item.block);
    }
    fn visit_block(&mut self, block: &'ast syn::Block) {
        let before = self.locals.clone();
        let ctx = self.context.clone();
        let items = block
            .stmts
            .iter()
            .filter_map(|s| {
                if let Stmt::Item(i) = s {
                    Some(i.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        self.context = self.context.with_items(&items);
        for stmt in &block.stmts {
            self.visit_stmt(stmt);
        }
        self.locals = before;
        self.context = ctx;
    }
    fn visit_local(&mut self, local: &'ast syn::Local) {
        if has_cfg_test(&local.attrs) {
            return;
        }
        let ty = if let Some(init) = &local.init {
            self.visit_expr(&init.expr);
            if let Some((_, diverge)) = &init.diverge {
                self.visit_expr(diverge);
            }
            infer(self.index, &self.context, &self.locals, &init.expr)
        } else {
            Ty::Unknown
        };
        self.bind(&local.pat, ty);
    }
    fn visit_expr_method_call(&mut self, expr: &'ast syn::ExprMethodCall) {
        let ty = infer(self.index, &self.context, &self.locals, &expr.receiver);
        let key = self.repository_key(&ty, &expr.method.to_string());
        self.report(
            &ty,
            &expr.method.to_string(),
            expr.method.span().start().line,
        );
        self.visit_expr(&expr.receiver);
        for arg in &expr.args {
            if let Some(key) = &key {
                self.visit_lookup_key(arg, key);
                continue;
            }
            if let Expr::Closure(closure) = arg
                && matches!(&ty, Ty::Wrapper(kind, _) if matches!(kind.as_str(), "Iterator" | "Option" | "Result"))
                && matches!(
                    expr.method.to_string().as_str(),
                    "map" | "filter" | "filter_map" | "and_then" | "for_each"
                )
                && closure.inputs.len() == 1
            {
                let previous = self.locals.clone();
                self.bind(&closure.inputs[0], ty.inner());
                self.visit_expr(&closure.body);
                self.locals = previous;
            } else {
                self.visit_expr(arg);
            }
        }
    }
    fn visit_expr_call(&mut self, expr: &'ast syn::ExprCall) {
        if let Expr::Path(p) = &*expr.func
            && let Some((owner, method)) = call_owner(self.index, &self.context, p)
        {
            if let Some(key) = self.repository_key(&Ty::Named(owner.clone()), &method)
                && expr.args.len() == 2
            {
                self.visit_expr(&expr.args[0]);
                self.visit_lookup_key(&expr.args[1], &key);
                return;
            }
            self.report(
                &Ty::Named(owner),
                &method,
                p.path.segments.last().unwrap().ident.span().start().line,
            );
        }
        visit::visit_expr_call(self, expr);
    }
    fn visit_expr_match(&mut self, expr: &'ast syn::ExprMatch) {
        self.visit_expr(&expr.expr);
        let ty = infer(self.index, &self.context, &self.locals, &expr.expr);
        for arm in &expr.arms {
            let previous = self.locals.clone();
            self.bind(&arm.pat, ty.clone());
            if let Some((_, guard)) = &arm.guard {
                self.visit_expr(guard);
            }
            self.visit_expr(&arm.body);
            self.locals = previous;
        }
    }
    fn visit_expr_if(&mut self, expr: &'ast syn::ExprIf) {
        let previous = self.locals.clone();
        self.visit_expr(&expr.cond);
        self.visit_block(&expr.then_branch);
        self.locals = previous.clone();
        if let Some((_, branch)) = &expr.else_branch {
            self.visit_expr(branch);
        }
        self.locals = previous;
    }
    fn visit_expr_while(&mut self, expr: &'ast syn::ExprWhile) {
        let previous = self.locals.clone();
        self.visit_expr(&expr.cond);
        self.visit_block(&expr.body);
        self.locals = previous;
    }
    fn visit_expr_for_loop(&mut self, expr: &'ast syn::ExprForLoop) {
        let previous = self.locals.clone();
        self.visit_expr(&expr.expr);
        let ty = infer(self.index, &self.context, &self.locals, &expr.expr);
        self.bind(&expr.pat, ty.inner());
        self.visit_block(&expr.body);
        self.locals = previous;
    }
    fn visit_expr_let(&mut self, expr: &'ast syn::ExprLet) {
        self.visit_expr(&expr.expr);
        let ty = infer(self.index, &self.context, &self.locals, &expr.expr);
        self.bind(&expr.pat, ty);
    }
    fn visit_expr_closure(&mut self, expr: &'ast syn::ExprClosure) {
        let previous = self.locals.clone();
        for input in &expr.inputs {
            self.bind(input, Ty::Unknown);
        }
        self.visit_expr(&expr.body);
        self.locals = previous;
    }
}
