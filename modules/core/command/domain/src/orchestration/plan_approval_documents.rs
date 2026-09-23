//! 計画承認のために入力境界で読み取った文書。
/// 読書きの機構を持たず、計画・指示・質問の原文と正規の質問パスを運ぶ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanApprovalDocuments {
    plan: String,
    instructions: String,
    questions: String,
    questions_file: String,
}
impl PlanApprovalDocuments {
    /// 正規の配置から読み取った原文を束ねる。
    #[must_use]
    pub const fn new(
        plan: String,
        instructions: String,
        questions: String,
        questions_file: String,
    ) -> Self {
        Self {
            plan,
            instructions,
            questions,
            questions_file,
        }
    }
    /// 計画の原文。
    #[must_use]
    pub fn plan(&self) -> &str {
        &self.plan
    }
    /// テスト指示の原文。
    #[must_use]
    pub fn instructions(&self) -> &str {
        &self.instructions
    }
    /// 承認が束ねる形の計画 — 作業者へ渡してよいのはこれだけである（2.8.2
    /// `workerBrief` が渡す `projectPlanApprovalContent(plan)`）。
    ///
    /// 末尾の `## Review` 付録を落とし、進捗の印を `[ ]` へ戻す。承認の指紋
    /// （[`super::CodeGenerationAuthority::approval_fingerprint`]）も同じ形を束ねるので、承認後に
    /// 付録へ足した手順や、手順の印の書き換えは、指紋を変えないまま作業者へ届くことがない。
    #[must_use]
    pub fn approved_plan(&self) -> String {
        project_plan(&self.plan)
    }
    /// 承認が束ねる形のテスト指示 — 改行だけを LF へ正規化した全文（2.8.2
    /// `projectInstructionsContent`）。作業者へ渡すのもこの形である。
    #[must_use]
    pub fn approved_instructions(&self) -> String {
        self.instructions.replace("\r\n", "\n").replace('\r', "\n")
    }
    /// 質問の原文。
    #[must_use]
    pub fn questions(&self) -> &str {
        &self.questions
    }
    /// 正規の質問文書のプロジェクト相対パス。
    #[must_use]
    pub fn questions_file(&self) -> &str {
        &self.questions_file
    }
}

/// 計画の承認射影（2.8.2 `projectPlanApprovalContent` のうち、段自身が承認後に命じる編集を
/// 消す 2 点）。
///
/// 1. 末尾の `## Review` 節を落とす（位置は [`super::review_appendix`] と同じ規則）。
/// 2. フェンスと HTML コメントの外で、リストのタスク印 `[x]` / `[X]` / `[-]` を `[ ]` へ戻す。
///
/// 2.8.2 は加えて改行・行末空白・連続空行も畳むが、この build はそれ以外をバイトどおりに
/// 束ねる（より厳しい側。進捗の印を付けても承認が失効しない、という性質はこの 2 点で足りる）。
fn project_plan(text: &str) -> String {
    let retained = super::review_appendix::ReviewAppendix::content_before(text);
    let mut projected = String::with_capacity(retained.len());
    let mut fence: Option<(char, usize)> = None;
    let mut in_comment = false;
    for raw in retained.split_inclusive('\n') {
        let body = raw.trim_end_matches(['\n', '\r']);
        let ending = raw.get(body.len()..).unwrap_or_default();
        let line = body.trim_end_matches([' ', '\t']);
        if let Some(open) = fence {
            if closes_fence(line, open) {
                fence = None;
            }
            projected.push_str(raw);
            continue;
        }
        if in_comment {
            if line.contains("-->") {
                in_comment = false;
            }
            projected.push_str(raw);
            continue;
        }
        if let Some(open) = fence_opening(line) {
            fence = Some(open);
            projected.push_str(raw);
            continue;
        }
        if leading_spaces(line) <= 3
            && line.trim_start_matches(' ').starts_with("<!--")
            && !line.contains("-->")
        {
            in_comment = true;
            projected.push_str(raw);
            continue;
        }
        projected.push_str(&reset_task_marker(body));
        projected.push_str(ending);
    }
    projected
}

fn leading_spaces(line: &str) -> usize {
    line.bytes().take_while(|byte| *byte == b' ').count()
}

/// フェンスの開始（2.8.2 `fenceOpening` — バッククォートのフェンスは情報文字列に
/// バッククォートを含まない）。
fn fence_opening(line: &str) -> Option<(char, usize)> {
    if leading_spaces(line) > 3 {
        return None;
    }
    let rest = line.trim_start_matches(' ');
    let marker = rest.chars().next().filter(|c| *c == '`' || *c == '~')?;
    let length = rest.chars().take_while(|c| *c == marker).count();
    if length < 3 {
        return None;
    }
    let info = rest.get(length..).unwrap_or_default();
    if marker == '`' && info.contains('`') {
        return None;
    }
    Some((marker, length))
}

/// フェンスの終了（2.8.2 `closesFence`）。
fn closes_fence(line: &str, (marker, length): (char, usize)) -> bool {
    if leading_spaces(line) > 3 {
        return false;
    }
    let rest = line.trim_start_matches(' ').trim_end_matches([' ', '\t']);
    !rest.is_empty() && rest.chars().all(|c| c == marker) && rest.chars().count() >= length
}

/// リストのタスク印を未着手へ戻す（2.8.2 `PLAN_TASK_MARKER_RE`:
/// `^([ \t]*(?:[-*+]|\d+[.)])[ \t]+)\[[xX-]\](?=[ \t]|$)`）。
fn reset_task_marker(line: &str) -> String {
    let indent = line.len() - line.trim_start_matches([' ', '\t']).len();
    let rest = line.get(indent..).unwrap_or_default();
    let bullet = if rest.starts_with(['-', '*', '+']) {
        1
    } else {
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        if digits > 0
            && rest
                .get(digits..)
                .is_some_and(|tail| tail.starts_with(['.', ')']))
        {
            digits + 1
        } else {
            return line.to_string();
        }
    };
    let after = rest.get(bullet..).unwrap_or_default();
    let spaces = after.len() - after.trim_start_matches([' ', '\t']).len();
    if spaces == 0 {
        return line.to_string();
    }
    let prefix_len = indent + bullet + spaces;
    let tail = line.get(prefix_len..).unwrap_or_default();
    let marked = ["[x]", "[X]", "[-]"]
        .iter()
        .any(|mark| tail.starts_with(mark));
    let boundary = tail
        .get(3..)
        .is_some_and(|after| after.is_empty() || after.starts_with([' ', '\t']));
    if marked && boundary {
        format!(
            "{}[ ]{}",
            line.get(..prefix_len).unwrap_or_default(),
            tail.get(3..).unwrap_or_default()
        )
    } else {
        line.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{PlanApprovalDocuments, project_plan};

    #[test]
    fn the_approved_plan_drops_a_smuggled_appendix_and_resets_progress_marks() {
        let documents = PlanApprovalDocuments::new(
            "# Plan\n\n- [x] Step 3\n\n## Review\n\n- [ ] Step 9: unapproved\n".to_string(),
            "Run\r\ncargo test.\r".to_string(),
            String::new(),
            String::new(),
        );
        assert_eq!(documents.approved_plan(), "# Plan\n\n- [ ] Step 3\n");
        assert_eq!(documents.approved_instructions(), "Run\ncargo test.\n");
    }

    #[test]
    fn ticking_a_step_or_appending_a_review_does_not_change_the_projection() {
        let plan = "# Plan\n\n- [ ] Step 1\n1. [ ] Step 2\n";
        let ticked = "# Plan\n\n- [x] Step 1\n1. [X] Step 2\n";
        let reviewed = "# Plan\n\n- [-] Step 1\n1. [ ] Step 2\n\n## Review\n\n**Verdict:** READY\n";
        assert_eq!(project_plan(ticked), project_plan(plan));
        assert_eq!(project_plan(reviewed), project_plan(plan));
        // それ以外はバイトどおり — 手順の書き換えは射影を変える。
        assert_ne!(
            project_plan("# Plan\n\n- [ ] Step one\n1. [ ] Step 2\n"),
            project_plan(plan)
        );
    }

    #[test]
    fn markers_inside_fences_and_comments_are_content() {
        let fenced = "```md\n- [x] literal\n```\n<!--\n- [x] note\n-->\n";
        assert_eq!(project_plan(fenced), fenced);
        // 印の後に語が続かない綴りや、箇条でない行は印ではない。
        assert_eq!(
            project_plan("- [x]done\n[x] item\n"),
            "- [x]done\n[x] item\n"
        );
    }
}
