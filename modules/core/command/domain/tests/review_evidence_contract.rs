//! レビュー原文の受理判断 — 不安定・欠落・改変・不正な Review 節を成功に丸めない。
//!
//! `ReviewDocuments` は入力境界が読んだ観測を束ね、要求 (`bind`) と判定 (`certify`) の
//! 両時点で原文が同じであることを指紋で保証する。ここでは拒否経路が
//! `ReviewEvidenceError` の材料を正しく選ぶことを契約として固定する。
#![allow(clippy::unwrap_used)]
use core_command_domain::orchestration::{
    ReviewArtifact, ReviewDocuments, ReviewDraft, ReviewEvidenceError, ReviewVerdict,
};

const TARGET: &str = "stage/artifact.md";
const NONCE_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn appendix(reviewer: &str, iteration: u32, verdict: ReviewVerdict) -> String {
    format!(
        "## Review\n\n**Reviewer:** {reviewer}\n**Verdict:** {}\n**Iteration:** {iteration}\n",
        verdict.as_str()
    )
}

fn target(body: &str) -> ReviewArtifact {
    ReviewArtifact::new(
        TARGET.to_string(),
        Some(body.as_bytes().to_vec()),
        true,
        true,
        true,
    )
}

fn documents(artifacts: Vec<ReviewArtifact>, stable: bool) -> ReviewDocuments {
    ReviewDocuments::new(
        artifacts,
        true,
        Some("src".to_string()),
        NONCE_A.to_string(),
        stable,
        Vec::new(),
    )
}

fn with_drafts(artifacts: Vec<ReviewArtifact>, drafts: Vec<ReviewDraft>) -> ReviewDocuments {
    ReviewDocuments::new(
        artifacts,
        true,
        Some("src".to_string()),
        NONCE_A.to_string(),
        true,
        drafts,
    )
}

#[test]
fn an_unstable_snapshot_binds_nothing_and_verifies_nothing() {
    let unstable = documents(vec![target("# Artifact\n")], false);
    assert_eq!(
        unstable.bind(None).unwrap_err(),
        ReviewEvidenceError::ArtifactsUnavailable
    );
    let request = documents(vec![target("# Artifact\n")], true)
        .bind(None)
        .unwrap();
    assert_eq!(
        unstable.verify_request(&request).unwrap_err(),
        ReviewEvidenceError::ArtifactsUnavailable
    );
}

#[test]
fn a_required_declared_artifact_that_is_not_a_regular_file_or_has_no_body_is_unavailable() {
    let missing_body = ReviewArtifact::new("stage/other.md".into(), None, true, true, false);
    assert_eq!(
        documents(vec![target("# Artifact\n"), missing_body], true)
            .bind(None)
            .unwrap_err(),
        ReviewEvidenceError::ArtifactsUnavailable
    );
    let directory = ReviewArtifact::new("stage/dir".into(), None, false, true, false);
    assert_eq!(
        documents(vec![target("# Artifact\n"), directory], true)
            .bind(None)
            .unwrap_err(),
        ReviewEvidenceError::ArtifactsUnavailable
    );
}

#[test]
fn optional_declared_artifacts_are_fingerprinted_by_their_absence_not_refused() {
    // 任意の成果物は「無い」「ファイルでない」という観測そのものが指紋に入る。
    let with_missing = documents(
        vec![
            target("# Artifact\n"),
            ReviewArtifact::new("stage/opt.md".into(), None, true, false, false),
        ],
        true,
    )
    .bind(None)
    .unwrap();
    let with_directory = documents(
        vec![
            target("# Artifact\n"),
            ReviewArtifact::new("stage/opt.md".into(), None, false, false, false),
        ],
        true,
    )
    .bind(None)
    .unwrap();
    let alone = documents(vec![target("# Artifact\n")], true)
        .bind(None)
        .unwrap();
    assert_ne!(with_missing.fingerprint(), alone.fingerprint());
    assert_ne!(with_directory.fingerprint(), alone.fingerprint());
    assert_ne!(with_missing.fingerprint(), with_directory.fingerprint());
}

#[test]
fn a_changed_artifact_or_source_no_longer_matches_the_request() {
    let request = documents(vec![target("# Artifact\n")], true)
        .bind(None)
        .unwrap();
    // 追記先より前の原文が変わると要求と一致しない (追記先より後ろは Review 節の領域)。
    assert_eq!(
        documents(vec![target("# Changed!\n")], true)
            .verify_request(&request)
            .unwrap_err(),
        ReviewEvidenceError::ArtifactsChanged
    );
    let other_source = ReviewDocuments::new(
        vec![target("# Artifact\n")],
        true,
        Some("moved".to_string()),
        NONCE_A.to_string(),
        true,
        Vec::new(),
    );
    assert_eq!(
        other_source.verify_request(&request).unwrap_err(),
        ReviewEvidenceError::SourceChanged
    );
}

#[test]
fn a_retained_prior_review_section_is_stale_and_a_replacement_must_render_the_challenge() {
    // 要求前に既に Review 節がある: 結合値はその節を pin し、challenge を発行する。
    let prior = format!(
        "# Artifact\n\n{}",
        appendix("r", 1, ReviewVerdict::NotReady)
    );
    let request = documents(vec![target(&prior)], true).bind(None).unwrap();
    assert_eq!(
        request.challenge(),
        Some(format!("review:{NONCE_A}").as_str())
    );
    // 同じバイトのまま判定を出すと、前の節が残っている。
    assert_eq!(
        documents(vec![target(&prior)], true)
            .certify(&request, "r", 2, ReviewVerdict::Ready, false)
            .unwrap_err(),
        ReviewEvidenceError::StaleAppendix
    );
    // 節を書き直しても challenge 行が無ければ受理しない。
    let replaced = format!("# Artifact\n\n{}", appendix("r", 2, ReviewVerdict::Ready));
    assert_eq!(
        documents(vec![target(&replaced)], true)
            .certify(&request, "r", 2, ReviewVerdict::Ready, false)
            .unwrap_err(),
        ReviewEvidenceError::InvalidAppendix(
            "the reviewer appendix must contain exactly one Request Challenge line matching the request".to_string()
        )
    );
    // challenge 行が逐語で並べば受理する。
    let answered = format!("{replaced}**Request Challenge:** review:{NONCE_A}\n");
    let completion = documents(vec![target(&answered)], true)
        .certify(&request, "r", 2, ReviewVerdict::Ready, false)
        .unwrap();
    assert!(documents(vec![target(&answered)], true).covers(&completion));
    assert!(!documents(vec![target(&prior)], true).covers(&completion));
}

#[test]
fn a_challenge_line_is_refused_when_the_request_issued_none() {
    let request = documents(vec![target("# Artifact\n")], true)
        .bind(None)
        .unwrap();
    assert_eq!(request.challenge(), None);
    let body = format!(
        "# Artifact\n{}**Request Challenge:** review:{NONCE_A}\n",
        appendix("r", 1, ReviewVerdict::Ready)
    );
    assert_eq!(
        documents(vec![target(&body)], true)
            .certify(&request, "r", 1, ReviewVerdict::Ready, false)
            .unwrap_err(),
        ReviewEvidenceError::InvalidAppendix(
            "the reviewer appendix must omit Request Challenge when the request did not issue one"
                .to_string()
        )
    );
}

#[test]
fn the_appendix_must_start_with_the_exact_review_heading_and_stay_terminal() {
    let request = documents(vec![target("# Artifact\n")], true)
        .bind(None)
        .unwrap();
    let wrong_heading = "# Artifact\n## Reviews\n\n**Reviewer:** r\n";
    assert_eq!(
        documents(vec![target(wrong_heading)], true)
            .certify(&request, "r", 1, ReviewVerdict::Ready, false)
            .unwrap_err(),
        ReviewEvidenceError::InvalidAppendix(
            "the appended bytes must begin with only blank lines followed by an exact `## Review` heading".to_string()
        )
    );
    let later_heading = format!(
        "# Artifact\n{}\n## Later\n",
        appendix("r", 1, ReviewVerdict::Ready)
    );
    assert_eq!(
        documents(vec![target(&later_heading)], true)
            .certify(&request, "r", 1, ReviewVerdict::Ready, false)
            .unwrap_err(),
        ReviewEvidenceError::InvalidAppendix(
            "the reviewer appendix must be terminal and contain no later rendered H1 or H2 heading"
                .to_string()
        )
    );
    let wrong_verdict = format!("# Artifact\n{}", appendix("r", 1, ReviewVerdict::NotReady));
    assert_eq!(
        documents(vec![target(&wrong_verdict)], true)
            .certify(&request, "r", 1, ReviewVerdict::Ready, false)
            .unwrap_err(),
        ReviewEvidenceError::InvalidAppendix(
            "the reviewer appendix must contain exactly one canonical verdict line matching --verdict".to_string()
        )
    );
}

#[test]
fn leading_blank_lines_with_crlf_and_tabs_are_not_part_of_the_evidence() {
    // 追記の先頭に空白行 (CRLF / タブ) があっても、証跡は `## Review` から始まる。
    let request = documents(vec![target("# Artifact")], true)
        .bind(None)
        .unwrap();
    let padded = format!(
        "# Artifact \t\r\n\r\n  \t\n{}",
        appendix("r", 1, ReviewVerdict::Ready).replace('\n', "\r\n")
    );
    let completion = documents(vec![target(&padded)], true)
        .certify(&request, "r", 1, ReviewVerdict::Ready, false)
        .unwrap();
    assert_eq!(completion.request(), &request);
}

#[test]
fn a_retried_not_ready_verdict_may_leave_the_appendix_empty() {
    let request = documents(vec![target("# Artifact\n")], true)
        .bind(None)
        .unwrap();
    let completion = documents(vec![target("# Artifact\n")], true)
        .certify(&request, "r", 1, ReviewVerdict::NotReady, true)
        .unwrap();
    assert!(documents(vec![target("# Artifact\n")], true).covers(&completion));
    // 再試行でなければ、レビューが書かれていないとして受理しない。
    assert_eq!(
        documents(vec![target("# Artifact\n")], true)
            .certify(&request, "r", 1, ReviewVerdict::NotReady, false)
            .unwrap_err(),
        ReviewEvidenceError::ReviewMissing(request.identity().attempt().to_string())
    );
}

#[test]
fn an_existing_review_section_after_crlf_is_pinned_from_its_heading() {
    let body = "# Artifact\r\n\r\n## Review\r\n\r\n**Reviewer:** r\r\n**Verdict:** READY\r\n**Iteration:** 1\r\n";
    let request = documents(vec![target(body)], true).bind(None).unwrap();
    assert_eq!(request.appendix_artifact(), TARGET);
    assert_eq!(request.appendix_offset(), "# Artifact\r\n".len());
    assert!(request.prior_length() > 0);
}

/// 2.8.2 — reviewer は成果物へ追記せず、依頼の試行の下書きへレビューを書く。
#[test]
fn a_standalone_review_in_the_requests_attempt_is_the_review() {
    let request = documents(vec![target("# Artifact\n")], true)
        .bind(None)
        .unwrap();
    let attempt = request.identity().attempt().to_string();
    // テンプレートどおり `## Review` で始めても、見出し無しでもよい。
    for review in [
        appendix("r", 1, ReviewVerdict::Ready),
        "**Reviewer:** r\n**Verdict:** READY\n**Iteration:** 1\n".to_string(),
    ] {
        let completion = with_drafts(
            vec![target("# Artifact\n")],
            vec![ReviewDraft::new(attempt.clone(), review.into_bytes())],
        )
        .certify(&request, "r", 1, ReviewVerdict::Ready, false)
        .unwrap();
        assert_eq!(completion.request(), &request);
    }
}

#[test]
fn a_draft_from_another_attempt_is_not_this_requests_review() {
    let request = documents(vec![target("# Artifact\n")], true)
        .bind(None)
        .unwrap();
    let elsewhere = ReviewDraft::new(
        "0123456789abcdef".to_string(),
        appendix("r", 1, ReviewVerdict::Ready).into_bytes(),
    );
    assert_eq!(
        with_drafts(vec![target("# Artifact\n")], vec![elsewhere])
            .certify(&request, "r", 1, ReviewVerdict::Ready, false)
            .unwrap_err(),
        ReviewEvidenceError::ReviewMissing(request.identity().attempt().to_string())
    );
}

#[test]
fn a_review_file_and_an_appended_section_together_are_refused() {
    let request = documents(vec![target("# Artifact\n")], true)
        .bind(None)
        .unwrap();
    let appended = format!("# Artifact\n{}", appendix("r", 1, ReviewVerdict::Ready));
    let draft = ReviewDraft::new(
        request.identity().attempt().to_string(),
        appendix("r", 1, ReviewVerdict::Ready).into_bytes(),
    );
    assert_eq!(
        with_drafts(vec![target(&appended)], vec![draft])
            .certify(&request, "r", 1, ReviewVerdict::Ready, false)
            .unwrap_err(),
        ReviewEvidenceError::ReviewWrittenTwice
    );
}

#[test]
fn a_standalone_review_is_validated_like_an_appended_one() {
    let request = documents(vec![target("# Artifact\n")], true)
        .bind(None)
        .unwrap();
    let draft = ReviewDraft::new(
        request.identity().attempt().to_string(),
        appendix("someone-else", 1, ReviewVerdict::Ready).into_bytes(),
    );
    assert_eq!(
        with_drafts(vec![target("# Artifact\n")], vec![draft])
            .certify(&request, "r", 1, ReviewVerdict::Ready, false)
            .unwrap_err(),
        ReviewEvidenceError::InvalidAppendix(
            "the reviewer appendix must contain exactly one Reviewer line matching the requested reviewer"
                .to_string()
        )
    );
}

#[test]
fn a_later_request_in_the_same_attempt_keeps_the_attempt_id() {
    let first = documents(vec![target("# Artifact\n")], true)
        .bind(None)
        .unwrap();
    let second = ReviewDocuments::new(
        vec![target("# Artifact v2\n")],
        true,
        Some("src".to_string()),
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
        true,
        Vec::new(),
    )
    .bind(Some(first.identity()))
    .unwrap();
    assert_eq!(second.identity().attempt(), first.identity().attempt());
    assert_ne!(
        second.identity().request_id(),
        first.identity().request_id()
    );
}
