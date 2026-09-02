//! 定義 DTO 群の共有フィクスチャと**ワイヤ形式のゴールデン**。
//!
//! 定義の内容は 5 つの型 (`StageNodeDto` / `ConsumeDeclDto` / `RuleInContextDto` /
//! `SensorRefDto` / `ScopeMetadataDto`) に分かれるので、飽和フィクスチャを 1 か所に置いて
//! 各ファイルのインラインテストから共有する (同じ材料を 3 度書かない)。
//!
//! ゴールデンはここが持つ — 行のバイトは**変種をまたいだ 1 つの契約**であり、型ごとの
//! ファイルに割ると「全体としてどう見えるか」が読めなくなる。書く側との一致は
//! `modules/app/aidlc/tests/journal_protocol_conformance.rs` が両側の直列化結果を
//! 突き合わせて固定する (逐語の写しではなく実測の一致を見る)。

#![allow(
    clippy::panic,
    reason = "想定外ケースの即時失敗はテストの検証手段である (house style)"
)]

use std::collections::BTreeMap;

use core_command_domain::workflow_definition::{
    BrownfieldGreenfield, ConsumeDecl, Defined, DefinitionRevision, ExecutionKind, PhaseId,
    Redefined, ReviewCapValue, ReviewClass, RuleInContext, RuleScope, ScopeGrid, ScopeMetadata,
    SensorRef, SkeletonDefault, StageGraph, StageMode, StageNode, StageNodeBuilder, StageNumber,
    StageSlug, WorkflowDefinitionEvent, WorkflowDefinitionId,
};

use super::dto_decode_error::DtoDecodeError;
use super::workflow_definition_event_dto::WorkflowDefinitionEventDto;

/// **任意フィールドを 1 つ残らず埋めた**ステージ。
///
/// 配布物 (`stage-graph.json`) には `plugin` も `enabled` も現れないので、ゴールデン
/// パリティの往復ではこの 2 つの写像が通らない。行のバイトを読むのはこの層なので、
/// 「出荷データに出ないフィールド」こそ意図的に埋めて往復させる。
pub(super) fn saturated_node() -> StageNode {
    StageNodeBuilder::new(
        StageSlug::parse("code-generation").unwrap(),
        StageNumber::parse("3.1").unwrap(),
        "Code Generation".to_string(),
        PhaseId::Construction,
        ExecutionKind::Conditional,
        StageMode::Mob,
    )
    .condition("brownfield".to_string())
    .lead_agent("developer".to_string())
    .support_agents(vec!["quality".to_string()])
    .for_each("unit".to_string())
    .workspace_requires(true)
    .produces(vec!["code".to_string()])
    .optional_produces(vec!["notes".to_string()])
    .produces_kinds(vec![("code".to_string(), vec!["rust".to_string()])])
    .consumes(vec![ConsumeDecl::new(
        "design",
        true,
        Some(BrownfieldGreenfield::Brownfield),
    )])
    .requires_stage(vec![StageSlug::parse("domain-design").unwrap()])
    .sensors(vec!["linter".to_string()])
    .scopes(vec!["feature".to_string()])
    .reviewer("architecture-reviewer".to_string())
    .reviewer_max_iterations(2)
    .review_class(ReviewClass::Adversarial)
    .summary_confirmation("required".to_string())
    .plugin("acme".to_string())
    .enabled(false)
    .inputs("design".to_string())
    .outputs("code".to_string())
    .rules_in_context(vec![RuleInContext::new("org.md", RuleScope::Org)])
    .sensors_applicable(vec![SensorRef::new(
        "linter",
        "sensors/linter.md",
        Some("*.rs".to_string()),
    )])
    .build()
}

/// **任意フィールドを 1 つ残らず埋めた**スコープカタログ。
pub(super) fn saturated_scopes() -> BTreeMap<String, ScopeMetadata> {
    let metadata = ScopeMetadata::new("feature")
        .unwrap()
        .with_depth("standard".to_string())
        .with_keywords(vec!["api".to_string(), "endpoint".to_string()])
        .with_skeleton(SkeletonDefault::On)
        .with_review_cap(ReviewCapValue::Advisory)
        .with_freeform_default(true);
    [("feature".to_string(), metadata)].into_iter().collect()
}

/// 内容の 3 入力 (飽和ノード 1 件のグラフ・そこから導いたグリッド・飽和スコープ)。
pub(super) fn content() -> (StageGraph, ScopeGrid, BTreeMap<String, ScopeMetadata>) {
    let graph = StageGraph::new(vec![saturated_node()]).unwrap();
    let grid = ScopeGrid::from_graph(&graph);
    (graph, grid, saturated_scopes())
}

/// フィクスチャの系譜 ID。
pub(super) fn definition_id() -> WorkflowDefinitionId {
    WorkflowDefinitionId::parse("claude").unwrap()
}

/// フィクスチャの内容版 (同じ文字で埋めた 64 桁)。
pub(super) fn revision(fill: char) -> DefinitionRevision {
    DefinitionRevision::parse(&format!("sha256:{}", fill.to_string().repeat(64))).unwrap()
}

/// 誕生イベント。
pub(super) fn defined_event() -> WorkflowDefinitionEvent {
    let (graph, grid, scopes) = content();
    WorkflowDefinitionEvent::Defined(Defined::new(
        definition_id(),
        revision('0'),
        graph,
        grid,
        scopes,
    ))
}

/// 改訂イベント (系譜 ID を運ばない)。
pub(super) fn redefined_event() -> WorkflowDefinitionEvent {
    let (graph, grid, scopes) = content();
    WorkflowDefinitionEvent::Redefined(Redefined::new(revision('1'), graph, grid, scopes))
}

/// 誕生行のワイヤ形式 (逐語)。
const DEFINED_GOLDEN: &str = concat!(
    r#"{"Defined":{"id":"claude","revision":"sha256:0000000000000000000000000000000000000000000000000000000000000000","content":{"#,
    r#""graph":[{"slug":"code-generation","number":"3.1","name":"Code Generation","phase":"Construction","execution":"Conditional","mode":"Mob","condition":"brownfield","lead_agent":"developer","support_agents":["quality"],"for_each":"unit","workspace_requires":true,"produces":["code"],"optional_produces":["notes"],"produces_kinds":{"code":["rust"]},"consumes":[{"artifact":"design","required":true,"conditional_on":"brownfield"}],"requires_stage":["domain-design"],"sensors":["linter"],"scopes":["feature"],"reviewer":"architecture-reviewer","reviewer_max_iterations":2,"review_class":"Adversarial","summary_confirmation":"required","plugin":"acme","enabled":false,"inputs":"design","outputs":"code","rules_in_context":[{"path":"org.md","scope":"Org"}],"sensors_applicable":[{"id":"linter","path":"sensors/linter.md","matches":"*.rs"}]}],"#,
    r#""grid":{"feature":{"code-generation":"Execute"}},"#,
    r#""scopes":[{"name":"feature","depth":"standard","keywords":["api","endpoint"],"skeleton":"On","review_cap":"Advisory","freeform_default":true}]}}}"#,
);

/// 改訂行のワイヤ形式 (逐語 — 系譜 ID の欄が無いことが誕生との差)。
const REDEFINED_GOLDEN: &str = concat!(
    r#"{"Redefined":{"revision":"sha256:1111111111111111111111111111111111111111111111111111111111111111","content":{"#,
    r#""graph":[{"slug":"code-generation","number":"3.1","name":"Code Generation","phase":"Construction","execution":"Conditional","mode":"Mob","condition":"brownfield","lead_agent":"developer","support_agents":["quality"],"for_each":"unit","workspace_requires":true,"produces":["code"],"optional_produces":["notes"],"produces_kinds":{"code":["rust"]},"consumes":[{"artifact":"design","required":true,"conditional_on":"brownfield"}],"requires_stage":["domain-design"],"sensors":["linter"],"scopes":["feature"],"reviewer":"architecture-reviewer","reviewer_max_iterations":2,"review_class":"Adversarial","summary_confirmation":"required","plugin":"acme","enabled":false,"inputs":"design","outputs":"code","rules_in_context":[{"path":"org.md","scope":"Org"}],"sensors_applicable":[{"id":"linter","path":"sensors/linter.md","matches":"*.rs"}]}],"#,
    r#""grid":{"feature":{"code-generation":"Execute"}},"#,
    r#""scopes":[{"name":"feature","depth":"standard","keywords":["api","endpoint"],"skeleton":"On","review_cap":"Advisory","freeform_default":true}]}}}"#,
);

#[expect(
    clippy::disallowed_methods,
    reason = "契約 JSON ではなくワイヤ形式そのものの逐語固定 (BR1.7 の射程外)"
)]
#[test]
fn the_birth_row_is_written_verbatim() {
    // 行のバイトそのものの逐語固定。欄名・並び・閉集合の綴りのどれが動いてもここで割れる。
    let json = serde_json::to_string(&WorkflowDefinitionEventDto::of(&defined_event())).unwrap();
    assert_eq!(json, DEFINED_GOLDEN);
}

#[expect(
    clippy::disallowed_methods,
    reason = "契約 JSON ではなくワイヤ形式そのものの逐語固定 (BR1.7 の射程外)"
)]
#[test]
fn the_revision_row_is_written_verbatim() {
    // 改訂は系譜 ID を運ばない — 欄が 1 つ少ないことが誕生との唯一の差である。
    let json = serde_json::to_string(&WorkflowDefinitionEventDto::of(&redefined_event())).unwrap();
    assert_eq!(json, REDEFINED_GOLDEN);
    assert!(
        json.starts_with(r#"{"Redefined":{"revision":"#),
        "改訂は系譜 ID を運ばない (誕生と違い先頭が revision): {json}"
    );
}

#[test]
fn both_variants_survive_the_round_trip() {
    for event in [defined_event(), redefined_event()] {
        let decoded = WorkflowDefinitionEventDto::of(&event).to_domain().unwrap();
        assert_eq!(decoded, event);
    }
}

#[test]
fn the_golden_rows_decode_back_into_the_same_events() {
    // ゴールデン文字列側からも往復する — 「書けるが読めない」を許さない。
    for (golden, event) in [
        (DEFINED_GOLDEN, defined_event()),
        (REDEFINED_GOLDEN, redefined_event()),
    ] {
        let dto: WorkflowDefinitionEventDto = serde_json::from_str(golden).unwrap();
        assert_eq!(dto.to_domain().unwrap(), event);
    }
}

#[test]
fn a_scope_catalog_that_names_the_same_scope_twice_is_refused() {
    // カタログはワイヤでは**並び**であり、ドメインでは名前を鍵にした写像である。黙って
    // 後勝ちで畳むと、行が 2 件あるのに 1 件しか載らない — どちらが載ったかも読めない。
    // 畳めない並びは解釈せず拒む (`StageGraph` が重複 slug を拒むのと同じ規律)。
    let duplicated = DEFINED_GOLDEN.replace(
        r#""scopes":[{"name":"feature","depth":"standard""#,
        r#""scopes":[{"name":"feature","depth":"express","keywords":[],"skeleton":null,"review_cap":null,"freeform_default":false},{"name":"feature","depth":"standard""#,
    );
    assert_ne!(duplicated, DEFINED_GOLDEN, "置換が効いている");

    let dto: WorkflowDefinitionEventDto =
        serde_json::from_str(&duplicated).expect("JSON としては読める");
    assert_eq!(
        dto.to_domain().expect_err("同名スコープ 2 件は畳めない"),
        DtoDecodeError::malformed("scope_name", "feature".to_string())
    );
}

#[test]
fn a_row_with_a_broken_spelling_is_refused_field_by_field() {
    // 閉集合外の綴り・文法外の識別子は、どの欄で落ちたかを材料に載せて拒む。
    // ワイヤ形式の JSON を直接壊す — 実装に破壊用のフックを開けない。
    for (from, to, field) in [
        (r#""id":"claude""#, r#""id":"  ""#, "id"),
        (r#""revision":"sha256:"#, r#""revision":"nope:"#, "revision"),
        (r#""mode":"Mob""#, r#""mode":"mob""#, "mode"),
        (
            r#""execution":"Conditional""#,
            r#""execution":"CONDITIONAL""#,
            "execution",
        ),
        (
            r#""phase":"Construction""#,
            r#""phase":"construction""#,
            "phase",
        ),
        (
            r#""review_class":"Adversarial""#,
            r#""review_class":"adversarial""#,
            "review_class",
        ),
        (r#""scope":"Org""#, r#""scope":"org""#, "rule_scope"),
        (
            r#""conditional_on":"brownfield""#,
            r#""conditional_on":"Brownfield""#,
            "project_type",
        ),
        (r#""skeleton":"On""#, r#""skeleton":"on""#, "skeleton"),
        (
            r#""review_cap":"Advisory""#,
            r#""review_cap":"advisory""#,
            "review_cap",
        ),
        (r#""name":"feature""#, r#""name":"""#, "scope_name"),
        (r#""number":"3.1""#, r#""number":"three""#, "number"),
        (
            r#""slug":"code-generation""#,
            r#""slug":"Code Generation""#,
            "slug",
        ),
        (
            r#""requires_stage":["domain-design"]"#,
            r#""requires_stage":["Domain Design"]"#,
            "requires_stage",
        ),
        (
            r#"{"code-generation":"Execute"}"#,
            r#"{"Bad Slug":"Execute"}"#,
            "grid_slug",
        ),
    ] {
        let broken = DEFINED_GOLDEN.replacen(from, to, 1);
        assert_ne!(broken, DEFINED_GOLDEN, "置換が効いていない: {from}");
        let dto: WorkflowDefinitionEventDto = serde_json::from_str(&broken).unwrap();
        match dto.to_domain().unwrap_err() {
            DtoDecodeError::Malformed { field: got, .. } => assert_eq!(got, field),
            other => panic!("{field}: 綴りの拒否ではない — {other:?}"),
        }
    }
}

#[test]
fn a_graph_that_breaks_its_invariant_is_refused_as_an_invariant_violation() {
    // slug の重複はグラフの不変条件違反 — 綴りの問題ではないので材料は欄名を持たない。
    let node_start = DEFINED_GOLDEN
        .find(r#"{"slug":"code-generation""#)
        .expect("ノードが在る");
    let node_end = DEFINED_GOLDEN
        .find(r#"],"grid""#)
        .expect("グラフ配列の終端");
    let node = DEFINED_GOLDEN
        .get(node_start..node_end)
        .expect("ノードの範囲")
        .to_string();
    let duplicated = DEFINED_GOLDEN.replacen(&node, &format!("{node},{node}"), 1);

    let dto: WorkflowDefinitionEventDto = serde_json::from_str(&duplicated).unwrap();
    assert_eq!(
        dto.to_domain().unwrap_err(),
        DtoDecodeError::InvariantViolation
    );
}
