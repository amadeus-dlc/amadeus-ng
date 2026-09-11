//! reviewer-scope の PreToolUse 封筒から、識別と経路候補を取り出す。
use core_command_domain::orchestration::{
    InspectedCommand, InspectedCommandStep, InspectedTool, ReviewerScopeCandidate,
    ReviewerScopeCandidates, ScopeToken,
};
use harness_infrastructure::ReviewerScopeSegments;
use serde_json::Value;

/// upstream `REVIEW_AGENT_RE` — 出荷されるレビュー専用エージェント 2 種 (逐語)。
const REVIEW_AGENTS: [&str; 2] = [
    "aidlc-architecture-reviewer-agent",
    "aidlc-product-lead-agent",
];

/// PreToolUse 入力 1 件を、工具・呼び手・経路候補として読んだもの。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewerScopeEnvelope {
    tool: Option<InspectedTool>,
    agent_type: String,
    scoped_registration: bool,
    cwd: Option<String>,
    candidates: ReviewerScopeCandidates,
    texts: Vec<String>,
}
impl ReviewerScopeEnvelope {
    /// 読んだ材料を同時に構築する (**この型の唯一の構築経路**)。
    const fn new(
        tool: Option<InspectedTool>,
        agent_type: String,
        scoped_registration: bool,
        cwd: Option<String>,
        candidates: ReviewerScopeCandidates,
        texts: Vec<String>,
    ) -> ReviewerScopeEnvelope {
        ReviewerScopeEnvelope {
            tool,
            agent_type,
            scoped_registration,
            cwd,
            candidates,
            texts,
        }
    }
    /// PreToolUse の JSON を読む。読めない入力は工具名なしとして扱う。
    #[must_use]
    pub fn parse(input: &str) -> ReviewerScopeEnvelope {
        let value = serde_json::from_str::<Value>(input).unwrap_or(Value::Null);
        let tool = value
            .get("tool_name")
            .and_then(Value::as_str)
            .and_then(InspectedTool::parse);
        let agent_type = value
            .get("agent_type")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let scoped_registration = value.get("scoped_registration") == Some(&Value::Bool(true));
        let cwd = value
            .get("cwd")
            .and_then(Value::as_str)
            .filter(|dir| !dir.is_empty())
            .map(str::to_string);
        let tool_input = value.get("tool_input");
        let texts = tool
            .map(|tool| raw_texts(tool, tool_input))
            .unwrap_or_default();
        let candidates = ReviewerScopeCandidates::new(read_candidates(&texts));
        ReviewerScopeEnvelope::new(
            tool,
            agent_type,
            scoped_registration,
            cwd,
            candidates,
            texts.into_iter().map(|(text, _)| text).collect(),
        )
    }
    /// upstream が見る工具か (それ以外の呼出しは素通しする)。
    #[must_use]
    pub const fn inspected(&self) -> bool {
        self.tool.is_some()
    }
    /// 見る工具として読めた場合のその工具。
    #[must_use]
    pub const fn tool(&self) -> Option<InspectedTool> {
        self.tool
    }
    /// 判定にかける候補 (掲載順)。
    #[must_use]
    pub const fn candidates(&self) -> &ReviewerScopeCandidates {
        &self.candidates
    }
    /// ハーネスが渡した作業ディレクトリ (空・欠落は `None`)。
    #[must_use]
    pub fn cwd(&self) -> Option<&str> {
        self.cwd.as_deref()
    }
    /// Kiro CLI の scoped registration を名乗るか。
    #[must_use]
    pub const fn scoped_registration(&self) -> bool {
        self.scoped_registration
    }
    /// 呼び手が出荷のレビュー専用エージェントか。
    #[must_use]
    pub fn review_agent(&self) -> bool {
        REVIEW_AGENTS.contains(&self.agent_type.as_str())
    }
    /// 呼び手の名前 (main セッションの呼出しでは空)。
    #[must_use]
    pub fn agent_type(&self) -> &str {
        &self.agent_type
    }
    /// 経路候補のいずれかが Construction の記録へ触れるか。
    #[must_use]
    pub fn touches_construction(&self) -> bool {
        self.texts
            .iter()
            .any(|text| text.replace('\\', "/").contains("construction/"))
    }
}

/// 候補 1 件の読み方 (upstream `candidateStrings` の `kind`)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Target,
    SearchRoot,
    Glob,
    Command,
}

/// 工具ごとに upstream `candidateStrings` が見る鍵を、掲載順に読む。
///
/// `Grep` の `pattern` は**内容の正規表現**なので見ない — 内容の一致はファイルへの
/// 到達ではないし、見ると「兄弟の経路に言及するだけの現 Unit の検索」まで拒否してしまう。
fn raw_texts(tool: InspectedTool, input: Option<&Value>) -> Vec<(String, Kind)> {
    let keys: &[(&str, Kind)] = match tool {
        InspectedTool::Bash => &[("command", Kind::Command)],
        InspectedTool::Ls => &[("path", Kind::SearchRoot)],
        InspectedTool::Glob => &[("pattern", Kind::Glob), ("path", Kind::SearchRoot)],
        InspectedTool::Grep => &[("glob", Kind::Glob), ("path", Kind::SearchRoot)],
        _ => &[
            ("file_path", Kind::Target),
            ("notebook_path", Kind::Target),
            ("path", Kind::Target),
        ],
    };
    let mut texts: Vec<(String, Kind)> = keys
        .iter()
        .filter_map(|(key, kind)| {
            input
                .and_then(|input| input.get(*key))
                .and_then(Value::as_str)
                .filter(|text| !text.is_empty())
                .map(|text| (text.to_string(), *kind))
        })
        .collect();
    if tool.names_paths() {
        texts.extend(
            input
                .and_then(|input| input.get("paths"))
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .filter(|text| !text.is_empty())
                .map(|text| (text.to_string(), Kind::Target)),
        );
    }
    texts
}

/// 読んだ綴りを、判定にかけられる候補へ変える。
fn read_candidates(texts: &[(String, Kind)]) -> Vec<ReviewerScopeCandidate> {
    texts
        .iter()
        .filter_map(|(text, kind)| match kind {
            Kind::Command => Some(ReviewerScopeCandidate::Command(read_command(text))),
            Kind::Target => ScopeToken::parse(text)
                .ok()
                .map(ReviewerScopeCandidate::Target),
            Kind::SearchRoot => ScopeToken::parse(text)
                .ok()
                .map(ReviewerScopeCandidate::SearchRoot),
            Kind::Glob => ScopeToken::parse(text)
                .ok()
                .map(ReviewerScopeCandidate::Glob),
        })
        .collect()
}

/// `Bash` の綴りを実行位置・語へ切り、ドメインが読める形にする。
fn read_command(command: &str) -> InspectedCommand {
    let segments = ReviewerScopeSegments::parse(command);
    InspectedCommand::new(
        (0..segments.len())
            .filter_map(|index| segments.at(index))
            .map(|words| {
                InspectedCommandStep::new(
                    (0..words.len())
                        .filter_map(|index| words.at(index))
                        .filter_map(|word| ScopeToken::parse(word).ok())
                        .collect(),
                )
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::{ReviewerScopeEnvelope, read_command};
    use core_command_domain::orchestration::InspectedTool;

    #[test]
    fn a_reviewer_reading_a_construction_path_is_both_inspected_and_flagged() {
        let envelope = ReviewerScopeEnvelope::parse(
            r#"{"tool_name":"Read","agent_type":"aidlc-architecture-reviewer-agent","tool_input":{"file_path":"/w/x/construction/u2/functional-design/entities.md"}}"#,
        );
        assert!(envelope.inspected());
        assert_eq!(envelope.tool(), InspectedTool::parse("Read"));
        assert!(envelope.review_agent());
        assert!(envelope.touches_construction());
        assert_eq!(envelope.agent_type(), "aidlc-architecture-reviewer-agent");
    }

    #[test]
    fn a_glob_or_grep_names_its_pattern_and_search_root() {
        for input in [
            r#"{"tool_name":"Glob","agent_type":"aidlc-product-lead-agent","tool_input":{"pattern":"construction/*/design.md"}}"#,
            r#"{"tool_name":"Grep","agent_type":"aidlc-product-lead-agent","tool_input":{"glob":"*.md","path":"/w/x/construction/u1"}}"#,
        ] {
            let envelope = ReviewerScopeEnvelope::parse(input);
            assert!(envelope.inspected(), "{input}");
            assert!(envelope.touches_construction(), "{input}");
        }
    }

    #[test]
    fn a_bash_command_is_read_from_its_command_string() {
        let envelope = ReviewerScopeEnvelope::parse(
            r#"{"tool_name":"Bash","agent_type":"aidlc-product-lead-agent","tool_input":{"command":"cat aidlc/spaces/default/intents/x/construction/u1/plan.md"}}"#,
        );
        assert!(envelope.touches_construction());
    }

    #[test]
    fn a_non_inspected_tool_and_a_non_review_agent_are_passed_through() {
        let envelope = ReviewerScopeEnvelope::parse(
            r#"{"tool_name":"Task","agent_type":"aidlc-developer-agent","tool_input":{"file_path":"/w/x/construction/u1/a.md"}}"#,
        );
        assert!(!envelope.inspected());
        assert!(!envelope.review_agent());
        assert!(
            !envelope.touches_construction(),
            "見ない工具からは候補を集めない"
        );
    }

    #[test]
    fn paths_outside_construction_and_malformed_input_raise_nothing() {
        for input in [
            r#"{"tool_name":"Read","agent_type":"aidlc-product-lead-agent","tool_input":{"file_path":"/w/x/inception/requirements-analysis/requirements.md"}}"#,
            // upstream の判定は区切り込みの `construction/` 一致である — 末尾が
            // `construction` で終わる探索根はこの助言を上げない (逐語の境界)。
            r#"{"tool_name":"LS","agent_type":"aidlc-product-lead-agent","tool_input":{"path":"/w/x/construction"}}"#,
            "not json",
            "{}",
        ] {
            let envelope = ReviewerScopeEnvelope::parse(input);
            assert!(!envelope.touches_construction(), "{input}");
        }
    }

    #[test]
    fn windows_separators_still_reveal_the_construction_segment() {
        let envelope = ReviewerScopeEnvelope::parse(
            r#"{"tool_name":"Read","agent_type":"aidlc-product-lead-agent","tool_input":{"file_path":"C:\\w\\x\\construction\\u1\\a.md"}}"#,
        );
        assert!(envelope.touches_construction());
    }

    #[test]
    fn the_working_directory_and_scoped_registration_are_read_when_present() {
        let envelope = ReviewerScopeEnvelope::parse(
            r#"{"tool_name":"Read","cwd":"/w","scoped_registration":true,"tool_input":{"file_path":"/w/a.md"}}"#,
        );
        assert_eq!(envelope.cwd(), Some("/w"));
        assert!(envelope.scoped_registration());
        let bare = ReviewerScopeEnvelope::parse(r#"{"tool_name":"Read","cwd":"","tool_input":{}}"#);
        assert_eq!(bare.cwd(), None);
        assert!(!bare.scoped_registration());
    }

    #[test]
    fn a_command_is_split_into_execution_positions_and_words() {
        let command = read_command("cd /r && cat a.md");
        assert_eq!(command.len(), 2);
        assert_eq!(
            command.at(0).map(|step| step.len()).zip(
                command
                    .at(0)
                    .and_then(|step| step.at(1))
                    .map(|word| word.as_str().to_string())
            ),
            Some((2, "/r".to_string()))
        );
    }
}
