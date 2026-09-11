//! 計画承認の質問文から読み取る、可視の最終確認節。
use core_infrastructure::ecmascript::trim;
/// 文書内の回答は人間応答の受領とは別であり、この値だけでは実行を許可しない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanQuestions {
    answer: Option<String>,
    approved: bool,
    pending: bool,
    fingerprint: Option<String>,
}
impl Default for PlanQuestions {
    fn default() -> Self {
        Self::parse("")
    }
}
impl PlanQuestions {
    /// 質問文書の可視の最終確認節を解析する。
    #[must_use]
    pub fn parse(content: &str) -> Self {
        let mut in_approval = false;
        let mut awaiting_label = false;
        let mut answer = None;
        let mut fingerprint = None;
        let lines = super::summary_questions::visible_markdown_lines(content);
        for line in &lines {
            if let Some(title) = heading(line) {
                let numbered = after_number(title);
                let label = numbered
                    .filter(|rest| rest.starts_with([':', '.', ')', '-']))
                    .map_or(title, |rest| {
                        rest.get(1..)
                            .unwrap_or_default()
                            .trim_start_matches([' ', '\t'])
                    });
                in_approval = is_label(label);
                awaiting_label = !in_approval
                    && numbered.is_some_and(|rest| {
                        rest.trim_matches([' ', '\t', '.', ':', ')', '-'])
                            .is_empty()
                    });
                if in_approval {
                    answer = None;
                    fingerprint = None;
                }
                continue;
            }
            if awaiting_label && !trim(line).is_empty() {
                awaiting_label = false;
                in_approval = is_label(line);
                if in_approval {
                    answer = None;
                    fingerprint = None;
                }
            }
            if !in_approval {
                continue;
            }
            if let Some(value) = line.strip_prefix("[Answer]:") {
                answer = Some(trim(value));
            }
            if let Some(value) = line.strip_prefix("[Approval Fingerprint]:") {
                let value = value.trim_matches([' ', '\t']);
                if valid_fingerprint(value) {
                    fingerprint = Some(value.to_string());
                } else if value.is_empty() {
                    fingerprint = None;
                }
            }
        }
        Self {
            answer: answer.map(str::to_string),
            approved: answer.is_some_and(is_approved),
            pending: answer.is_some_and(|value| value.bytes().all(|byte| byte == b'_')),
            fingerprint,
        }
    }
    /// 最終確認節の回答原文。厳密な受領には選択肢の別名を変換せず照合する。
    #[must_use]
    pub fn answer(&self) -> Option<&str> {
        self.answer.as_deref()
    }

    /// 文書に承認の回答があるか。
    #[must_use]
    pub const fn approved(&self) -> bool {
        self.approved
    }
    /// 文書の回答欄が未記入か。
    #[must_use]
    pub const fn pending(&self) -> bool {
        self.pending
    }
    /// 文書が掲げる承認対象の指紋。
    #[must_use]
    pub fn fingerprint(&self) -> Option<&str> {
        self.fingerprint.as_deref()
    }
}

fn valid_fingerprint(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hash| {
        hash.len() == 64
            && hash
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn heading(line: &str) -> Option<&str> {
    let count = line.bytes().take_while(|byte| *byte == b'#').count();
    if !(1..=6).contains(&count) {
        return None;
    }
    let rest = line.get(count..)?;
    if !rest.starts_with([' ', '\t']) {
        return None;
    }
    let title = trim(
        rest.trim_start_matches([' ', '\t'])
            .trim_end_matches([' ', '\t', '#']),
    );
    if title.is_empty() { None } else { Some(title) }
}
fn after_number(title: &str) -> Option<&str> {
    let lower = title.to_ascii_lowercase();
    let skip = if lower.starts_with("question") {
        8
    } else {
        usize::from(lower.starts_with('q'))
    };
    let number = title.get(skip..)?.trim_start_matches([' ', '\t']);
    let count = number.bytes().take_while(u8::is_ascii_digit).count();
    if count == 0 {
        return None;
    }
    Some(number.get(count..)?.trim_start_matches([' ', '\t']))
}
fn is_label(value: &str) -> bool {
    let mut label = trim(value);
    if let Some(value) = label.strip_suffix(['?', ':']) {
        label = trim(value);
    }
    for marker in ["**", "__", "*", "_"] {
        if let Some(inner) = label
            .strip_prefix(marker)
            .and_then(|value| value.strip_suffix(marker))
        {
            label = trim(inner);
            break;
        }
    }
    label.eq_ignore_ascii_case("plan approval")
}

fn is_approved(value: &str) -> bool {
    let bytes = value.as_bytes();
    let value = if bytes.first().is_some_and(u8::is_ascii_alphabetic)
        && bytes.get(1).is_some_and(|byte| matches!(byte, b'.' | b')'))
    {
        value
            .get(2..)
            .unwrap_or_default()
            .trim_start_matches([' ', '\t'])
    } else {
        value
    };
    let value = value.strip_prefix(['\"', '\'']).unwrap_or(value);
    let value = value.strip_suffix(['\"', '\'']).unwrap_or(value);
    value.eq_ignore_ascii_case("approve plan")
}
