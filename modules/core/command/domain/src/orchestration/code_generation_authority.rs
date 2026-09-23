//! 計画承認を現在の指示発行へ結び付ける値。
use super::{ActiveDirective, CodeGenerationRunFloor, PlanApprovalError, PublishedDirective};
use core_infrastructure::canon_json::{JsonValue, Number, ObjectMembers, hash_canonical};
/// 計画承認の対象と、発行時点の識別子。文書だけでは構築しない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeGenerationAuthority {
    directive_epoch: String,
    target_id: String,
    intent_id: String,
    unit: Option<String>,
    run_floor: String,
    source_floor: String,
    marker_revision: u64,
}
impl CodeGenerationAuthority {
    const fn of_values(
        directive_epoch: String,
        target_id: String,
        intent_id: String,
        unit: Option<String>,
        run_floor: String,
        source_floor: String,
        marker_revision: u64,
    ) -> Self {
        Self {
            directive_epoch,
            target_id,
            intent_id,
            unit,
            run_floor,
            source_floor,
            marker_revision,
        }
    }

    /// 記録された発行の値を、対象と指紋の不変条件を確認して組む。
    /// # Errors
    /// 発行識別子・ソース基準・実行境界が不正な場合。
    pub fn new(
        target: &super::PlanTarget,
        intent: &super::IntentId,
        epoch: String,
        run_floor: String,
        source_floor: String,
        revision: u64,
    ) -> Result<Self, PlanApprovalError> {
        let hexadecimal = |value: &str| {
            value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        };
        let epoch_valid = epoch
            .strip_prefix("sha256:")
            .is_some_and(|hex| hex.len() == 64 && hexadecimal(hex));
        let source_valid = source_floor == "unbindable"
            || (matches!(source_floor.len(), 40 | 64) && hexadecimal(&source_floor));
        let floor_valid = run_floor == "unstarted#0"
            || run_floor.rsplit_once('#').is_some_and(|(boundary, count)| {
                count.parse::<u64>().is_ok_and(|count| count > 0)
                    && boundary.split_once(':').is_some_and(|(kind, time)| {
                        matches!(
                            kind,
                            "WORKFLOW_STARTED" | "STAGE_STARTED" | "STAGE_JUMPED" | "GATE_REJECTED"
                        ) && chrono::DateTime::parse_from_rfc3339(time).is_ok()
                    })
            });
        if !epoch_valid || !source_valid || !floor_valid {
            return Err(PlanApprovalError::new(
                "invalid recorded Code Generation authority",
            ));
        }
        Ok(Self::of_values(
            epoch,
            target.id(),
            intent.as_str().to_string(),
            target.unit().map(str::to_string),
            run_floor,
            source_floor,
            revision,
        ))
    }

    /// 発行済み指示と集約の実行境界から承認対象を解決する。
    /// # Errors
    /// 指示が欠落、対象不一致、または発行が古い場合。
    pub fn resolve(
        directive: Option<&ActiveDirective>,
        floor: Option<&CodeGenerationRunFloor>,
        unit: Option<&str>,
        state_sha256: Option<&str>,
    ) -> Result<Self, PlanApprovalError> {
        let state = state_sha256.ok_or_else(|| {
            PlanApprovalError::new(
                "Code Generation approval authority requires an active workflow state",
            )
        })?;
        let unavailable = || {
            PlanApprovalError::new(
                "Code Generation approval authority is unavailable because the active directive is missing, stale, or legacy; run a fresh `next`",
            )
        };
        let directive = directive
            .filter(|directive| directive.state_sha256() == state)
            .ok_or_else(unavailable)?;
        let floor = floor.ok_or_else(unavailable)?;
        let stage = directive.directive().stage().as_str();
        if stage != "code-generation" {
            return Err(PlanApprovalError::new(format!(
                "Code Generation approval authority does not match active directive stage \"{stage}\""
            )));
        }
        let PublishedDirective::RunStage {
            unit: issued_unit, ..
        } = directive.directive()
        else {
            let kind = if matches!(directive.directive(), PublishedDirective::Error { .. }) {
                "error"
            } else {
                "load-steering"
            };
            return Err(PlanApprovalError::new(format!(
                "Code Generation approval authority requires a run-stage or invoke-swarm directive, got \"{kind}\""
            )));
        };
        if let Some(unit) = unit {
            if issued_unit.as_deref() != Some(unit) {
                return Err(PlanApprovalError::new(format!(
                    "Code Generation approval target unit \"{unit}\" does not match active directive unit \"{}\"",
                    issued_unit.as_deref().unwrap_or("(none)")
                )));
            }
        } else if issued_unit.is_some() {
            return Err(PlanApprovalError::new(
                "Stage-level Code Generation approval requires a zero-Unit run-stage directive",
            ));
        }
        let target_id = unit.map_or_else(
            || "stage:code-generation".to_string(),
            |unit| format!("unit:{unit}"),
        );
        let source_floor = directive.source_floor().unwrap_or("unbindable").to_string();
        let text = |text: &str| JsonValue::String(text.to_string());
        let mut fields = ObjectMembers::new();
        fields.insert("version", JsonValue::Number(Number::PosInt(2)));
        fields.insert("project", text(directive.project_sha256()));
        fields.insert("intent", text(directive.intent_id().as_str()));
        fields.insert("state", text(directive.state_sha256()));
        fields.insert("stage", text(stage));
        fields.insert(
            "directive_unit",
            issued_unit.as_deref().map_or(JsonValue::Null, text),
        );
        fields.insert("kind", text("run-stage"));
        fields.insert(
            "issuance_revision",
            JsonValue::Number(Number::PosInt(directive.revision())),
        );
        fields.insert(
            "owner_epoch",
            JsonValue::Number(Number::PosInt(directive.owner_epoch())),
        );
        fields.insert(
            "context_epoch",
            JsonValue::Number(Number::PosInt(directive.context_epoch())),
        );
        fields.insert("continue_token", JsonValue::Null);
        fields.insert("target", text(&target_id));
        fields.insert("source_floor", text(&source_floor));
        let directive_epoch = hash_canonical(&JsonValue::Object(fields)).rendered();
        Ok(Self::of_values(
            directive_epoch,
            target_id,
            directive.intent_id().as_str().to_string(),
            unit.map(str::to_string),
            floor.rendered(),
            source_floor,
            directive.revision(),
        ))
    }
    /// 計画・テスト指示・解決済み契約を、この承認対象へ束縛する（`sha256:<hex>`）。
    ///
    /// 計画は 2.8.2 `projectPlanApprovalContent` と同じ射影を通してから束ねる — 承認後に
    /// 段自身が命じる編集（開発者が進捗の印 `[x]` を付ける）と、旧手順が末尾へ足した
    /// `## Review` 節と、編集器が生む空白の差だけを消し、それ以外はバイトどおりに束ねる。
    /// テスト指示は改行だけを正規化する（`projectInstructionsContent`）。
    ///
    /// 2.8.2 の `sha256:v3:` は内容・対象・intent・試行床だけを束ねるが、この build は指示の
    /// 発行エポックとソース床も束ねる（より厳しい側。ソースのずれは 2.8.2 では
    /// `[Planned Source]` と Change Control が扱う）。束ねる中身が違うので v3 を名乗らない。
    #[must_use]
    pub fn approval_fingerprint(
        &self,
        plan: &str,
        instructions: &str,
        contract_hash: &str,
    ) -> String {
        let plan = project_plan(plan);
        let instructions = instructions.replace("\r\n", "\n").replace('\r', "\n");
        let mut fields = ObjectMembers::new();
        for (name, value) in [
            ("plan", plan.as_str()),
            ("instructions", instructions.as_str()),
            ("testing_contract", contract_hash),
            ("target", self.target_id()),
            ("intent", self.intent_id()),
            ("directive_epoch", self.directive_epoch()),
            ("run_floor", self.run_floor()),
            ("source_floor", self.source_floor()),
        ] {
            fields.insert(name, JsonValue::String(value.to_string()));
        }
        hash_canonical(&JsonValue::Object(fields)).rendered()
    }

    /// 指示の発行エポック。
    #[must_use]
    pub fn directive_epoch(&self) -> &str {
        &self.directive_epoch
    }
    /// 承認の対象。
    #[must_use]
    pub fn target_id(&self) -> &str {
        &self.target_id
    }
    /// 依頼の識別子。
    #[must_use]
    pub fn intent_id(&self) -> &str {
        &self.intent_id
    }
    /// 個別作業単位。段階全体ではNone。
    #[must_use]
    pub fn unit(&self) -> Option<&str> {
        self.unit.as_deref()
    }
    /// 試行の境界。
    #[must_use]
    pub fn run_floor(&self) -> &str {
        &self.run_floor
    }
    /// 指示が束縛したソースの基準。
    #[must_use]
    pub fn source_floor(&self) -> &str {
        &self.source_floor
    }
    /// 発行回数。
    #[must_use]
    pub const fn marker_revision(&self) -> u64 {
        self.marker_revision
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
    use super::project_plan;

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
