//! ドメインメソッド本体の保持データ射影を分類する。

use super::index::{Index, Ty};
use super::infer::{Locals, call_owner, infer};
use super::resolve::Context;
use std::collections::BTreeMap;
use syn::{Expr, Stmt};

/// 「保持したものをそのまま返す」式だけを分類する。比較・算術・述語・整形・
/// 優先順位解決は含めない。メソッド名ではなく本体の構造が根拠になる。
pub(super) fn is_projection(index: &Index, owner: &str, ctx: &Context, block: &syn::Block) -> bool {
    let mut locals = BTreeMap::from([("self".into(), Ty::Named(owner.into()))]);
    for (i, stmt) in block.stmts.iter().enumerate() {
        match stmt {
            Stmt::Local(local) => {
                let Some(init) = &local.init else {
                    return false;
                };
                let syn::Pat::Ident(pat) = &local.pat else {
                    return false;
                };
                if init.diverge.is_some() || !projection(index, ctx, &locals, &init.expr) {
                    return false;
                }
                locals.insert(
                    pat.ident.to_string(),
                    infer(index, ctx, &locals, &init.expr),
                );
            }
            Stmt::Expr(expr, _) if i + 1 == block.stmts.len() => {
                return projection(index, ctx, &locals, expr);
            }
            _ => return false,
        }
    }
    false
}

fn projection(index: &Index, ctx: &Context, locals: &Locals, expr: &Expr) -> bool {
    match expr {
        Expr::Path(p) => p
            .path
            .get_ident()
            .is_some_and(|name| name != "self" && locals.contains_key(&name.to_string())),
        Expr::Match(m) => {
            let receiver = infer(index, ctx, locals, &m.expr);
            let Some(definition) = receiver
                .named()
                .and_then(|owner| index.definitions.get(owner))
            else {
                return false;
            };
            if !matches!(&*m.expr, Expr::Path(p) if p.path.is_ident("self"))
                && !projection(index, ctx, locals, &m.expr)
            {
                return false;
            }
            !m.arms.is_empty()
                && m.arms.iter().all(|arm| {
                    if arm.guard.is_some() {
                        return false;
                    }
                    let syn::Pat::TupleStruct(pattern) = &arm.pat else {
                        return false;
                    };
                    let Some(variant) = pattern.path.segments.last() else {
                        return false;
                    };
                    let mut locals = locals.clone();
                    for (i, pat) in pattern.elems.iter().enumerate() {
                        let syn::Pat::Ident(pat) = pat else {
                            return false;
                        };
                        let Some(ty) = definition.fields.get(&format!("{}::{i}", variant.ident))
                        else {
                            return false;
                        };
                        locals.insert(pat.ident.to_string(), ty.clone());
                    }
                    projection(index, ctx, &locals, &arm.body)
                })
        }

        Expr::Reference(r) => projection(index, ctx, locals, &r.expr),
        Expr::Paren(p) => projection(index, ctx, locals, &p.expr),
        Expr::Return(r) => r
            .expr
            .as_ref()
            .is_some_and(|e| projection(index, ctx, locals, e)),
        Expr::Field(f) => {
            matches!(&*f.base, Expr::Path(p) if p.path.is_ident("self"))
                || projection(index, ctx, locals, &f.base)
        }
        Expr::Index(i) => projection(index, ctx, locals, &i.expr),
        Expr::MethodCall(m) => {
            let receiver = infer(index, ctx, locals, &m.receiver);
            let name = m.method.to_string();
            let getter = receiver
                .named()
                .and_then(|owner| index.definitions.get(owner))
                .and_then(|d| d.methods.get(&name))
                .is_some_and(|method| method.getter);
            if getter && m.args.is_empty() {
                return matches!(&*m.receiver, Expr::Path(p) if p.path.is_ident("self"))
                    || projection(index, ctx, locals, &m.receiver);
            }
            if !projection(index, ctx, locals, &m.receiver) {
                return false;
            }
            // 借用・複製・コンテナからの取り出しは判断を追加しない。
            if matches!(
                name.as_str(),
                "as_str" | "as_slice" | "as_ref" | "as_deref" | "clone" | "copied" | "cloned"
            ) && m.args.is_empty()
            {
                // 型が既知の独自ドメインメソッドを標準変換と取り違えない。
                return receiver.named().is_none();
            }
            if name == "get" && m.args.len() == 1 {
                return matches!(receiver, Ty::Wrapper(ref w, _) if w == "Vec");
            }
            if name == "map"
                && m.args.len() == 1
                && let Some(Expr::Path(p)) = m.args.first()
                && let Some((owner, method)) = call_owner(index, ctx, p)
            {
                return index
                    .definitions
                    .get(&owner)
                    .and_then(|d| d.methods.get(&method))
                    .is_some_and(|m| m.getter);
            }
            false
        }
        _ => false,
    }
}
