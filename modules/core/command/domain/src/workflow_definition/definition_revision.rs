//! `DefinitionRevision` — `WorkflowDefinition` の内容版 (ADR-008)。識別子ではなく**値属性**。

use std::collections::BTreeMap;
use std::fmt;

use core_infrastructure::canon_json::{Digest, JsonValue, Number, ObjectMembers, hash_canonical};

use super::brownfield_greenfield::BrownfieldGreenfield;
use super::definition_revision_error::DefinitionRevisionError;
use super::review_class::ReviewClass;
use super::scope_grid::ScopeGrid;
use super::scope_metadata::ScopeMetadata;
use super::stage_graph::StageGraph;
use super::stage_node::StageNode;

/// 正準ダイジェストの接頭辞 (canon-json の正準族 `Digest::rendered()` と同じ表記)。
const PREFIX: &str = "sha256:";
/// sha256 の 16 進表記の桁数。`DefinitionRevisionError` の Display も同じ桁数を文言に
/// 載せるため、値の正本をここ 1 箇所に置いたまま兄弟モジュールへ見せる。
pub(super) const HEX_LEN: usize = 64;

/// 3 入力 (コンパイル済み `stage-graph.json` / `scope-grid.json` / scope identity 群) の
/// 正準 JSON の sha256 ダイジェスト (Always Valid)。
///
/// 同じ内容なら同じ revision、ピン更新で変わる。**識別子ではない**ため、これが変わっても
/// 定義の系譜 (`WorkflowDefinitionId`) は変わらない。来歴と drift 検出の材料。
///
/// 値は**内容の純粋な関数**として集約 `CompiledDefinition` が自分で導出する
/// ([`DefinitionRevision::of_content`] — ADR-008 改訂 2026-09-02、b36。~~アダプタ層が
/// 生バイトから計算し、ドメインは計算しない~~ は、配布束が読取専用だった頃の裁定で失効)。
/// ダイジェストは言語拡張 canon-json の正準族 (`hash_canonical`) で、投影はドメインの値だけ
/// から組む — ファイルの生バイト・未知フィールド・キー順といった**表現**は入力に含めない。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DefinitionRevision(String);

impl DefinitionRevision {
    /// `sha256:<hex64>` (16 進は小文字) のみを受理する。
    ///
    /// # Errors
    ///
    /// 接頭辞の欠落・桁数違い・16 進小文字以外の文字を拒否する。
    pub fn parse(s: &str) -> Result<DefinitionRevision, DefinitionRevisionError> {
        let hex = s
            .strip_prefix(PREFIX)
            .ok_or(DefinitionRevisionError::MissingPrefix)?;
        if hex.len() != HEX_LEN {
            return Err(DefinitionRevisionError::InvalidLength { actual: hex.len() });
        }
        if let Some(c) = hex
            .chars()
            .find(|c| !(c.is_ascii_digit() || matches!(c, 'a'..='f')))
        {
            return Err(DefinitionRevisionError::InvalidHexDigit(c));
        }
        Ok(DefinitionRevision::of_rendered(s.to_string()))
    }

    /// 内容 (graph / grid / scopes) から内容版を導出する。
    ///
    /// 同じ内容なら同じ値、内容が 1 箇所でも違えば別の値。集約が遷移のたびに呼ぶので、
    /// 集約は常に自分の内容版を知っている (呼出側に適用後の内容を先読みさせない —
    /// tell-dont-ask)。`produces_kinds` だけは**並びが内容**なので、正準族のキーソートに
    /// 潰されない配列で投影する。
    #[must_use]
    pub fn of_content(
        graph: &StageGraph,
        grid: &ScopeGrid,
        scopes: &BTreeMap<String, ScopeMetadata>,
    ) -> DefinitionRevision {
        let mut input = ObjectMembers::new();
        input.insert(
            "stage_graph",
            JsonValue::Array(graph.fold_left(
                Vec::with_capacity(graph.len()),
                |mut nodes, node| {
                    nodes.push(project_node(node));
                    nodes
                },
            )),
        );
        input.insert("scope_grid", project_grid(grid));
        input.insert(
            "scopes",
            JsonValue::Array(
                scopes
                    .iter()
                    .map(|(name, metadata)| project_scope(name, metadata))
                    .collect(),
            ),
        );
        DefinitionRevision::of_digest(&hash_canonical(&JsonValue::Object(input)))
    }

    /// 正準族のダイジェストをそのまま内容版にする。
    ///
    /// `Digest::rendered()` は `sha256:<hex64>` の綴りそのものなので、`parse` の検査を通し
    /// 直さない (往復はテストで固定)。
    fn of_digest(digest: &Digest) -> DefinitionRevision {
        DefinitionRevision::of_rendered(digest.rendered())
    }

    /// 構造体リテラルはここだけ (`parse` の検査済み値と、正準ダイジェストの 2 経路が通る)。
    const fn of_rendered(rendered: String) -> DefinitionRevision {
        DefinitionRevision(rendered)
    }

    /// `sha256:` 接頭辞付きの生表記。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 接頭辞を除いた 16 進 64 桁。
    #[must_use]
    pub fn hex(&self) -> &str {
        &self.0[PREFIX.len()..]
    }
}

// ---------------------------------------------------------------------------
// 内容版の投影 — ドメインの値だけを、正準ダイジェストの入力へ写す
// ---------------------------------------------------------------------------

fn text(value: &str) -> JsonValue {
    JsonValue::String(value.to_string())
}

fn optional_text(value: Option<&str>) -> JsonValue {
    value.map_or(JsonValue::Null, text)
}

fn texts(values: &[String]) -> JsonValue {
    JsonValue::Array(values.iter().map(|value| text(value)).collect())
}

fn project_node(node: &StageNode) -> JsonValue {
    let mut members = ObjectMembers::new();
    members.insert("slug", text(node.slug().as_str()));
    members.insert("number", text(node.number().as_str()));
    members.insert("name", text(node.name()));
    members.insert("phase", text(node.phase().as_str()));
    members.insert("execution", text(node.execution().as_str()));
    members.insert("condition", text(node.condition()));
    members.insert("lead_agent", text(node.lead_agent()));
    members.insert("support_agents", texts(node.support_agents()));
    members.insert("mode", text(node.mode().as_str()));
    members.insert("for_each", optional_text(node.for_each()));
    members.insert(
        "workspace_requires",
        JsonValue::Bool(node.workspace_requires()),
    );
    members.insert("produces", texts(node.produces()));
    members.insert("optional_produces", texts(node.optional_produces()));
    members.insert(
        "produces_kinds",
        JsonValue::Array(
            node.produces_kinds()
                .iter()
                .map(|(kind, artifacts)| JsonValue::Array(vec![text(kind), texts(artifacts)]))
                .collect(),
        ),
    );
    members.insert(
        "consumes",
        JsonValue::Array(
            node.consumes()
                .iter()
                .map(|consume| {
                    let mut decl = ObjectMembers::new();
                    decl.insert("artifact", text(consume.artifact()));
                    decl.insert("required", JsonValue::Bool(consume.required()));
                    decl.insert(
                        "conditional_on",
                        optional_text(consume.conditional_on().map(BrownfieldGreenfield::as_str)),
                    );
                    JsonValue::Object(decl)
                })
                .collect(),
        ),
    );
    members.insert(
        "requires_stage",
        JsonValue::Array(
            node.requires_stage()
                .iter()
                .map(|slug| text(slug.as_str()))
                .collect(),
        ),
    );
    members.insert("sensors", texts(node.sensors()));
    members.insert("scopes", texts(node.scopes()));
    members.insert("reviewer", optional_text(node.reviewer()));
    members.insert(
        "reviewer_max_iterations",
        node.reviewer_max_iterations().map_or(JsonValue::Null, |n| {
            JsonValue::Number(Number::PosInt(u64::from(n)))
        }),
    );
    members.insert(
        "review_class",
        optional_text(node.review_class().map(ReviewClass::as_str)),
    );
    members.insert(
        "summary_confirmation",
        optional_text(node.summary_confirmation()),
    );
    members.insert("plugin", optional_text(node.plugin()));
    members.insert(
        "enabled",
        node.enabled().map_or(JsonValue::Null, JsonValue::Bool),
    );
    members.insert("inputs", text(node.inputs()));
    members.insert("outputs", text(node.outputs()));
    members.insert(
        "rules_in_context",
        JsonValue::Array(
            node.rules_in_context()
                .iter()
                .map(|rule| {
                    let mut entry = ObjectMembers::new();
                    entry.insert("path", text(rule.path()));
                    entry.insert("scope", text(rule.scope().as_str()));
                    JsonValue::Object(entry)
                })
                .collect(),
        ),
    );
    members.insert(
        "sensors_applicable",
        JsonValue::Array(
            node.sensors_applicable()
                .iter()
                .map(|sensor| {
                    let mut entry = ObjectMembers::new();
                    entry.insert("id", text(sensor.id()));
                    entry.insert("path", text(sensor.path()));
                    entry.insert("matches", optional_text(sensor.matches()));
                    JsonValue::Object(entry)
                })
                .collect(),
        ),
    );
    JsonValue::Object(members)
}

fn project_grid(grid: &ScopeGrid) -> JsonValue {
    let mut columns = ObjectMembers::new();
    for (scope, column) in grid.columns() {
        let mut cells = ObjectMembers::new();
        for (slug, action) in column {
            cells.insert(slug.as_str(), text(action.as_str()));
        }
        columns.insert(scope.as_str(), JsonValue::Object(cells));
    }
    JsonValue::Object(columns)
}

/// 辞書のキー (`valid_scopes` / `scope_metadata` が観測するスコープ識別子) とメタデータの両方を
/// 投影する — 同じメタデータを別のキーで持つ辞書は別の内容である。
fn project_scope(name: &str, metadata: &ScopeMetadata) -> JsonValue {
    let mut members = ObjectMembers::new();
    members.insert("key", text(name));
    members.insert("name", text(metadata.name()));
    members.insert("depth", optional_text(metadata.depth()));
    members.insert("keywords", texts(metadata.keywords()));
    members.insert(
        "skeleton",
        optional_text(metadata.skeleton().map(|s| s.as_str())),
    );
    members.insert(
        "review_cap",
        optional_text(metadata.review_cap().map(|c| c.as_str())),
    );
    members.insert(
        "freeform_default",
        JsonValue::Bool(metadata.freeform_default()),
    );
    JsonValue::Object(members)
}

impl TryFrom<String> for DefinitionRevision {
    type Error = DefinitionRevisionError;

    fn try_from(value: String) -> Result<DefinitionRevision, DefinitionRevisionError> {
        DefinitionRevision::parse(&value)
    }
}

impl fmt::Display for DefinitionRevision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// canon-json の正準族 `Digest::rendered()` が返す形の代表値。
    const SAMPLE: &str = "sha256:303d9bb7b5d777d54a6761be9ed154d85d5bb3f2d6b9cce02f71f4ed1b3a4ff3";

    #[test]
    fn parse_round_trips_a_canonical_digest_rendering() {
        let revision = DefinitionRevision::parse(SAMPLE).unwrap();
        assert_eq!(revision.as_str(), SAMPLE);
        assert_eq!(revision.to_string(), SAMPLE);
        assert_eq!(
            revision.hex(),
            "303d9bb7b5d777d54a6761be9ed154d85d5bb3f2d6b9cce02f71f4ed1b3a4ff3"
        );
    }

    #[test]
    fn the_all_zero_digest_used_by_synthetic_fixtures_is_accepted() {
        let raw = format!("sha256:{}", "0".repeat(HEX_LEN));
        assert_eq!(DefinitionRevision::parse(&raw).unwrap().as_str(), raw);
    }

    #[test]
    fn a_bare_hex_digest_is_rejected_because_the_family_is_part_of_the_form() {
        // 非正準族 (`hash_compact`) は生 hex を返す。取り違えを型で止める。
        let bare = "303d9bb7b5d777d54a6761be9ed154d85d5bb3f2d6b9cce02f71f4ed1b3a4ff3";
        assert_eq!(
            DefinitionRevision::parse(bare),
            Err(DefinitionRevisionError::MissingPrefix)
        );
        assert_eq!(
            DefinitionRevision::parse(""),
            Err(DefinitionRevisionError::MissingPrefix)
        );
        assert_eq!(
            DefinitionRevision::parse("md5:abcd"),
            Err(DefinitionRevisionError::MissingPrefix)
        );
    }

    #[test]
    fn a_digest_of_the_wrong_width_is_rejected() {
        assert_eq!(
            DefinitionRevision::parse("sha256:abc"),
            Err(DefinitionRevisionError::InvalidLength { actual: 3 })
        );
        let too_long = format!("sha256:{}", "a".repeat(HEX_LEN + 1));
        assert_eq!(
            DefinitionRevision::parse(&too_long),
            Err(DefinitionRevisionError::InvalidLength {
                actual: HEX_LEN + 1
            })
        );
    }

    #[test]
    fn uppercase_hex_and_non_hex_characters_are_rejected() {
        // canon-json は小文字 hex を返すので、大文字は「別経路で作った値」の印。
        let upper = format!("sha256:{}", "A".repeat(HEX_LEN));
        assert_eq!(
            DefinitionRevision::parse(&upper),
            Err(DefinitionRevisionError::InvalidHexDigit('A'))
        );
        let with_g = format!("sha256:g{}", "0".repeat(HEX_LEN - 1));
        assert_eq!(
            DefinitionRevision::parse(&with_g),
            Err(DefinitionRevisionError::InvalidHexDigit('g'))
        );
    }

    #[test]
    fn ordering_and_equality_follow_the_raw_rendering() {
        let a = DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(HEX_LEN))).unwrap();
        let b = DefinitionRevision::parse(&format!("sha256:{}", "1".repeat(HEX_LEN))).unwrap();
        assert!(a < b);
        assert_eq!(a, DefinitionRevision::parse(a.as_str()).unwrap());
        assert_ne!(a, b);
    }

    #[test]
    fn the_rejection_carries_material_not_wording() {
        assert_eq!(
            DefinitionRevisionError::MissingPrefix.to_string(),
            "missing sha256: prefix"
        );
        assert_eq!(
            DefinitionRevisionError::InvalidLength { actual: 3 }.to_string(),
            "expected 64 hex digits, got 3"
        );
        assert_eq!(
            DefinitionRevisionError::InvalidHexDigit('A').to_string(),
            "not a lowercase hex digit: 'A'"
        );
    }
}

#[cfg(test)]
mod content_tests {
    use super::*;

    use crate::workflow_definition::{
        ExecutionKind, PhaseId, StageGraph, StageMode, StageNodeBuilder, StageNumber, StageSlug,
    };

    fn graph() -> StageGraph {
        StageGraph::new(vec![
            StageNodeBuilder::new(
                StageSlug::parse("state-init").expect("slug"),
                StageNumber::parse("0.1").expect("番号"),
                "State Init".to_string(),
                PhaseId::Initialization,
                ExecutionKind::Always,
                StageMode::Inline,
            )
            .scopes(vec!["classic".to_string()])
            .build(),
        ])
        .expect("グラフ")
    }

    fn scopes_under(key: &str) -> BTreeMap<String, ScopeMetadata> {
        [(
            key.to_string(),
            ScopeMetadata::new("classic").expect("スコープ"),
        )]
        .into_iter()
        .collect()
    }

    #[test]
    fn the_revision_round_trips_through_parse() {
        let graph = graph();
        let grid = ScopeGrid::from_graph(&graph);
        let revision = DefinitionRevision::of_content(&graph, &grid, &scopes_under("classic"));
        assert_eq!(
            DefinitionRevision::parse(revision.as_str()),
            Ok(revision.clone()),
            "正準ダイジェストは受理される綴りそのもの"
        );
    }

    #[test]
    fn the_same_metadata_under_another_key_is_another_content() {
        // 辞書のキーはスコープ識別子 (`valid_scopes` が観測する) なので投影に含める —
        // 落とすと、キーだけを変えた改訂が `Unchanged` に潰れる (CodeRabbit 指摘)。
        let graph = graph();
        let grid = ScopeGrid::from_graph(&graph);
        assert_ne!(
            DefinitionRevision::of_content(&graph, &grid, &scopes_under("alpha")),
            DefinitionRevision::of_content(&graph, &grid, &scopes_under("beta"))
        );
    }
}
