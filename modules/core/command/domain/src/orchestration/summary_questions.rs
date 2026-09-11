//! 人間が確認する質問内容と、その意味上のハッシュ。
use super::SummaryQuestionsError;
use core_infrastructure::ecmascript::{trim, trim_end};
mod containers;
mod headings;
mod visibility;
use headings::{HeadingStyle, hash_heading, heading, question_id};
use std::collections::BTreeSet;
use visibility::visible_lines;
/// 他の保護された確認節にも同じ可視性境界を使う。
pub(super) fn visible_markdown_lines(content: &str) -> Vec<String> {
    let normalized = content.replace("\r\n", "\n").replace('\r', "\n");
    visible_lines(&normalized.split('\n').collect::<Vec<_>>())
        .into_iter()
        .map(|line| line.replace('\0', ""))
        .collect()
}
const SUMMARY: &str = "Consolidated Summary Confirmation";
/// 確認節の回答を検証済みの質問文書。ファイルの読書きは持たない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryQuestions {
    answer: String,
    sha256: String,
}
impl SummaryQuestions {
    /// 回答行を検査し、confirmed-content-v1の範囲をハッシュする。
    ///
    /// # Errors
    /// 回答が一致しない、重複する節や許されない後続見出しがある場合。
    pub fn parse(content: &str, expected_answer: &str) -> Result<Self, SummaryQuestionsError> {
        let normalized = content.replace("\r\n", "\n").replace('\r', "\n");
        let lines: Vec<&str> = normalized.split('\n').collect();
        let visible = visible_lines(&lines);
        let mut in_summary = false;
        let mut answers = Vec::new();
        for line in &visible {
            let answer_line = line.replace('\0', "");
            if let Some((2, title)) = heading(line) {
                if in_summary {
                    break;
                }
                in_summary = title == SUMMARY;
            } else if in_summary && let Some(answer) = answer_line.strip_prefix("[Answer]:") {
                answers.push(trim(answer).to_string());
            }
        }
        if !in_summary || answers.as_slice() != [expected_answer] {
            return Err(SummaryQuestionsError::InvalidAnswer);
        }
        let confirmed = confirmed_lines(&lines, &visible)?;
        let sha256 = core_infrastructure::hash::sha256_hex(trim_end(&confirmed).as_bytes());
        Ok(Self {
            answer: expected_answer.to_string(),
            sha256,
        })
    }
    /// 検証した回答。
    #[must_use]
    pub fn answer(&self) -> &str {
        &self.answer
    }
    /// confirmed-content-v1のSHA-256。
    #[must_use]
    pub fn sha256(&self) -> &str {
        &self.sha256
    }
}
fn confirmed_lines(lines: &[&str], visible: &[String]) -> Result<String, SummaryQuestionsError> {
    let mut saw_summary = false;
    let mut saw_assumption = false;
    let mut excluded = false;
    let mut questions = BTreeSet::new();
    let mut included = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if let Some((style, level, title)) = hash_heading(visible, index) {
            let atx_h2 = style == HeadingStyle::Atx && level == 2;
            if atx_h2 && title == SUMMARY {
                if saw_summary {
                    return Err(SummaryQuestionsError::InvalidStructure(format!(
                        "duplicate H2 section \"{SUMMARY}\""
                    )));
                }
                saw_summary = true;
            } else if saw_summary && atx_h2 && title == "Assumption Confirmation" {
                if saw_assumption {
                    return Err(SummaryQuestionsError::InvalidStructure(
                        "duplicate H2 section \"Assumption Confirmation\"".to_string(),
                    ));
                }
                saw_assumption = true;
                excluded = true;
            } else if atx_h2
                && (question_id(&title).is_some()
                    || saw_summary && title == "Requested Changes Feedback")
            {
                if let Some(id) = question_id(&title)
                    && !questions.insert(id.to_string())
                {
                    return Err(SummaryQuestionsError::InvalidStructure(format!(
                        "duplicate H2 section \"{id}\""
                    )));
                }
                excluded = false;
            } else if saw_summary {
                let boundary = if saw_assumption {
                    "after \"Assumption Confirmation\"; only Q<n> or \"Requested Changes Feedback\" sections may follow"
                } else {
                    "after the consolidated summary; only Q<n>, \"Requested Changes Feedback\", or one \"Assumption Confirmation\" section may follow"
                };
                return Err(SummaryQuestionsError::InvalidStructure(format!(
                    "unsupported {}H{level} heading \"{title}\" {boundary}",
                    match style {
                        HeadingStyle::Atx | HeadingStyle::NestedAtx => "",
                        HeadingStyle::Setext => "Setext ",
                        HeadingStyle::Html => "HTML ",
                    }
                )));
            }
        }
        if !excluded {
            included.push(*line);
        }
    }
    if !saw_summary {
        return Err(SummaryQuestionsError::InvalidStructure(format!(
            "missing required H2 section \"{SUMMARY}\""
        )));
    }
    Ok(included.join("\n"))
}
