//! 構文と索引の契約から分かる範囲だけで式の型を復元する。

use super::index::{Index, Ty};
use super::resolve::Context;
use std::collections::BTreeMap;
use syn::{Expr, Stmt};
pub(super) type Locals = BTreeMap<String, Ty>;

fn path_text(path: &syn::Path) -> String {
    path.segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

fn member_text(member: &syn::Member) -> String {
    match member {
        syn::Member::Named(n) => n.to_string(),
        syn::Member::Unnamed(n) => n.index.to_string(),
    }
}

fn method_output(index: &Index, receiver: &Ty, name: &str) -> Ty {
    if let Some(owner) = receiver.named() {
        if let Some(method) = index
            .definitions
            .get(owner)
            .and_then(|d| d.methods.get(name))
        {
            return method.output.clone();
        }
        if name == "clone" {
            return receiver.clone();
        }
        return Ty::Unknown;
    }
    if let Ty::Wrapper(wrapper, inner) = receiver {
        match (wrapper.as_str(), name) {
            ("Vec", "iter" | "iter_mut" | "into_iter") => {
                return Ty::Wrapper("Iterator".into(), inner.clone());
            }

            ("Result", "map_err") | ("Option" | "Result", "as_ref" | "as_mut") => {
                return receiver.clone();
            }
            ("Option", "ok_or" | "ok_or_else") => {
                return Ty::Wrapper("Result".into(), inner.clone());
            }
            ("Option" | "Result", "unwrap" | "expect" | "unwrap_or" | "unwrap_or_else") => {
                return *inner.clone();
            }
            ("Box" | "Arc" | "Rc", _) => return method_output(index, inner, name),
            _ => {}
        }
    }
    if name == "clone" {
        return receiver.clone();
    }
    Ty::Unknown
}

pub(super) fn infer(index: &Index, ctx: &Context, locals: &Locals, expr: &Expr) -> Ty {
    match expr {
        Expr::Tuple(t) => Ty::Tuple(
            t.elems
                .iter()
                .map(|e| infer(index, ctx, locals, e))
                .collect(),
        ),
        Expr::Path(p) if p.qself.is_none() => {
            locals.get(&path_text(&p.path)).cloned().unwrap_or_default()
        }
        Expr::Reference(r) => infer(index, ctx, locals, &r.expr),
        Expr::Paren(p) => infer(index, ctx, locals, &p.expr),
        Expr::Group(g) => infer(index, ctx, locals, &g.expr),
        Expr::Unary(u) if matches!(u.op, syn::UnOp::Deref(_)) => {
            let ty = infer(index, ctx, locals, &u.expr);
            if matches!(ty, Ty::Wrapper(_, _)) {
                ty.inner()
            } else {
                ty
            }
        }
        Expr::Await(a) => infer(index, ctx, locals, &a.base),
        Expr::Try(t) => infer(index, ctx, locals, &t.expr).inner(),
        Expr::Field(f) => {
            let ty = infer(index, ctx, locals, &f.base);
            ty.named()
                .and_then(|n| index.definitions.get(n))
                .and_then(|d| d.fields.get(&member_text(&f.member)))
                .cloned()
                .unwrap_or_default()
        }
        Expr::MethodCall(m) => method_output(
            index,
            &infer(index, ctx, locals, &m.receiver),
            &m.method.to_string(),
        ),
        Expr::Call(call) => {
            let Expr::Path(path) = &*call.func else {
                return Ty::Unknown;
            };
            if path.qself.is_none() && path.path.segments.len() == 1 {
                let name = path.path.segments[0].ident.to_string();
                if matches!(name.as_str(), "Some" | "Ok")
                    && let Some(arg) = call.args.first()
                {
                    return Ty::Wrapper(
                        if name == "Some" { "Option" } else { "Result" }.into(),
                        Box::new(infer(index, ctx, locals, arg)),
                    );
                }
                if let Some(name) = index.resolve(ctx, &name) {
                    return Ty::Named(name);
                }
            }
            call_owner(index, ctx, path)
                .map(|(owner, method)| method_output(index, &Ty::Named(owner), &method))
                .unwrap_or_default()
        }
        Expr::Struct(s) => index
            .resolve(ctx, &path_text(&s.path))
            .map(Ty::Named)
            .unwrap_or_default(),
        Expr::Block(b) => b
            .block
            .stmts
            .last()
            .and_then(|s| {
                if let Stmt::Expr(e, None) = s {
                    Some(infer(index, ctx, locals, e))
                } else {
                    None
                }
            })
            .unwrap_or_default(),
        _ => Ty::Unknown,
    }
}

pub(super) fn call_owner(
    index: &Index,
    ctx: &Context,
    path: &syn::ExprPath,
) -> Option<(String, String)> {
    let method = path.path.segments.last()?.ident.to_string();
    let owner = if let Some(qself) = &path.qself {
        if qself.position > 0 {
            return None;
        }
        index.ty(ctx, &qself.ty).named()?.to_string()
    } else {
        let raw = path
            .path
            .segments
            .iter()
            .take(path.path.segments.len().checked_sub(1)?)
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        if let Some(ty) = ctx.generics.get(&raw) {
            ty.named()?.to_string()
        } else {
            index.resolve(ctx, &raw)?
        }
    };
    Some((owner, method))
}
