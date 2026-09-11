//! 固定本家のテスト契約の保存済み観測。
#![allow(clippy::unwrap_used)]
use core_command_domain::orchestration::{TestingContext, TestingPosture, TestingSections};
use core_infrastructure::canon_json::{SerializationProfile, serialize};
#[test]
fn testing_posture_matches_the_saved_upstream_observations() {
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../tests/golden/selfhost-stage1/testing-posture.json"
    ))
    .unwrap();
    assert_eq!(
        corpus
            .get("source")
            .unwrap()
            .get("commit")
            .unwrap()
            .as_str(),
        Some("a277af218f0df7f325d3b8be7b6d90fce2c5bd40")
    );
    let observations = corpus.get("observations").unwrap().as_array().unwrap();
    assert!(!observations.is_empty());
    let mut identifiers = std::collections::BTreeSet::new();
    for case in observations {
        let id = case.get("id").unwrap().as_str().unwrap();
        assert!(identifiers.insert(id));
        let read = |container: &str, name: &str| {
            case.get(container)
                .and_then(|object| object.get(name))
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
        };
        let sections = if case.get("documents").is_some() {
            TestingSections::from_documents(
                read("documents", "org"),
                read("documents", "team"),
                read("documents", "project"),
            )
        } else {
            TestingSections::new(
                read("sections", "org").to_string(),
                read("sections", "team").to_string(),
                read("sections", "project").to_string(),
            )
        };
        let options = &case.get("options").unwrap();
        let actual = TestingPosture::resolve(
            &sections,
            &TestingContext::new(
                options.get("scope").unwrap().as_str().unwrap().to_string(),
                options
                    .get("testStrategy")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_string(),
                options
                    .get("projectType")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_string(),
            ),
        );
        if let Some(error) = case.get("error").unwrap().as_str() {
            assert_eq!(actual.unwrap_err().to_string(), error, "{id}");
        } else {
            assert!(actual.is_ok(), "{id}: {actual:?}");
            let value: serde_json::Value = serde_json::from_str(&serialize(
                actual.unwrap().value(),
                SerializationProfile::ContractCompact,
            ))
            .unwrap();
            assert_eq!(&value, case.get("contract").unwrap(), "{id}");
        }
    }
}

/// team 節の本文だけから解決し、`methodology` / `source` / `ordering` の 3 値を取り出す。
fn resolve_team(section: &str) -> (String, String, String) {
    let posture = TestingPosture::resolve(
        &TestingSections::new(String::new(), section.to_string(), String::new()),
        &TestingContext::new(
            "classic".to_string(),
            "minimal".to_string(),
            "greenfield".to_string(),
        ),
    )
    .unwrap();
    let rendered = serialize(posture.value(), SerializationProfile::ContractCompact);
    let field = |name: &str| {
        let key = format!("\"{name}\":\"");
        let start = rendered.find(&key).unwrap() + key.len();
        rendered[start..].split('"').next().unwrap().to_string()
    };
    (field("methodology"), field("source"), field("ordering"))
}

/// 空の `Methodology:` は構造化された宣言ではなく、語句照合にも掛からなければ既定へ落ちる。
#[test]
fn an_empty_structured_field_is_not_a_declaration() {
    let (methodology, source, ordering) =
        resolve_team("- **Methodology**:\n- **Ordering**: run tests first\n");
    assert_eq!(methodology, "test-after");
    assert_eq!(source, "fallback");
    assert_eq!(
        ordering,
        "Implement each testable layer, then write and run that layer's tests."
    );
}

/// `custom` は Ordering を欠くと本文全体 (空白を畳んだもの) をそのまま順序として保つ。
#[test]
fn a_custom_methodology_without_an_ordering_keeps_the_collapsed_body() {
    let (methodology, source, ordering) =
        resolve_team("- **Methodology**: custom\n\nWe keep a custom ordering: probe   spaces.\n");
    assert_eq!(methodology, "custom");
    assert_eq!(source, "team");
    assert_eq!(
        ordering,
        "- **Methodology**: custom We keep a custom ordering: probe spaces."
    );
    let (methodology, _, ordering) = resolve_team("- **Methodology**: atdd\n");
    assert_eq!(methodology, "atdd");
    assert_eq!(
        ordering,
        "Write executable acceptance tests before implementing the complete feature across its required layers."
    );
}

/// フェンス・インラインコード・HTML コメントの中の宣言は分類対象から外れ、注記には残る。
#[test]
fn code_examples_and_comments_do_not_declare_a_methodology() {
    for (section, expected) in [
        (
            "```\n- **Methodology**: bdd\n```\n- **Methodology**: tdd\n",
            "tdd",
        ),
        // 4 空白の字下げはフェンスではない。
        (
            "    ```\n    - **Methodology**: bdd\n- **Methodology**: tdd\n",
            "bdd",
        ),
        // 短い閉じ記号では閉じない。
        (
            "~~~~\n- **Methodology**: bdd\n~~~\n~~~~\n- **Methodology**: tdd\n",
            "tdd",
        ),
        // 二重バッククォートの中の単独バッククォートは区切りにならない。
        (
            "`` ` `` - **Methodology**: bdd `x` `y\n- **Methodology**: tdd\n",
            "tdd",
        ),
        // バッククォートを含む info 文字列はフェンスを開かない。
        ("``` `\n- **Methodology**: bdd\n", "bdd"),
        // エスケープされたバッククォートはコードを開かない。
        ("\\`code\\` - **Methodology**: tdd\n", "tdd"),
        (
            "- **Methodology**: tdd\n- **Ordering**: red then green\n<!-- - **Methodology**: bdd -->\n",
            "tdd",
        ),
    ] {
        let (methodology, source, _) = resolve_team(section);
        assert_eq!(methodology, expected, "{section:?}");
        assert_eq!(source, "team", "{section:?}");
    }
}
