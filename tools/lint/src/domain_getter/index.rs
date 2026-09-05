//! 型名・import・フィールド・メソッド契約の索引。名前だけの全域一致は行わない。

use super::resolve::Context;
use crate::check::{has_cfg_test, item_attrs};
use std::collections::{BTreeMap, BTreeSet};
use syn::{Item, ReturnType, Type};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) enum Ty {
    Named(String),
    Tuple(Vec<Ty>),
    Wrapper(String, Box<Ty>),
    #[default]
    Unknown,
}

impl Ty {
    pub(super) fn named(&self) -> Option<&str> {
        match self {
            Self::Named(name) => Some(name),
            _ => None,
        }
    }
    pub(super) fn inner(&self) -> Self {
        match self {
            Self::Wrapper(_, inner) => *inner.clone(),
            _ => Self::Unknown,
        }
    }
}

#[derive(Clone)]
pub(super) struct Method {
    pub(super) output: Ty,
    pub(super) getter: bool,
    pub(super) body: Option<syn::Block>,
    pub(super) context: Context,
}

#[derive(Default)]
pub(super) struct Definition {
    pub(super) domain: bool,
    pub(super) fields: BTreeMap<String, Ty>,
    pub(super) methods: BTreeMap<String, Method>,
}

#[derive(Default)]
pub(super) struct Index {
    pub(super) definitions: BTreeMap<String, Definition>,
    pub(super) aliases: BTreeMap<String, String>,
    pub(super) packages: BTreeSet<String>,
    pub(super) contexts: BTreeMap<String, Context>,
    pub(super) test_files: BTreeSet<String>,
}

fn module_path(path: &str) -> Option<String> {
    let (package, file) = path.split_once("/src/")?;
    let package = package.strip_prefix("modules/")?.replace(['/', '-'], "_");
    let file = file.strip_suffix(".rs")?;
    let file = file.strip_suffix("/mod").unwrap_or(file);
    let suffix = if matches!(file, "lib" | "main" | "mod") {
        String::new()
    } else {
        format!("::{}", file.replace('/', "::"))
    };
    Some(format!("{package}{suffix}"))
}

impl Index {
    pub(super) fn build(sources: &[(String, String)]) -> Self {
        let packages = sources
            .iter()
            .filter_map(|(p, _)| module_path(p))
            .filter_map(|p| p.split("::").next().map(str::to_owned))
            .collect();
        let mut index = Self {
            packages,
            ..Self::default()
        };
        let parsed: Vec<_> = sources
            .iter()
            .filter_map(|(path, source)| Some((path, syn::parse_file(source).ok()?)))
            .collect();
        // cfg(test) の外部モジュールはRustの配置規則で解決し、その子孫も除外する。
        let mut test_prefixes = Vec::new();
        for (path, file) in &parsed {
            let directory = module_directory(path);
            if has_cfg_test(&file.attrs) {
                index.test_files.insert((*path).clone());
                test_prefixes.push(format!("{directory}/"));
            }
            collect_test_modules(
                &file.items,
                &directory,
                &mut index.test_files,
                &mut test_prefixes,
            );
        }
        for (path, _) in sources {
            if test_prefixes.iter().any(|prefix| path.starts_with(prefix)) {
                index.test_files.insert(path.clone());
            }
        }
        for (path, file) in &parsed {
            let Some(module) = module_path(path) else {
                continue;
            };
            let context = Context::new(module, &file.items);
            index.contexts.insert((*path).clone(), context.clone());
            index.declare(&context, &file.items, path.contains("/domain/src/"));
        }
        for (path, file) in &parsed {
            if let Some(ctx) = index.contexts.get(*path).cloned() {
                index.contracts(&ctx, &file.items);
            }
        }
        // フィールド → 下位 getter の委譲も固定点まで分類する。
        loop {
            let mut getters = Vec::new();
            for (owner, definition) in &index.definitions {
                if !definition.domain {
                    continue;
                }
                for (name, method) in &definition.methods {
                    if !method.getter
                        && let Some(body) = &method.body
                        && super::projection::is_projection(&index, owner, &method.context, body)
                    {
                        getters.push((owner.clone(), name.clone()));
                    }
                }
            }
            if getters.is_empty() {
                break;
            }
            for (owner, method) in getters {
                index
                    .definitions
                    .get_mut(&owner)
                    .unwrap()
                    .methods
                    .get_mut(&method)
                    .unwrap()
                    .getter = true;
            }
        }
        index
    }

    fn declare(&mut self, ctx: &Context, items: &[Item], domain: bool) {
        for item in items {
            if has_cfg_test(item_attrs(item)) {
                continue;
            }
            let name = match item {
                Item::Struct(s) => Some(&s.ident),
                Item::Enum(e) => Some(&e.ident),
                Item::Trait(t) => Some(&t.ident),
                _ => None,
            };
            if let Some(name) = name {
                self.definitions.insert(
                    format!("{}::{name}", ctx.module),
                    Definition {
                        domain,
                        ..Definition::default()
                    },
                );
            }
            if let Item::Use(u) = item
                && matches!(u.vis, syn::Visibility::Public(_))
            {
                let imported = Context::new(ctx.module.clone(), std::slice::from_ref(item));
                for (alias, target) in &imported.imports {
                    self.aliases.insert(
                        format!("{}::{alias}", ctx.module),
                        self.absolute(ctx, target),
                    );
                }
            }
            if let Item::Type(t) = item
                && let Type::Path(p) = &*t.ty
            {
                let target = p
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::");
                self.aliases.insert(
                    format!("{}::{}", ctx.module, t.ident),
                    self.absolute(ctx, &target),
                );
            }
            if let Item::Mod(m) = item
                && let Some((_, inner)) = &m.content
            {
                self.declare(
                    &Context::new(format!("{}::{}", ctx.module, m.ident), inner),
                    inner,
                    domain,
                );
            }
        }
    }

    fn contracts(&mut self, ctx: &Context, items: &[Item]) {
        for item in items {
            if has_cfg_test(item_attrs(item)) {
                continue;
            }
            match item {
                Item::Struct(s) => {
                    let ctx = self.with_generics(ctx, &s.generics);
                    let fields = s
                        .fields
                        .iter()
                        .enumerate()
                        .map(|(i, f)| {
                            (
                                f.ident
                                    .as_ref()
                                    .map(ToString::to_string)
                                    .unwrap_or(i.to_string()),
                                self.ty(&ctx, &f.ty),
                            )
                        })
                        .collect();
                    if let Some(def) = self
                        .definitions
                        .get_mut(&format!("{}::{}", ctx.module, s.ident))
                    {
                        def.fields = fields;
                    }
                }
                Item::Enum(e) => {
                    let mut fields = BTreeMap::new();
                    for variant in &e.variants {
                        for (i, field) in variant.fields.iter().enumerate() {
                            let name = field
                                .ident
                                .as_ref()
                                .map(ToString::to_string)
                                .unwrap_or(i.to_string());
                            fields.insert(
                                format!("{}::{name}", variant.ident),
                                self.ty(ctx, &field.ty),
                            );
                        }
                    }
                    if let Some(def) = self
                        .definitions
                        .get_mut(&format!("{}::{}", ctx.module, e.ident))
                    {
                        def.fields = fields;
                    }
                }
                Item::Impl(i) => {
                    let ctx = self.with_generics(ctx, &i.generics);
                    let Some(owner) = self.ty(&ctx, &i.self_ty).named().map(str::to_owned) else {
                        continue;
                    };
                    for item in &i.items {
                        if let syn::ImplItem::Fn(f) = item
                            && !has_cfg_test(&f.attrs)
                        {
                            let eligible = i.trait_.is_none()
                                && f.sig.inputs.len() == 1
                                && f.sig.receiver().is_some_and(|r| r.mutability.is_none());
                            self.add_method(
                                &ctx,
                                &owner,
                                &f.sig,
                                eligible.then(|| f.block.clone()),
                            );
                        }
                    }
                }
                Item::Trait(t) => {
                    let owner = format!("{}::{}", ctx.module, t.ident);
                    for item in &t.items {
                        if let syn::TraitItem::Fn(f) = item
                            && !has_cfg_test(&f.attrs)
                        {
                            self.add_method(ctx, &owner, &f.sig, None);
                        }
                    }
                }
                Item::Mod(m) => {
                    if let Some((_, inner)) = &m.content {
                        self.contracts(
                            &Context::new(format!("{}::{}", ctx.module, m.ident), inner),
                            inner,
                        );
                    }
                }
                _ => {}
            }
        }
    }

    fn add_method(
        &mut self,
        ctx: &Context,
        owner: &str,
        sig: &syn::Signature,
        body: Option<syn::Block>,
    ) {
        let mut ctx = self.with_generics(ctx, &sig.generics);
        ctx.generics.insert("Self".into(), Ty::Named(owner.into()));
        let output = match &sig.output {
            ReturnType::Type(_, t) => self.ty(&ctx, t),
            _ => Ty::Unknown,
        };
        if let Some(def) = self.definitions.get_mut(owner) {
            def.methods.insert(
                sig.ident.to_string(),
                Method {
                    output,
                    getter: false,
                    body,
                    context: ctx,
                },
            );
        }
    }
}

/// 入力パスはCLIで `/` 区切りへ正規化済み。foo.rs内の子はfoo/配下になる。
fn module_directory(path: &str) -> String {
    let (parent, name) = path.rsplit_once('/').unwrap_or(("", path));
    if matches!(name, "lib.rs" | "main.rs" | "mod.rs") {
        parent.to_string()
    } else {
        path.strip_suffix(".rs").unwrap_or(path).to_string()
    }
}

fn collect_test_modules(
    items: &[Item],
    directory: &str,
    files: &mut BTreeSet<String>,
    prefixes: &mut Vec<String>,
) {
    for item in items {
        if let Item::Mod(module) = item {
            let nested = format!("{directory}/{}", module.ident);
            if has_cfg_test(&module.attrs) {
                files.insert(format!("{nested}.rs"));
                prefixes.push(format!("{nested}/"));
            }
            if let Some((_, items)) = &module.content {
                collect_test_modules(items, &nested, files, prefixes);
            }
        }
    }
}
