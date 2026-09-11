//! `DispatchRulesEnvelope` — 規則配送フックの PreToolUse 封筒。
//!
//! 本家 2.7.1 `hooks/aidlc-deliver-stage-rules.ts:47-48,157-258` に対応する。
//! ここが持つのは**封筒の読みと書き戻し**だけで、どの stage を選ぶか・どのファイルを読むか
//! の判断は持たない（それは合成ルート側の責務である）。

use std::path::Path;

use core_infrastructure::canon_json::{
    JsonValue, ObjectMembers, SerializationProfile, parse as parse_json, serialize,
};

use crate::stage_rule_bundle::StageRuleBundle;

/// 本家 `DISPATCH_TOOLS`（小文字で比較する）。
const DISPATCH_TOOLS: [&str; 4] = ["task", "agent", "spawn_agent", "subagent"];
/// 本家 `EXEMPT_AGENTS` — 規則束を付けないエージェント。
const EXEMPT_AGENTS: [&str; 1] = ["aidlc-composer-agent"];
/// 本家 `promptText` / `withPrompt` が見る本文フィールド（この順）。
const PROMPT_FIELDS: [&str; 4] = ["prompt", "message", "description", "task"];
/// 本家 `augmentSingleDispatch` が見るエージェント名フィールド（この順）。
const AGENT_FIELDS: [&str; 4] = ["subagent_type", "agent_type", "agent", "role"];

/// 規則を足す先。書き戻しの位置を封筒の中に閉じ込める。
#[derive(Debug, Clone, PartialEq, Eq)]
enum BriefSlot {
    /// `tool_input.<field>` の文字列を差し替える。
    Field(String),
    /// `tool_input.items` の末尾へ 1 件足す。
    Items,
    /// `tool_input.stages[<index>].prompt_template` を差し替える。
    Stage(usize),
}

/// 規則配送フックが読んだ PreToolUse 入力 1 件。
#[derive(Debug, Clone, PartialEq)]
pub struct DispatchRulesEnvelope {
    tool: String,
    input: ObjectMembers,
    slots: Vec<BriefSlot>,
    briefs: Vec<String>,
}

impl DispatchRulesEnvelope {
    /// 工具名・封筒・対象 brief を同時に確定する（**この型の唯一の構築経路**）。
    const fn new(
        tool: String,
        input: ObjectMembers,
        slots: Vec<BriefSlot>,
        briefs: Vec<String>,
    ) -> DispatchRulesEnvelope {
        DispatchRulesEnvelope {
            tool,
            input,
            slots,
            briefs,
        }
    }

    /// PreToolUse の JSON を読む。読めない入力・dispatch でない工具は brief 0 件になる。
    ///
    /// `agents_dir` はエージェントペルソナの置き場（本家 `agentsDir()`）。名前がその下の
    /// `<name>.md` を指していなければ AI-DLC のエージェントとして扱わない。
    #[must_use]
    pub fn parse(raw: &str, agents_dir: &Path) -> DispatchRulesEnvelope {
        let Ok(JsonValue::Object(root)) = parse_json(raw) else {
            return DispatchRulesEnvelope::new(
                String::new(),
                ObjectMembers::new(),
                Vec::new(),
                Vec::new(),
            );
        };
        let tool = match root.get("tool_name") {
            Some(JsonValue::String(name)) => name.clone(),
            _ => String::new(),
        };
        let input = match root.get("tool_input") {
            Some(JsonValue::Object(members)) => members.clone(),
            _ => ObjectMembers::new(),
        };
        let lowered = tool.to_lowercase();
        if !DISPATCH_TOOLS.contains(&lowered.as_str()) {
            return DispatchRulesEnvelope::new(tool, input, Vec::new(), Vec::new());
        }
        let (slots, briefs) = if lowered == "subagent" {
            stage_briefs(&input, agents_dir)
        } else {
            single_brief(&input, agents_dir)
        };
        DispatchRulesEnvelope::new(tool, input, slots, briefs)
    }

    /// 呼び出された工具名（欠落・非文字列は空文字）。
    #[must_use]
    pub fn tool(&self) -> &str {
        &self.tool
    }

    /// 規則束を足す候補の brief 本文（封筒に現れる順）。
    #[must_use]
    pub fn briefs(&self) -> &[String] {
        &self.briefs
    }

    /// brief ごとの束を当てて、フックが stdout へ出す 1 行を組む。
    ///
    /// `bundles` は [`briefs`](Self::briefs) と同じ順・同じ長さで渡す。1 件も変わらなければ
    /// `None`（本家 `{changed:false}`）。返る文字列に末尾改行は含めない。
    #[must_use]
    pub fn pre_tool_use_output(&self, bundles: &[Option<StageRuleBundle>]) -> Option<String> {
        let mut updated = self.input.clone();
        let mut changed = false;
        for (position, slot) in self.slots.iter().enumerate() {
            let (Some(Some(bundle)), Some(brief)) =
                (bundles.get(position), self.briefs.get(position))
            else {
                continue;
            };
            if bundle.is_empty() || bundle.already_in(brief) {
                continue;
            }
            if apply(&mut updated, slot, brief, &bundle.block()) {
                changed = true;
            }
        }
        if !changed {
            return None;
        }
        let mut hook = ObjectMembers::new();
        hook.insert("hookEventName", JsonValue::String("PreToolUse".to_string()));
        hook.insert("updatedInput", JsonValue::Object(updated));
        let mut root = ObjectMembers::new();
        root.insert("hookSpecificOutput", JsonValue::Object(hook));
        Some(serialize(
            &JsonValue::Object(root),
            SerializationProfile::ContractCompact,
        ))
    }
}

/// 単発 dispatch の brief を取り出す（本家 `augmentSingleDispatch`）。
fn single_brief(input: &ObjectMembers, agents_dir: &Path) -> (Vec<BriefSlot>, Vec<String>) {
    let nothing = (Vec::new(), Vec::new());
    // `??` は `null` と欠落だけを飛ばす — 非文字列が先に居れば、そこで打ち切る。
    let named = AGENT_FIELDS
        .iter()
        .find_map(|field| match input.get(field) {
            None | Some(JsonValue::Null) => None,
            Some(value) => Some(value),
        });
    let Some(JsonValue::String(agent)) = named else {
        return nothing;
    };
    if !is_aidlc_agent(agent, agents_dir) {
        return nothing;
    }
    // 本文フィールドが 1 つでも文字列なら、それが brief である（空でも打ち切る）。
    if let Some((field, body)) = PROMPT_FIELDS
        .iter()
        .find_map(|field| match input.get(field) {
            Some(JsonValue::String(text)) => Some(((*field).to_string(), text.clone())),
            _ => None,
        })
    {
        return if body.is_empty() {
            nothing
        } else {
            (vec![BriefSlot::Field(field)], vec![body])
        };
    }
    let Some(JsonValue::Array(items)) = input.get("items") else {
        return nothing;
    };
    let body = items
        .iter()
        .map(item_text)
        .collect::<Vec<&str>>()
        .join("\n");
    if body.is_empty() {
        nothing
    } else {
        (vec![BriefSlot::Items], vec![body])
    }
}

/// `subagent` の `stages[]` から brief を取り出す（本家 `augmentDispatchRules` の後半）。
fn stage_briefs(input: &ObjectMembers, agents_dir: &Path) -> (Vec<BriefSlot>, Vec<String>) {
    let Some(JsonValue::Array(stages)) = input.get("stages") else {
        return (Vec::new(), Vec::new());
    };
    stages.iter().enumerate().fold(
        (Vec::new(), Vec::new()),
        |(mut slots, mut briefs), (index, stage)| {
            let JsonValue::Object(entry) = stage else {
                return (slots, briefs);
            };
            let Some(JsonValue::String(role)) = entry.get("role") else {
                return (slots, briefs);
            };
            let Some(JsonValue::String(template)) = entry.get("prompt_template") else {
                return (slots, briefs);
            };
            if is_aidlc_agent(role, agents_dir) {
                slots.push(BriefSlot::Stage(index));
                briefs.push(template.clone());
            }
            (slots, briefs)
        },
    )
}

/// `items` の 1 要素が運ぶ本文（`{type,text}` 以外は空文字として数える）。
fn item_text(item: &JsonValue) -> &str {
    match item {
        JsonValue::Object(members) => match members.get("text") {
            Some(JsonValue::String(text)) => text,
            _ => "",
        },
        _ => "",
    }
}

/// 規則束を足す先へ書き戻す。書けたら `true`。
fn apply(input: &mut ObjectMembers, slot: &BriefSlot, brief: &str, block: &str) -> bool {
    match slot {
        BriefSlot::Field(field) => {
            input.insert(field.clone(), JsonValue::String(format!("{brief}{block}")));
            true
        }
        BriefSlot::Items => {
            let Some(JsonValue::Array(items)) = input.get("items") else {
                return false;
            };
            let mut items = items.clone();
            let mut added = ObjectMembers::new();
            added.insert("type", JsonValue::String("text".to_string()));
            added.insert("text", JsonValue::String(block.to_string()));
            items.push(JsonValue::Object(added));
            input.insert("items", JsonValue::Array(items));
            true
        }
        BriefSlot::Stage(index) => {
            let Some(JsonValue::Array(stages)) = input.get("stages") else {
                return false;
            };
            let mut stages = stages.clone();
            let Some(JsonValue::Object(entry)) = stages.get(*index) else {
                return false;
            };
            let mut entry = entry.clone();
            entry.insert(
                "prompt_template",
                JsonValue::String(format!("{brief}{block}")),
            );
            let Some(slot) = stages.get_mut(*index) else {
                return false;
            };
            *slot = JsonValue::Object(entry);
            input.insert("stages", JsonValue::Array(stages));
            true
        }
    }
}

/// AI-DLC のエージェント名か（本家 `isAidlcAgent`）。
///
/// 名前の形・ペルソナファイルの実在・除外リストの 3 つをすべて満たすときだけ真である。
fn is_aidlc_agent(value: &str, agents_dir: &Path) -> bool {
    agent_name_shape(value)
        && !EXEMPT_AGENTS.contains(&value)
        && agents_dir.join(format!("{value}.md")).exists()
}

/// 本家の `/^[a-z0-9][a-z0-9-]*-agent$/`。
fn agent_name_shape(value: &str) -> bool {
    let bytes = value.as_bytes();
    value.ends_with("-agent")
        && bytes
            .first()
            .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
}

#[cfg(test)]
mod tests {
    // 想定外の形で即座に落とすのは、このテストの検証手段そのものである (house style)。
    #![allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

    use super::*;
    use crate::rule_file::RuleFile;
    use std::fs;
    use tempfile::{TempDir, tempdir};

    /// `aidlc-developer-agent` と `aidlc-quality-agent` だけを置いたペルソナ置き場。
    fn agents() -> TempDir {
        let dir = tempdir().expect("一時ディレクトリ");
        for name in ["aidlc-developer-agent", "aidlc-quality-agent"] {
            fs::write(dir.path().join(format!("{name}.md")), "persona").expect("ペルソナを置く");
        }
        dir
    }

    fn bundle() -> StageRuleBundle {
        StageRuleBundle::new(
            "code-generation".to_string(),
            vec![RuleFile::new("a.md".to_string(), "x".to_string())],
        )
    }

    fn read(output: &str) -> ObjectMembers {
        let JsonValue::Object(root) = parse_json(output).expect("出力は JSON") else {
            panic!("出力はオブジェクト")
        };
        let Some(JsonValue::Object(hook)) = root.get("hookSpecificOutput") else {
            panic!("hookSpecificOutput が無い")
        };
        assert_eq!(
            hook.get("hookEventName"),
            Some(&JsonValue::String("PreToolUse".to_string()))
        );
        let Some(JsonValue::Object(updated)) = hook.get("updatedInput") else {
            panic!("updatedInput が無い")
        };
        updated.clone()
    }

    fn keys(members: &ObjectMembers) -> Vec<String> {
        members.iter().map(|(key, _)| key.to_string()).collect()
    }

    fn text(members: &ObjectMembers, key: &str) -> String {
        match members.get(key) {
            Some(JsonValue::String(value)) => value.clone(),
            other => panic!("{key} は文字列ではない: {other:?}"),
        }
    }

    fn envelope(raw: &str, dir: &TempDir) -> DispatchRulesEnvelope {
        DispatchRulesEnvelope::parse(raw, dir.path())
    }

    #[test]
    fn only_the_four_dispatch_tools_carry_briefs() {
        let dir = agents();
        for tool in ["Task", "TASK", "Agent", "spawn_agent", "task"] {
            let raw = format!(
                r#"{{"tool_name":"{tool}","tool_input":{{"subagent_type":"aidlc-developer-agent","prompt":"work"}}}}"#
            );
            assert_eq!(
                envelope(&raw, &dir).briefs(),
                ["work".to_string()],
                "{tool}"
            );
        }
        for tool in ["Write", "Bash", "", "tasks"] {
            let raw = format!(
                r#"{{"tool_name":"{tool}","tool_input":{{"subagent_type":"aidlc-developer-agent","prompt":"work"}}}}"#
            );
            assert!(envelope(&raw, &dir).briefs().is_empty(), "{tool}");
        }
    }

    #[test]
    fn an_agent_must_name_a_persona_file_and_must_not_be_exempt() {
        let dir = agents();
        let with = |agent: &str| {
            format!(
                r#"{{"tool_name":"Task","tool_input":{{"subagent_type":"{agent}","prompt":"work"}}}}"#
            )
        };
        assert_eq!(
            envelope(&with("aidlc-developer-agent"), &dir).briefs(),
            ["work".to_string()]
        );
        for agent in [
            "general-purpose",
            "aidlc-composer-agent",
            "aidlc-missing-agent",
            "AIDLC-Developer-Agent",
            "-agent",
            "agent",
        ] {
            assert!(envelope(&with(agent), &dir).briefs().is_empty(), "{agent}");
        }
    }

    #[test]
    fn the_agent_name_is_taken_from_the_first_field_that_is_present() {
        let dir = agents();
        for field in ["subagent_type", "agent_type", "agent", "role"] {
            let raw = format!(
                r#"{{"tool_name":"Task","tool_input":{{"{field}":"aidlc-developer-agent","prompt":"work"}}}}"#
            );
            assert_eq!(
                envelope(&raw, &dir).briefs(),
                ["work".to_string()],
                "{field}"
            );
        }
        // 先に現れたフィールドが AI-DLC のエージェントでなければ、後ろは見ない。
        let shadowed = r#"{"tool_name":"Task","tool_input":{"subagent_type":"general-purpose","role":"aidlc-developer-agent","prompt":"work"}}"#;
        assert!(envelope(shadowed, &dir).briefs().is_empty());
        // `null` は「無い」と同じ扱いで、次のフィールドへ進む。
        let nulled = r#"{"tool_name":"Task","tool_input":{"subagent_type":null,"role":"aidlc-developer-agent","prompt":"work"}}"#;
        assert_eq!(envelope(nulled, &dir).briefs(), ["work".to_string()]);
    }

    #[test]
    fn the_brief_is_the_first_string_body_field_and_the_block_lands_there() {
        let dir = agents();
        for field in ["prompt", "message", "description", "task"] {
            let raw = format!(
                r#"{{"tool_name":"Task","tool_input":{{"subagent_type":"aidlc-developer-agent","{field}":"work"}}}}"#
            );
            let envelope = envelope(&raw, &dir);
            assert_eq!(envelope.briefs(), ["work".to_string()], "{field}");
            let output = envelope
                .pre_tool_use_output(&[Some(bundle())])
                .expect("変化する");
            let updated = read(&output);
            assert_eq!(keys(&updated), ["subagent_type", field], "{field}");
            assert_eq!(
                text(&updated, field),
                format!("work{}", bundle().block()),
                "{field}"
            );
        }
    }

    #[test]
    fn a_later_body_field_never_wins_over_an_earlier_one() {
        let dir = agents();
        let raw = r#"{"tool_name":"Task","tool_input":{"message":"second","subagent_type":"aidlc-developer-agent","prompt":"first"}}"#;
        let envelope = envelope(raw, &dir);
        assert_eq!(envelope.briefs(), ["first".to_string()]);
        let updated = read(
            &envelope
                .pre_tool_use_output(&[Some(bundle())])
                .expect("変化する"),
        );
        assert_eq!(keys(&updated), ["message", "subagent_type", "prompt"]);
        assert_eq!(text(&updated, "message"), "second");
        assert_eq!(
            text(&updated, "prompt"),
            format!("first{}", bundle().block())
        );
    }

    #[test]
    fn an_empty_or_absent_body_carries_no_brief() {
        let dir = agents();
        for input in [
            r#"{"subagent_type":"aidlc-developer-agent","prompt":""}"#,
            r#"{"subagent_type":"aidlc-developer-agent"}"#,
            r#"{"subagent_type":"aidlc-developer-agent","items":[]}"#,
            r#"{"subagent_type":"aidlc-developer-agent","prompt":7}"#,
        ] {
            let raw = format!(r#"{{"tool_name":"Task","tool_input":{input}}}"#);
            assert!(envelope(&raw, &dir).briefs().is_empty(), "{input}");
        }
    }

    #[test]
    fn an_items_body_joins_with_newlines_and_the_block_arrives_as_a_new_item() {
        let dir = agents();
        let raw = r#"{"tool_name":"Task","tool_input":{"subagent_type":"aidlc-developer-agent","items":[{"type":"text","text":"one"},{"type":"image"},{"type":"text","text":"two"}]}}"#;
        let envelope = envelope(raw, &dir);
        assert_eq!(envelope.briefs(), ["one\n\ntwo".to_string()]);
        let updated = read(
            &envelope
                .pre_tool_use_output(&[Some(bundle())])
                .expect("変化する"),
        );
        assert_eq!(keys(&updated), ["subagent_type", "items"]);
        let Some(JsonValue::Array(items)) = updated.get("items") else {
            panic!("items が配列でない")
        };
        assert_eq!(items.len(), 4, "元の 3 件の後ろへ 1 件足す");
        let JsonValue::Object(added) = &items[3] else {
            panic!("足したのはオブジェクト")
        };
        assert_eq!(keys(added), ["type", "text"]);
        assert_eq!(
            added.get("text"),
            Some(&JsonValue::String(bundle().block()))
        );
    }

    #[test]
    fn a_non_string_body_field_falls_through_to_items() {
        let dir = agents();
        let raw = r#"{"tool_name":"Task","tool_input":{"subagent_type":"aidlc-developer-agent","prompt":7,"items":[{"type":"text","text":"one"}]}}"#;
        let envelope = envelope(raw, &dir);
        assert_eq!(envelope.briefs(), ["one".to_string()]);
        let updated = read(
            &envelope
                .pre_tool_use_output(&[Some(bundle())])
                .expect("変化する"),
        );
        assert_eq!(keys(&updated), ["subagent_type", "prompt", "items"]);
        assert_eq!(
            updated.get("prompt"),
            Some(&JsonValue::Number(
                core_infrastructure::canon_json::Number::PosInt(7)
            ))
        );
    }

    #[test]
    fn the_subagent_tool_reads_every_eligible_stage_entry() {
        let dir = agents();
        let raw = r#"{"tool_name":"subagent","tool_input":{"stages":[null,"x",{"role":"general-purpose","prompt_template":"skip"},{"role":"aidlc-developer-agent","prompt_template":7},{"role":"aidlc-developer-agent","prompt_template":"one"},{"role":"aidlc-quality-agent","prompt_template":"two"}]}}"#;
        let envelope = envelope(raw, &dir);
        assert_eq!(envelope.briefs(), ["one".to_string(), "two".to_string()]);
        let updated = read(
            &envelope
                .pre_tool_use_output(&[Some(bundle()), Some(bundle())])
                .expect("変化する"),
        );
        assert_eq!(keys(&updated), ["stages"]);
        let Some(JsonValue::Array(stages)) = updated.get("stages") else {
            panic!("stages が配列でない")
        };
        assert_eq!(stages.len(), 6, "対象外の要素もそのまま残す");
        assert_eq!(stages[0], JsonValue::Null);
        assert_eq!(stages[1], JsonValue::String("x".to_string()));
        for (index, brief) in [(4_usize, "one"), (5, "two")] {
            let JsonValue::Object(entry) = &stages[index] else {
                panic!("対象要素はオブジェクト")
            };
            assert_eq!(keys(entry), ["role", "prompt_template"]);
            assert_eq!(
                text(entry, "prompt_template"),
                format!("{brief}{}", bundle().block())
            );
        }
    }

    #[test]
    fn an_empty_stage_template_is_still_a_brief() {
        let dir = agents();
        let raw = r#"{"tool_name":"subagent","tool_input":{"stages":[{"role":"aidlc-developer-agent","prompt_template":""}]}}"#;
        let envelope = envelope(raw, &dir);
        assert_eq!(envelope.briefs(), [String::new()]);
        let updated = read(
            &envelope
                .pre_tool_use_output(&[Some(bundle())])
                .expect("変化する"),
        );
        let Some(JsonValue::Array(stages)) = updated.get("stages") else {
            panic!("stages が配列でない")
        };
        let JsonValue::Object(entry) = &stages[0] else {
            panic!("対象要素はオブジェクト")
        };
        assert_eq!(text(entry, "prompt_template"), bundle().block());
    }

    #[test]
    fn a_stage_entry_without_a_string_role_is_skipped() {
        let dir = agents();
        let raw = r#"{"tool_name":"subagent","tool_input":{"stages":[{"prompt_template":"no role"},{"role":7,"prompt_template":"numeric role"},{"role":"aidlc-developer-agent","prompt_template":"one"}]}}"#;
        assert_eq!(envelope(raw, &dir).briefs(), ["one".to_string()]);
    }

    #[test]
    fn an_items_entry_that_is_not_a_text_object_counts_as_an_empty_line() {
        let dir = agents();
        let raw = r#"{"tool_name":"Task","tool_input":{"subagent_type":"aidlc-developer-agent","items":["plain",7,null,{"type":"text","text":5},{"type":"text","text":"one"}]}}"#;
        assert_eq!(envelope(raw, &dir).briefs(), ["\n\n\n\none".to_string()]);
        let only_empty = r#"{"tool_name":"Task","tool_input":{"subagent_type":"aidlc-developer-agent","items":["plain",{"text":1}]}}"#;
        // 本家の `join("\n")` と同じく、空行だけでも本文は空にならない。
        assert_eq!(envelope(only_empty, &dir).briefs(), ["\n".to_string()]);
    }

    #[test]
    fn a_subagent_dispatch_without_stages_carries_no_brief() {
        let dir = agents();
        for input in [
            r#"{"subagent_type":"aidlc-developer-agent","prompt":"work"}"#,
            r#"{"stages":{}}"#,
            r#"{"stages":[]}"#,
        ] {
            let raw = format!(r#"{{"tool_name":"subagent","tool_input":{input}}}"#);
            assert!(envelope(&raw, &dir).briefs().is_empty(), "{input}");
        }
    }

    #[test]
    fn unreadable_or_shapeless_input_carries_no_brief() {
        let dir = agents();
        for raw in [
            "{not json",
            "",
            "null",
            "[1,2,3]",
            r#"{"tool_name":"Task"}"#,
            r#"{"tool_input":{"subagent_type":"aidlc-developer-agent","prompt":"work"}}"#,
            r#"{"tool_name":7,"tool_input":{"subagent_type":"aidlc-developer-agent","prompt":"work"}}"#,
            r#"{"tool_name":"Task","tool_input":"x"}"#,
        ] {
            assert!(envelope(raw, &dir).briefs().is_empty(), "{raw}");
            assert!(
                envelope(raw, &dir).pre_tool_use_output(&[]).is_none(),
                "{raw}"
            );
        }
    }

    #[test]
    fn nothing_is_written_when_no_brief_actually_changes() {
        let dir = agents();
        let raw = r#"{"tool_name":"Task","tool_input":{"subagent_type":"aidlc-developer-agent","prompt":"work"}}"#;
        let envelope = envelope(raw, &dir);
        assert!(envelope.pre_tool_use_output(&[None]).is_none(), "束が無い");
        let empty = StageRuleBundle::new("code-generation".to_string(), Vec::new());
        assert!(
            envelope.pre_tool_use_output(&[Some(empty)]).is_none(),
            "空束は書き換えない"
        );
        assert!(
            envelope.pre_tool_use_output(&[]).is_none(),
            "束の列が足りなければ書き換えない"
        );
    }

    #[test]
    fn a_brief_that_already_carries_the_exact_block_is_left_alone() {
        let dir = agents();
        let block = bundle().block();
        let escaped = serialize(
            &JsonValue::String(format!("work{block}")),
            SerializationProfile::ContractCompact,
        );
        let raw = format!(
            r#"{{"tool_name":"Task","tool_input":{{"subagent_type":"aidlc-developer-agent","prompt":{escaped}}}}}"#
        );
        assert!(
            envelope(&raw, &dir)
                .pre_tool_use_output(&[Some(bundle())])
                .is_none()
        );
        // 1 バイト違えば抑止されない。
        let near = serialize(
            &JsonValue::String(format!("work{}", &block[..block.len() - 1])),
            SerializationProfile::ContractCompact,
        );
        let raw = format!(
            r#"{{"tool_name":"Task","tool_input":{{"subagent_type":"aidlc-developer-agent","prompt":{near}}}}}"#
        );
        assert!(
            envelope(&raw, &dir)
                .pre_tool_use_output(&[Some(bundle())])
                .is_some()
        );
    }

    #[test]
    fn the_tool_name_is_reported_verbatim() {
        let dir = agents();
        let raw = r#"{"tool_name":"TASK","tool_input":{"subagent_type":"aidlc-developer-agent","prompt":"work"}}"#;
        assert_eq!(envelope(raw, &dir).tool(), "TASK");
        assert_eq!(envelope("{not json", &dir).tool(), "");
    }

    #[test]
    fn the_output_wraps_the_updated_input_in_the_pre_tool_use_envelope() {
        let dir = agents();
        let raw = r#"{"tool_name":"Task","tool_input":{"subagent_type":"aidlc-developer-agent","prompt":"work"}}"#;
        let output = envelope(raw, &dir)
            .pre_tool_use_output(&[Some(bundle())])
            .expect("変化する");
        assert!(
            output.starts_with(
                r#"{"hookSpecificOutput":{"hookEventName":"PreToolUse","updatedInput":{"#
            ),
            "{output}"
        );
        assert!(output.ends_with("}}}"), "{output}");
        assert!(!output.ends_with('\n'), "末尾改行は書く側が付ける");
    }
}
