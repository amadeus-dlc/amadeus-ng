//! 明示した規則と作業範囲から解決するテスト契約。
use super::{TestingContext, TestingPostureError, TestingSections};
use core_infrastructure::canon_json::{JsonValue, Number, ObjectMembers, hash_canonical};
mod classification;
mod markdown;
use classification::Classification;
const LAYERS: &[&str] = &[
    "Data model / database behavior",
    "Repository / data access",
    "Business logic",
    "API / endpoint",
    "Frontend behavior",
];
/// 入力と実行順序を結び付けた契約。保存形式の知識は持たない。
#[derive(Debug, Clone, PartialEq)]
pub struct TestingPosture {
    value: JsonValue,
}
impl TestingPosture {
    /// 階層ごとのテスト方針を解決する。
    /// # Errors
    /// 方法論が不正、またはチームとプロジェクトの規則が矛盾する場合。
    pub fn resolve(
        sections: &TestingSections,
        context: &TestingContext,
    ) -> Result<Self, TestingPostureError> {
        let mut selected = ("fallback", Classification::fallback());
        let mut notes = Vec::new();
        for (layer, section) in [
            ("org", sections.org()),
            ("team", sections.team()),
            ("project", sections.project()),
        ] {
            let (notes_text, classified) = markdown::projections(section);
            if !notes_text.is_empty() {
                notes.push(object([
                    ("layer", text(layer)),
                    ("text", text(&notes_text)),
                ]));
            }
            if let Some(classified) = Classification::parse(&classified)? {
                if layer == "project"
                    && selected.0 == "team"
                    && !classified.specializes(&selected.1)
                {
                    return Err(TestingPostureError::new(format!(
                        "Testing Posture conflict: project methodology \"{}\" contradicts team methodology \"{}\". Revise the narrower rule; strict-additive memory does not permit runtime override.",
                        classified.methodology(),
                        selected.1.methodology()
                    )));
                }
                selected = (layer, classified);
            }
        }
        let input = object([
            (
                "sections",
                object([
                    ("org", text(sections.org())),
                    ("team", text(sections.team())),
                    ("project", text(sections.project())),
                ]),
            ),
            ("scope", text(context.scope())),
            ("test_strategy", text(context.strategy())),
            ("project_type", text(context.project_type())),
        ]);
        let mut body = ObjectMembers::new();
        body.insert("version", JsonValue::Number(Number::PosInt(1)));
        body.insert("methodology", text(selected.1.methodology()));
        body.insert("source", text(selected.0));
        body.insert("ordering", text(selected.1.ordering()));
        body.insert("scope", text(context.scope()));
        body.insert("test_strategy", text(context.strategy()));
        body.insert("project_type", text(context.project_type()));
        body.insert("applicable_notes", JsonValue::Array(notes));
        body.insert("obligations", obligations(context)?);
        body.insert(
            "plan_profile",
            profile(context, selected.1.methodology(), selected.1.ordering()),
        );
        body.insert("input_sha256", text(&hash_canonical(&input).rendered()));
        let digest = hash_canonical(&JsonValue::Object(body.clone())).rendered();
        body.insert("contract_sha256", text(&digest));
        Ok(Self {
            value: JsonValue::Object(body),
        })
    }
    /// 解決済みの契約値。
    #[must_use]
    pub const fn value(&self) -> &JsonValue {
        &self.value
    }
}
fn text(value: &str) -> JsonValue {
    JsonValue::String(value.to_string())
}
fn strings(values: &[&str]) -> JsonValue {
    JsonValue::Array(values.iter().map(|value| text(value)).collect())
}
fn object<const N: usize>(entries: [(&str, JsonValue); N]) -> JsonValue {
    let mut fields = ObjectMembers::new();
    for (key, value) in entries {
        fields.insert(key, value);
    }
    JsonValue::Object(fields)
}
fn obligations(context: &TestingContext) -> Result<JsonValue, TestingPostureError> {
    let volume: &[&str] = match context.strategy() {
        "minimal" => &[
            "One verifiable test per requirement at the narrowest effective level.",
            "At least one happy-path unit test per component.",
            "Unit tests are the default; a bugfix/security scope floor may require an integration or E2E regression when that is the narrowest level that reproduces the defect.",
        ],
        "standard" => &[
            "Five to eight tests per component.",
            "Unit tests plus integration tests for key boundaries.",
            "Add E2E, performance, or security tests when requirements demand them.",
        ],
        "comprehensive" => &[
            "Ten to fifteen tests per component.",
            "Unit, integration, and E2E tests.",
            "Add performance and security tests when NFRs demand them.",
        ],
        _ => return Err(TestingPostureError::new("Invalid normalized test strategy")),
    };
    let floor: &[&str] = match context.scope().trim().to_lowercase().as_str() {
        "mvp" | "enterprise" | "feature" | "infra" => &[
            "Meet an 80% line-coverage floor.",
            "Run the selected tests in CI before merge.",
        ],
        "bugfix" | "security-patch" => &[
            "Include a targeted regression for the bug or vulnerability.",
            "Keep the existing test suite green.",
        ],
        _ => &[
            "Keep the existing test suite green.",
            "This scope adds no extra new-test floor beyond the selected test strategy.",
        ],
    };
    Ok(object([
        ("strategy", text(context.strategy())),
        ("strategy_volume", strings(volume)),
        ("scope_floor", strings(floor)),
        (
            "combination_rule",
            text(
                "Apply every selected-strategy obligation and every scope-floor obligation; neither replaces the other, and a targeted scope regression may add the narrowest necessary test type beyond the strategy default.",
            ),
        ),
    ]))
}
fn profile(context: &TestingContext, methodology: &str, ordering: &str) -> JsonValue {
    let runner = if context.project_type() == "greenfield" {
        "Bootstrap the minimal test runner/configuration and record the exact unit-scoped command."
    } else {
        "Verify the existing test runner/configuration and record the exact unit-scoped command."
    };
    let mut steps = vec![
        text("Project structure and production configuration skeleton."),
        text(runner),
    ];
    match methodology {
        "bdd" => steps.extend([
            text("Behavior scenarios - define executable examples for the observable feature slice before implementation."),
            text("Feature slice - implement the required data, repository, business, API, and frontend layers."),
            text("Behavior scenarios - run the scenarios until they pass."),
            text("Feature slice - refactor while the scenarios stay green."),
        ]),
        "atdd" => steps.extend([
            text("Acceptance Red - write executable acceptance tests for the complete feature before implementation."),
            text("Feature implementation - implement the required layers against the acceptance contract."),
            text("Acceptance Green - run the acceptance tests until they pass."),
            text("Feature Refactor - improve the cross-layer implementation while acceptance stays green."),
        ]),
        "custom" => steps.extend([
            text(&format!("Custom ordering - {ordering}")),
            text("Implementation and tests - preserve that exact ordering; do not convert it to layer-local TDD."),
        ]),
        _ => {
    for layer in LAYERS {
        if methodology == "tdd" {
            steps.push(text(&format!("{layer} - Red: write the failing tests and record the failing command output.")));
            steps.push(text(&format!("{layer} - Green: implement only enough behavior to pass.")));
            steps.push(text(&format!("{layer} - Refactor: improve the implementation while tests stay green.")));
        } else {
            steps.push(text(&format!("{layer} - implement.")));
            steps.push(text(&format!("{layer} - write and run its tests after implementation.")));
        }
    }
        }
    }
    steps.extend([
        text("Environment/build configuration."),
        text("Documentation and traceability."),
    ]);
    object([
        ("methodology", text(methodology)),
        ("runner_step", text(runner)),
        ("runner_ready_before_first_test", JsonValue::Bool(true)),
        ("testable_layers", strings(LAYERS)),
        ("steps", JsonValue::Array(steps)),
    ])
}

pub(super) fn extract_section(content: &str) -> String {
    markdown::extract_section(content)
}
