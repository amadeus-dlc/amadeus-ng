//! `StageRuleBundle` — 1 ステージぶんの規則束と、brief へ差し込むブロックの描画。

use core_infrastructure::canon_json::{JsonValue, ObjectMembers, hash_compact};

use crate::rule_file::RuleFile;

/// ブロックの見出し（本家が固定した 1 行 — t248 でピン留めされている）。
const HEADING: &str = "## Active AI-DLC Rule Bundle";
/// 見出しに続く 1 文。読み手が見るのはこの文なので、機構ではなく規則そのものを名乗る。
const FRAMING: &str = "These are the required rules for this stage. Apply the content verbatim; later prose summaries do not replace it.";

/// 1 ステージぶんの規則束（ファーストクラスコレクション）。
///
/// 本家 2.7.1 `hooks/aidlc-deliver-stage-rules.ts:108-134` の `bundleBlock` /
/// `hasExactBundle` に対応する。ここが持つのは**描画とダイジェスト**だけで、どの stage を
/// 選ぶか・どのファイルを読むかの判断は持たない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageRuleBundle {
    stage: String,
    files: Vec<RuleFile>,
}

impl StageRuleBundle {
    /// ステージ綴りと読み順の規則列から束を組む（**この型の唯一の構築経路**）。
    #[must_use]
    pub const fn new(stage: String, files: Vec<RuleFile>) -> StageRuleBundle {
        StageRuleBundle { stage, files }
    }

    /// 載る規則が 1 件も無いか。
    ///
    /// 空束は brief を書き換えない（本家 `bundle.content.length === 0` の枝）。
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// brief の末尾へ足すブロック（先頭 2 改行つき・末尾に改行を付けない）。
    #[must_use]
    pub fn block(&self) -> String {
        let digest = self.digest();
        let body = self.files.iter().fold(String::new(), |mut body, file| {
            body.push_str("\n### ");
            body.push_str(file.path());
            body.push('\n');
            body.push_str(file.text());
            body
        });
        format!(
            "\n\n<!-- AIDLC_DISPATCH_RULES_BEGIN sha256:{digest} stage:{stage} -->\n\
             {HEADING}\n{FRAMING}\n{body}\n<!-- AIDLC_DISPATCH_RULES_END sha256:{digest} -->",
            stage = self.stage,
        )
    }

    /// 束のダイジェスト — `[{path,text},...]` の正準 JSON に対する sha256（生 hex）。
    ///
    /// 素材は**規則列だけ**である。stage 綴りは BEGIN マーカーに載るが素材には入らない
    /// （本家 `createHash("sha256").update(JSON.stringify(content))`）。
    fn digest(&self) -> String {
        let material = JsonValue::Array(
            self.files
                .iter()
                .map(|file| {
                    let mut members = ObjectMembers::new();
                    members.insert("path", JsonValue::String(file.path().to_string()));
                    members.insert("text", JsonValue::String(file.text().to_string()));
                    JsonValue::Object(members)
                })
                .collect(),
        );
        hash_compact(&material).rendered()
    }

    /// この brief に**同一の**ブロックが既に入っているか（本家 `hasExactBundle`）。
    ///
    /// 部分文字列一致である — 末尾でも途中でも抑止する。
    #[must_use]
    pub fn already_in(&self, brief: &str) -> bool {
        brief.contains(&self.block())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 本家を実走行して採った合成 fixture の観測。出所は
    /// `stage-rules-logs/upstream-fixture-block.txt`。
    fn upstream_fixture() -> StageRuleBundle {
        StageRuleBundle::new(
            "code-generation".to_string(),
            vec![
                RuleFile::new(
                    "aidlc/spaces/default/memory/org.md".to_string(),
                    "# Org\n\nOrg rule body.\n".to_string(),
                ),
                RuleFile::new(
                    "aidlc/spaces/default/memory/project.md".to_string(),
                    "# Project\n\nNon-ASCII と \"引用符\" と バックスラッシュ \\ と タブ\tと 制御文字の並び。\n".to_string(),
                ),
                RuleFile::new(
                    "aidlc/spaces/default/memory/phases/construction.md".to_string(),
                    "# Construction\n\nPhase rule body.\n".to_string(),
                ),
            ],
        )
    }

    const UPSTREAM_DIGEST: &str =
        "3072de5dc9edccbd72b5e2110e0d07483b8c3376fe37d845710f6aa4422e5e64";

    #[test]
    fn the_block_is_byte_identical_to_the_upstream_capture() {
        let expected = format!(
            "\n\n<!-- AIDLC_DISPATCH_RULES_BEGIN sha256:{UPSTREAM_DIGEST} stage:code-generation -->\n\
             ## Active AI-DLC Rule Bundle\n\
             These are the required rules for this stage. Apply the content verbatim; later prose summaries do not replace it.\n\
             \n### aidlc/spaces/default/memory/org.md\n# Org\n\nOrg rule body.\n\
             \n### aidlc/spaces/default/memory/project.md\n# Project\n\nNon-ASCII と \"引用符\" と バックスラッシュ \\ と タブ\tと 制御文字の並び。\n\
             \n### aidlc/spaces/default/memory/phases/construction.md\n# Construction\n\nPhase rule body.\n\
             \n<!-- AIDLC_DISPATCH_RULES_END sha256:{UPSTREAM_DIGEST} -->"
        );
        assert_eq!(upstream_fixture().block(), expected);
    }

    #[test]
    fn the_digest_is_the_sha256_of_the_path_text_pairs() {
        let block = upstream_fixture().block();
        assert!(
            block.contains(&format!(
                "BEGIN sha256:{UPSTREAM_DIGEST} stage:code-generation"
            )),
            "{block}"
        );
        assert!(
            block.ends_with(&format!("END sha256:{UPSTREAM_DIGEST} -->")),
            "{block}"
        );
    }

    #[test]
    fn one_changed_byte_of_any_rule_changes_the_digest() {
        let base = upstream_fixture().block();
        let changed_text = StageRuleBundle::new(
            "code-generation".to_string(),
            vec![RuleFile::new("a.md".to_string(), "x".to_string())],
        );
        let changed_path = StageRuleBundle::new(
            "code-generation".to_string(),
            vec![RuleFile::new("b.md".to_string(), "x".to_string())],
        );
        assert_ne!(
            changed_text.block(),
            changed_path.block(),
            "綴りも素材である"
        );
        assert_ne!(base, changed_text.block());
    }

    #[test]
    fn the_stage_slug_rides_in_the_begin_marker_only() {
        let one = StageRuleBundle::new(
            "code-generation".to_string(),
            vec![RuleFile::new("a.md".to_string(), "x".to_string())],
        );
        let other = StageRuleBundle::new(
            "build-and-test".to_string(),
            vec![RuleFile::new("a.md".to_string(), "x".to_string())],
        );
        assert!(one.block().contains("stage:code-generation"));
        assert!(other.block().contains("stage:build-and-test"));
        // 同じ内容なら stage が違ってもダイジェストは同じ（素材は規則列だけ）。
        let digest = |block: &str| {
            block
                .split("sha256:")
                .nth(1)
                .map(|rest| {
                    rest.split_whitespace()
                        .next()
                        .unwrap_or_default()
                        .to_string()
                })
                .unwrap_or_default()
        };
        assert_eq!(digest(&one.block()), digest(&other.block()));
    }

    #[test]
    fn an_empty_bundle_is_empty_and_still_renders_a_stable_block() {
        let empty = StageRuleBundle::new("code-generation".to_string(), Vec::new());
        assert!(empty.is_empty());
        assert!(!upstream_fixture().is_empty());
        assert!(empty.block().contains("AIDLC_DISPATCH_RULES_BEGIN"));
    }

    #[test]
    fn an_exact_block_anywhere_in_the_brief_counts_as_already_delivered() {
        let bundle = upstream_fixture();
        let block = bundle.block();
        assert!(bundle.already_in(&format!("head{block}")));
        assert!(bundle.already_in(&format!("{block}tail")));
        assert!(bundle.already_in(&format!("head{block}tail")));
        assert!(!bundle.already_in(&block[..block.len() - 1]));
        assert!(!bundle.already_in("nothing here"));
    }
}
