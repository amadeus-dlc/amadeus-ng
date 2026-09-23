//! 原文と正確なReview節を持つ合成レビュー入力。実ワークフローの受領には使わない。
#![allow(dead_code)]
use core_command_domain::orchestration::{
    ReviewArtifact, ReviewBinding, ReviewCompletion, ReviewDocuments, ReviewVerdict,
};
pub(crate) fn documents(
    reviewer: &str,
    iteration: u32,
    verdict: Option<ReviewVerdict>,
) -> ReviewDocuments {
    let mut body = "# Artifact\n".to_string();
    if let Some(verdict) = verdict {
        body.push_str(&format!("\n## Review\n\n**Reviewer:** {reviewer}\n**Verdict:** {}\n**Iteration:** {iteration}\n",verdict.as_str()));
    }
    ReviewDocuments::new(
        vec![ReviewArtifact::new(
            "stage/artifact.md".into(),
            Some(body.into_bytes()),
            true,
            true,
            true,
        )],
        true,
        None,
        "a".repeat(32),
        true,
        Vec::new(),
    )
}
pub(crate) fn binding() -> ReviewBinding {
    documents("r", 1, None).bind(None).expect("合成の要求原文")
}
pub(crate) fn completion() -> ReviewCompletion {
    documents("r", 1, Some(ReviewVerdict::Ready))
        .certify(&binding(), "r", 1, ReviewVerdict::Ready, false)
        .expect("合成のReview節")
}
