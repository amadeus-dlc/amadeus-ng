//! 配列・構造の列を **1 行 JSON** にする (公開型ゼロの内部モジュール)。
//!
//! 行の値は基本データ型に限る (裁定 §10 — 読取コマンドが 1 回の引当で答えを得る形)。
//! 配列や入れ子は素の SQLite の列に収まらないので、`ContractCompact` プロファイルの
//! 正準 JSON 文字列 1 本にする。`serde_json::to_string*` は `clippy.toml` の
//! `disallowed-methods` で禁じられているので、`JsonValue` を手で組んで
//! [`serialize`] に渡す (BR1.7 — 契約 JSON の直列化は canon_json の 1 経路)。
//!
//! ここに在るのは**写像だけ**である。何を JSON にするかを決めるのは呼び手 (行) であり、
//! 判断は 1 つも含まない。

use core_command_domain::workflow_definition::{ConsumeDecl, RuleInContext, SensorRef, StageSlug};
use core_infrastructure::canon_json::{JsonValue, ObjectMembers, SerializationProfile, serialize};

/// `JsonValue` を 1 行 JSON にする。
fn compact(value: &JsonValue) -> String {
    serialize(value, SerializationProfile::ContractCompact)
}

/// 省略可能な文字列を JSON の `string | null` にする。
fn optional(value: Option<&str>) -> JsonValue {
    value.map_or(JsonValue::Null, |text| JsonValue::String(text.to_string()))
}

/// 文字列の配列。
pub(crate) fn strings(values: &[String]) -> String {
    compact(&JsonValue::Array(
        values
            .iter()
            .map(|value| JsonValue::String(value.clone()))
            .collect(),
    ))
}

/// slug の配列 (綴りは `StageSlug::as_str`)。
pub(crate) fn slugs(values: &[StageSlug]) -> String {
    compact(&JsonValue::Array(
        values
            .iter()
            .map(|value| JsonValue::String(value.as_str().to_string()))
            .collect(),
    ))
}

/// `produces_kinds` — 成果物名と種別の対の配列。
///
/// 対をオブジェクトのキーに畳まないのは、同じ成果物名が 2 度現れたときに黙って 1 つへ
/// 潰れるからである。行は歴史の写しであって整理ではない。
pub(crate) fn produces_kinds(values: &[(String, Vec<String>)]) -> String {
    compact(&JsonValue::Array(
        values
            .iter()
            .map(|(artifact, kinds)| {
                let mut members = ObjectMembers::new();
                members.insert("artifact", JsonValue::String(artifact.clone()));
                members.insert(
                    "kinds",
                    JsonValue::Array(
                        kinds
                            .iter()
                            .map(|kind| JsonValue::String(kind.clone()))
                            .collect(),
                    ),
                );
                JsonValue::Object(members)
            })
            .collect(),
    ))
}

/// `consumes` — 上流成果物の宣言の配列。
pub(crate) fn consumes(values: &[ConsumeDecl]) -> String {
    compact(&JsonValue::Array(
        values
            .iter()
            .map(|decl| {
                let mut members = ObjectMembers::new();
                members.insert("artifact", JsonValue::String(decl.artifact().to_string()));
                members.insert("required", JsonValue::Bool(decl.required()));
                members.insert(
                    "conditional_on",
                    optional(decl.conditional_on().map(|kind| kind.as_str())),
                );
                JsonValue::Object(members)
            })
            .collect(),
    ))
}

/// `rules_in_context` — compile 時に解決済みのルール行の配列。
pub(crate) fn rules_in_context(values: &[RuleInContext]) -> String {
    compact(&JsonValue::Array(
        values
            .iter()
            .map(|rule| {
                let mut members = ObjectMembers::new();
                members.insert("path", JsonValue::String(rule.path().to_string()));
                members.insert(
                    "scope",
                    JsonValue::String(rule.scope().as_str().to_string()),
                );
                JsonValue::Object(members)
            })
            .collect(),
    ))
}

/// `sensors_applicable` — compile 時に確定したセンサー適用宣言の配列。
pub(crate) fn sensors_applicable(values: &[SensorRef]) -> String {
    compact(&JsonValue::Array(
        values
            .iter()
            .map(|sensor| {
                let mut members = ObjectMembers::new();
                members.insert("id", JsonValue::String(sensor.id().to_string()));
                members.insert("path", JsonValue::String(sensor.path().to_string()));
                members.insert("matches", optional(sensor.matches()));
                JsonValue::Object(members)
            })
            .collect(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_command_domain::workflow_definition::{BrownfieldGreenfield, RuleScope};

    #[test]
    fn an_empty_collection_is_an_empty_array_not_null() {
        assert_eq!(strings(&[]), "[]");
        assert_eq!(slugs(&[]), "[]");
        assert_eq!(produces_kinds(&[]), "[]");
        assert_eq!(consumes(&[]), "[]");
        assert_eq!(rules_in_context(&[]), "[]");
        assert_eq!(sensors_applicable(&[]), "[]");
    }

    #[test]
    fn strings_and_slugs_keep_the_declared_order() {
        assert_eq!(strings(&["b".to_string(), "a".to_string()]), r#"["b","a"]"#);
        assert_eq!(
            slugs(&[
                StageSlug::parse("state-init").expect("文法内"),
                StageSlug::parse("intent-capture").expect("文法内"),
            ]),
            r#"["state-init","intent-capture"]"#
        );
    }

    #[test]
    fn a_repeated_artifact_name_survives_instead_of_collapsing() {
        let value = produces_kinds(&[
            ("a.md".to_string(), vec!["markdown".to_string()]),
            ("a.md".to_string(), vec!["json".to_string()]),
        ]);
        assert_eq!(
            value,
            r#"[{"artifact":"a.md","kinds":["markdown"]},{"artifact":"a.md","kinds":["json"]}]"#
        );
    }

    #[test]
    fn the_optional_members_are_null_when_the_domain_says_none() {
        assert_eq!(
            consumes(&[ConsumeDecl::new("scan.md", false, None)]),
            r#"[{"artifact":"scan.md","required":false,"conditional_on":null}]"#
        );
        assert_eq!(
            consumes(&[ConsumeDecl::new(
                "scan.md",
                true,
                Some(BrownfieldGreenfield::Greenfield)
            )]),
            r#"[{"artifact":"scan.md","required":true,"conditional_on":"greenfield"}]"#
        );
        assert_eq!(
            sensors_applicable(&[SensorRef::new("id", "sensors/id.md", None)]),
            r#"[{"id":"id","path":"sensors/id.md","matches":null}]"#
        );
    }

    #[test]
    fn a_rule_row_carries_its_path_and_its_layer() {
        assert_eq!(
            rules_in_context(&[
                RuleInContext::new("org.md", RuleScope::Org),
                RuleInContext::new("phases/construction.md", RuleScope::Phase),
            ]),
            r#"[{"path":"org.md","scope":"org"},{"path":"phases/construction.md","scope":"phase"}]"#
        );
    }
}
