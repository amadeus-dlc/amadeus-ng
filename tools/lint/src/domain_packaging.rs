//! R11: ドメイン層の技術駆動パッケージングの禁止
//! (`aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/domain-packaging.md`、
//! オーナー裁定 2026-09-25)。
//!
//! ドメイン層は境界づけられたコンテキストと、型が所有するサブツリーで分ける。
//! `entities/`・`value_objects/` のように、DDD のパターン名や実装の役割でディレクトリ・
//! モジュールを区切ったら所見にする。検出はパスの各区間 (ディレクトリ名とファイル名) と、
//! ファイル内のインライン `mod` 宣言の名前の完全一致だけで行う — `intent_execution_event/`
//! のように特定の型名が `_event` で終わるものは種類名ではないので鳴らさない。
use crate::check::Finding;

/// 安定したルール識別子。
pub(crate) const RULE: &str = "domain-packaging";

/// 射程 — コマンド側のドメイン層。
const DOMAIN_SCOPE: &str = "modules/core/command/domain/src/";

/// 種類・役割を表す名前。どれか 1 つでも区間名に一致したら所見。
const TECHNICAL_NAMES: [&str; 26] = [
    "entities",
    "entity",
    "value_objects",
    "value_object",
    "vo",
    "aggregates",
    "aggregate",
    "domain_events",
    "events",
    "services",
    "service",
    "domain_services",
    "repositories",
    "repository",
    "factories",
    "factory",
    "models",
    "model",
    "types",
    "common",
    "shared",
    "utils",
    "util",
    "helpers",
    "helper",
    "misc",
];

const HELP: &str = "ドメイン層は境界づけられたコンテキスト → 型が所有するサブツリー (所有者の型名の snake_case) で分け、\
種類・役割の名前で束ねない。型はその概念を所有するコンテキストへ置く — \
aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/domain-packaging.md";

/// `path` (リポジトリルートからの相対、区切りは `/`) と構文木を検査する。
pub(crate) fn check(path: &str, file: &syn::File) -> Vec<Finding> {
    let Some(inside) = path.strip_prefix(DOMAIN_SCOPE) else {
        return Vec::new();
    };
    let mut findings: Vec<Finding> = inside
        .split('/')
        .map(|segment| segment.strip_suffix(".rs").unwrap_or(segment))
        .filter(|segment| is_technical(segment))
        .map(|segment| {
            Finding::new(
                RULE,
                1,
                format!("ドメイン層の区間 `{segment}` は種類・役割の名前 (技術駆動パッケージング)"),
                HELP,
            )
        })
        .collect();
    findings.extend(file.items.iter().filter_map(|item| match item {
        syn::Item::Mod(module) if is_technical(&module.ident.to_string()) => Some(Finding::new(
            RULE,
            module.ident.span().start().line,
            format!(
                "ドメイン層の mod `{}` は種類・役割の名前 (技術駆動パッケージング)",
                module.ident
            ),
            HELP,
        )),
        _ => None,
    }));
    findings
}

/// 区間名が種類・役割の名前か。`#[path]` でハイフンを含むディレクトリ (`value-objects/`) にも
/// モジュールを置けるので、`-` を `_` に揃えてから比べる。
fn is_technical(name: &str) -> bool {
    let name = name.strip_prefix("r#").unwrap_or(name).replace('-', "_");
    TECHNICAL_NAMES.contains(&name.as_str())
}

#[cfg(test)]
mod tests {
    use super::RULE;

    fn findings(path: &str, source: &str) -> Vec<crate::check::Finding> {
        crate::check::check_source(path, source)
            .expect("テストのソースは構文解析できること")
            .into_iter()
            .filter(|finding| finding.rule == RULE)
            .collect()
    }

    // ---- 赤例 -----------------------------------------------------------

    #[test]
    fn detects_technical_directories_in_the_domain_layer() {
        // オーナー裁定の例そのもの: entities/・value-objects/ のような区切り。
        let found = findings(
            "modules/core/command/domain/src/entities/intent.rs",
            "pub struct Intent;\n",
        );
        assert_eq!(found.len(), 1);
        assert!(
            found[0].message.contains("`entities`"),
            "{}",
            found[0].message
        );
        let nested = findings(
            "modules/core/command/domain/src/orchestration/value_objects/stage_slug.rs",
            "pub struct StageSlug;\n",
        );
        assert_eq!(nested.len(), 1, "コンテキストの内側の種類別区切りも鳴らす");
        // オーナー裁定の綴りそのもの (`value-objects/`)。`#[path]` でハイフンのディレクトリにも置ける。
        let hyphen = findings(
            "modules/core/command/domain/src/value-objects/stage_slug.rs",
            "pub struct StageSlug;\n",
        );
        assert_eq!(hyphen.len(), 1);
        assert!(
            hyphen[0].message.contains("`value-objects`"),
            "{}",
            hyphen[0].message
        );
    }

    #[test]
    fn detects_technical_facade_files_and_inline_modules() {
        let facade = findings("modules/core/command/domain/src/events.rs", "mod intent;\n");
        assert_eq!(facade.len(), 1, "種類名のファサードファイル (events.rs)");
        let inline = findings(
            "modules/core/command/domain/src/orchestration/mod.rs",
            "mod services;\nmod intent;\n",
        );
        assert_eq!(inline.len(), 1);
        assert_eq!(inline[0].line, 1);
        assert!(inline[0].message.contains("`services`"));
    }

    // ---- 緑例 -----------------------------------------------------------

    #[test]
    fn allows_contexts_and_type_owned_subtrees() {
        for path in [
            "modules/core/command/domain/src/orchestration/intent_execution.rs",
            "modules/core/command/domain/src/orchestration/intent_execution_event/started.rs",
            "modules/core/command/domain/src/workflow_definition/stage_node/condition.rs",
            "modules/core/command/domain/src/workspace/mod.rs",
            "modules/core/command/domain/src/lib.rs",
        ] {
            assert!(
                findings(path, "pub struct Anything;\n").is_empty(),
                "{path}"
            );
        }
    }

    #[test]
    fn leaves_other_layers_out_of_scope() {
        // dto/ や port/ は既存裁定の技術境界。ドメイン層の外は射程外。
        for path in [
            "modules/core/command/interface-adapter/src/orchestration/services/x.rs",
            "modules/core/query/use-case/src/orchestration/port/run_stage_dao.rs",
            "modules/app/aidlc/src/common/x.rs",
        ] {
            assert!(findings(path, "mod events;\n").is_empty(), "{path}");
        }
    }
}
