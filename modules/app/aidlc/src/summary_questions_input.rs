//! 内容確認のファイル入力。配置はここで検査し、文書の意味はドメインへ渡す。
use crate::layout::Layout;
use crate::lexical_path::normalize;
use core_command_domain::orchestration::{
    SummaryEvidence, SummaryQuestions, SummaryQuestionsError,
};

pub(crate) fn read(
    layout: &Layout,
    supplied: Option<&str>,
    expected: &str,
) -> Result<SummaryEvidence, String> {
    let supplied = supplied.filter(|value| !value.is_empty()).ok_or_else(|| "Summary confirmation requires --questions-file <path> so the receipt can bind to the reviewed answers.".to_string())?;
    let absolute = normalize(&layout.project_dir().join(supplied));
    if layout
        .record_dir()
        .is_none_or(|root| !absolute.starts_with(root))
    {
        return Err(format!(
            "Summary confirmation questions file must be inside the active intent record: {supplied}"
        ));
    }
    if !absolute
        .to_str()
        .is_some_and(|path| path.ends_with("-questions.md"))
        || !absolute.exists()
    {
        return Err(format!(
            "Summary confirmation questions file does not exist: {supplied}"
        ));
    }
    let content = std::fs::read_to_string(&absolute).map_err(|error| error.to_string())?;
    let questions = SummaryQuestions::parse(&content, expected).map_err(|error| match error {
        SummaryQuestionsError::InvalidAnswer => format!("Summary confirmation section in {supplied} must contain exactly one `[Answer]:` line with {} before this command runs.", if expected.is_empty() { "a blank value" } else { expected }),
        SummaryQuestionsError::InvalidStructure(detail) => format!("Summary confirmation questions file {supplied} is invalid: {detail}."),
    })?;
    let relative = absolute
        .strip_prefix(layout.project_dir())
        .map_err(|error| error.to_string())?
        .to_str()
        .ok_or_else(|| "Summary confirmation path is not UTF-8".to_string())?
        .replace('\\', "/");
    SummaryEvidence::new(relative, questions.sha256()).map_err(|error| error.to_string())
}
