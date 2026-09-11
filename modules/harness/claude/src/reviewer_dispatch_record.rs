//! 差し向け記録 `<record>/.aidlc-reviewer-dispatch.json` の読取り。
use core_command_domain::orchestration::{
    ExemptPaths, ReviewedStage, ReviewedUnit, ReviewerDispatch, ScopeToken,
};
use serde_json::Value;

/// 指揮者が置いた差し向け記録を読む。形が違えば `None` (呼出側は強制を諦めて通す)。
///
/// upstream `parseDispatchRecord` (`aidlc-reviewer-scope.ts:673-690`) は 4 つの鍵を
/// すべて要求する — `reviewer` と `unit` は非空の文字列、`stage` は文字列、`exempt` は
/// 文字列の配列である。**形だけを見て文法は見ない** — `stage` の綴り・`unit` の区切りは
/// 検査せず、その値を読み取り範囲と `REVIEWER_SCOPE_BLOCKED` の `Stage` / `Unit` へ逐語で
/// 運ぶ ([`ReviewedStage`] / [`ReviewedUnit`]。裁定 2026-09-10 Q1 = A)。
#[must_use]
pub fn parse_reviewer_dispatch(raw: &str) -> Option<ReviewerDispatch> {
    let value = serde_json::from_str::<Value>(raw).ok()?;
    let record = value.as_object()?;
    let reviewer = record.get("reviewer").and_then(Value::as_str)?;
    if reviewer.is_empty() {
        return None;
    }
    let unit = record.get("unit").and_then(Value::as_str)?;
    let stage = record.get("stage").and_then(Value::as_str)?;
    let exempt = record.get("exempt").and_then(Value::as_array)?;
    // upstream は配列の要素が 1 つでも文字列でなければ記録全体を拒否する。
    let mut paths = Vec::with_capacity(exempt.len());
    for entry in exempt {
        let entry = entry.as_str()?;
        // 空の許可経路は照合に効かない (upstream も `normalizeResolved` が落とす)。
        if let Ok(token) = ScopeToken::parse(entry) {
            paths.push(token);
        }
    }
    Some(ReviewerDispatch::new(
        reviewer.to_string(),
        ReviewedStage::new(stage.to_string()),
        ReviewedUnit::parse(unit).ok()?,
        ExemptPaths::new(paths),
    ))
}

#[cfg(test)]
mod tests {
    use super::parse_reviewer_dispatch;

    const GOOD: &str = r#"{"reviewer":"aidlc-architecture-reviewer-agent","stage":"functional-design","unit":"u1-alpha","exempt":["/r/a.md","/r/construction/u2/b.md"]}"#;

    #[test]
    fn a_well_formed_record_carries_the_four_facts() {
        let dispatch = parse_reviewer_dispatch(GOOD).unwrap();
        assert_eq!(dispatch.reviewer(), "aidlc-architecture-reviewer-agent");
        assert_eq!(dispatch.stage().as_str(), "functional-design");
        assert_eq!(dispatch.unit().as_str(), "u1-alpha");
        assert_eq!(dispatch.exempt().len(), 2);
    }

    #[test]
    fn an_empty_exempt_list_is_valid_and_grants_nothing() {
        let dispatch = parse_reviewer_dispatch(
            r#"{"reviewer":"r","stage":"functional-design","unit":"u1","exempt":[]}"#,
        )
        .unwrap();
        assert!(dispatch.exempt().is_empty());
    }

    #[test]
    fn a_missing_or_mistyped_field_makes_the_record_unreadable() {
        for raw in [
            "not json",
            "null",
            "[]",
            r#""text""#,
            // exempt 欠落 — upstream も配列を要求する
            r#"{"reviewer":"r","stage":"functional-design","unit":"u1"}"#,
            // exempt の要素が文字列でない
            r#"{"reviewer":"r","stage":"functional-design","unit":"u1","exempt":[1]}"#,
            // reviewer が空
            r#"{"reviewer":"","stage":"functional-design","unit":"u1","exempt":[]}"#,
            // unit が空
            r#"{"reviewer":"r","stage":"functional-design","unit":"","exempt":[]}"#,
            // stage が文字列でない
            r#"{"reviewer":"r","stage":1,"unit":"u1","exempt":[]}"#,
        ] {
            assert!(parse_reviewer_dispatch(raw).is_none(), "{raw}");
        }
    }

    #[test]
    fn the_stage_and_unit_spellings_are_carried_verbatim_as_upstream_does() {
        // upstream `parseDispatchRecord` は `stage` が文字列であること、`unit` が非空で
        // あることしか見ない。空の stage・slug でない綴り・区切りを含む unit もそのまま受け、
        // `Stage` / `Unit` を監査と拒否文言へ逐語で載せる (hook-differential c24 / c25)。
        let empty_stage =
            parse_reviewer_dispatch(r#"{"reviewer":"r","stage":"","unit":"u1","exempt":[]}"#)
                .expect("空の stage は文字列である");
        assert_eq!(empty_stage.stage().as_str(), "");
        let spaced = parse_reviewer_dispatch(
            r#"{"reviewer":"r","stage":"Functional Design","unit":"u1","exempt":[]}"#,
        )
        .expect("slug でない綴りも文字列である");
        assert_eq!(spaced.stage().as_str(), "Functional Design");
        let nested = parse_reviewer_dispatch(
            r#"{"reviewer":"r","stage":"functional-design","unit":"a/b","exempt":[]}"#,
        )
        .expect("区切りを含む unit も非空である");
        assert_eq!(nested.unit().as_str(), "a/b");
    }
}
