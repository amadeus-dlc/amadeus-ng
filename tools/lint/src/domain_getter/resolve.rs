//! import・型エイリアス・型注釈を定義の所有元へ結ぶ。

use super::index::{Index, Ty};
use std::collections::{BTreeMap, BTreeSet};
use syn::{GenericArgument, Item, PathArguments, Type, UseTree};

#[derive(Clone)]
pub(super) struct Context {
    pub(super) module: String,
    pub(super) imports: BTreeMap<String, String>,
    globs: Vec<String>,
    pub(super) generics: BTreeMap<String, Ty>,
}

impl Context {
    pub(super) fn new(module: String, items: &[Item]) -> Self {
        let mut ctx = Self {
            module,
            imports: BTreeMap::new(),
            globs: Vec::new(),
            generics: BTreeMap::new(),
        };
        for item in items {
            if let Item::Use(item) = item {
                ctx.add_use(&item.tree, String::new());
            }
        }
        ctx
    }
    fn add_use(&mut self, tree: &UseTree, prefix: String) {
        match tree {
            UseTree::Path(p) => self.add_use(&p.tree, format!("{prefix}{}::", p.ident)),
            UseTree::Name(n) => {
                self.imports
                    .insert(n.ident.to_string(), format!("{prefix}{}", n.ident));
            }
            UseTree::Rename(n) => {
                self.imports
                    .insert(n.rename.to_string(), format!("{prefix}{}", n.ident));
            }
            UseTree::Group(g) => {
                for item in &g.items {
                    self.add_use(item, prefix.clone());
                }
            }
            UseTree::Glob(_) => self.globs.push(prefix.trim_end_matches("::").to_string()),
        }
    }
    pub(super) fn with_items(&self, items: &[Item]) -> Self {
        let mut ctx = self.clone();
        for item in items {
            if let Item::Use(i) = item {
                ctx.add_use(&i.tree, String::new());
            }
        }
        ctx
    }
}

impl Index {
    pub(super) fn absolute(&self, ctx: &Context, raw: &str) -> String {
        let mut parts = raw.split("::");
        let first = parts.next().unwrap_or_default();
        let rest = parts.collect::<Vec<_>>().join("::");
        let suffix = if rest.is_empty() {
            String::new()
        } else {
            format!("::{rest}")
        };
        match first {
            "crate" => format!("{}{suffix}", ctx.module.split("::").next().unwrap()),
            "self" => format!("{}{suffix}", ctx.module),
            "super" => {
                let mut parent = ctx.clone();
                parent.module = ctx
                    .module
                    .rsplit_once("::")
                    .map_or(ctx.module.as_str(), |(p, _)| p)
                    .to_string();
                if rest.starts_with("super::") {
                    self.absolute(&parent, &rest)
                } else {
                    format!("{}{suffix}", parent.module)
                }
            }
            _ => {
                if let Some(target) = ctx.imports.get(first) {
                    let mut bare = ctx.clone();
                    bare.imports.remove(first);
                    format!("{}{suffix}", self.absolute(&bare, target))
                } else if self.packages.contains(first) {
                    raw.to_string()
                } else {
                    format!("{}::{raw}", ctx.module)
                }
            }
        }
    }

    pub(super) fn resolve(&self, ctx: &Context, raw: &str) -> Option<String> {
        let mut candidate = self.absolute(ctx, raw);
        let mut seen = BTreeSet::new();
        while let Some(alias) = self.aliases.get(&candidate) {
            if !seen.insert(candidate.clone()) {
                return None;
            }
            candidate = alias.clone();
        }
        if self.definitions.contains_key(&candidate) {
            return Some(candidate);
        }
        let mut found = BTreeSet::new();
        for glob in &ctx.globs {
            let raw = format!("{glob}::{raw}");
            let mut without_globs = ctx.clone();
            without_globs.globs.clear();
            if let Some(name) = self.resolve(&without_globs, &raw) {
                found.insert(name);
            }
        }
        if found.len() == 1 {
            found.into_iter().next()
        } else {
            None
        }
    }

    pub(super) fn ty(&self, ctx: &Context, ty: &Type) -> Ty {
        match ty {
            Type::Tuple(t) => Ty::Tuple(t.elems.iter().map(|t| self.ty(ctx, t)).collect()),
            Type::Reference(r) => self.ty(ctx, &r.elem),
            Type::Paren(p) => self.ty(ctx, &p.elem),
            Type::Path(p) if p.qself.is_none() => {
                let raw = p
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::");
                if let Some(ty) = ctx.generics.get(&raw) {
                    return ty.clone();
                }
                if let Some(name) = self.resolve(ctx, &raw) {
                    return Ty::Named(name);
                }
                let Some(last) = p.path.segments.last() else {
                    return Ty::Unknown;
                };
                let name = last.ident.to_string();
                if matches!(
                    name.as_str(),
                    "Option" | "Result" | "Box" | "Arc" | "Rc" | "Vec"
                ) && (raw == name
                    || raw.starts_with("std::")
                    || raw.starts_with("core::")
                    || raw.starts_with("alloc::"))
                    && !ctx.imports.contains_key(&name)
                    && let PathArguments::AngleBracketed(args) = &last.arguments
                    && let Some(GenericArgument::Type(inner)) = args.args.first()
                {
                    return Ty::Wrapper(name, Box::new(self.ty(ctx, inner)));
                }
                Ty::Unknown
            }
            _ => Ty::Unknown,
        }
    }

    pub(super) fn with_generics(&self, ctx: &Context, generics: &syn::Generics) -> Context {
        let mut ctx = ctx.clone();
        let mut bounds = Vec::new();
        for p in generics.type_params() {
            ctx.generics.insert(p.ident.to_string(), Ty::Unknown);
            bounds.push((p.ident.to_string(), &p.bounds));
        }
        if let Some(clause) = &generics.where_clause {
            for pred in &clause.predicates {
                if let syn::WherePredicate::Type(p) = pred
                    && let Type::Path(t) = &p.bounded_ty
                    && t.path.segments.len() == 1
                {
                    bounds.push((t.path.segments[0].ident.to_string(), &p.bounds));
                }
            }
        }
        for (name, bounds) in bounds {
            let resolved: Vec<_> = bounds
                .iter()
                .filter_map(|b| {
                    let syn::TypeParamBound::Trait(t) = b else {
                        return None;
                    };
                    let raw = t
                        .path
                        .segments
                        .iter()
                        .map(|s| s.ident.to_string())
                        .collect::<Vec<_>>()
                        .join("::");
                    self.resolve(&ctx, &raw)
                })
                .collect();
            if resolved.len() == 1 {
                ctx.generics.insert(name, Ty::Named(resolved[0].clone()));
            }
        }
        ctx
    }
}
