//! codekb (リポジトリごとの durable な知識ストア) のドメイン契約 — 群 D。
//!
//! `codekb-snapshot` / `codekb-publish` が運ぶ 3 つの値と、公開の可否を決める集約の判断を
//! 固定する。**比較の意味論は upstream に合わせる**のが要点である — 呼び手が渡した
//! compare-and-swap の合言葉 (`--expect-store` / `--expect-source`) を upstream は検証せず
//! 文字列として突き合わせるので、こちらも綴りを検査して落とさない (落とすと upstream が
//! `CODEKB_STORE_CHANGED` を答える場面で native だけが別の拒否を返す)。
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use core_command_domain::workspace::{CodekbGeneration, CodekbRepoId, CodekbSourceFingerprint};

// ---------------------------------------------------------------------------
// CodekbRepoId — 生のまま join() に届いてはならないパス片
// ---------------------------------------------------------------------------

/// upstream `REPO_NAME_REGEX` (`/^[A-Za-z0-9][A-Za-z0-9._-]*$/`) を受ける。
#[test]
fn a_repo_id_is_one_path_segment_starting_with_an_alphanumeric() {
    for accepted in ["demo-repo", "svc-a", "A1", "a.b_c", "x", "0", "a-b.c_d"] {
        assert!(
            CodekbRepoId::parse(accepted).is_ok(),
            "受理されるはず: {accepted}"
        );
    }
    assert_eq!(CodekbRepoId::parse("svc-a").unwrap().as_str(), "svc-a");
}

/// パスの罠は `join()` に届く前にここで落ちる。
#[test]
fn a_repo_id_rejects_every_path_hazard() {
    for rejected in [
        "", ".", "..", "a/b", "a\\b", "-x", "_x", ".hidden", "a b", "a\nb", "../x",
    ] {
        assert!(
            CodekbRepoId::parse(rejected).is_err(),
            "拒否されるはず: {rejected:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// CodekbGeneration — ストア世代 (compare-and-swap の片側)
// ---------------------------------------------------------------------------

/// ストアが無いことも 1 つの世代である (upstream 逐語 `none`)。
#[test]
fn an_absent_store_has_the_none_generation() {
    assert_eq!(CodekbGeneration::absent().as_str(), "none");
}

/// 在るストアの世代は木のハッシュを `sha256:` で名乗る (upstream 逐語)。
#[test]
fn a_present_store_names_its_tree_hash_with_the_sha256_prefix() {
    let generation = CodekbGeneration::of_tree_hash("abc123");
    assert_eq!(generation.as_str(), "sha256:abc123");
    assert_ne!(generation, CodekbGeneration::absent());
    assert_eq!(generation, CodekbGeneration::of_tree_hash("abc123"));
}

/// 呼び手が渡した合言葉は**検査しない** — upstream は綴りを見ずに突き合わせるだけである。
#[test]
fn a_caller_supplied_generation_token_is_carried_verbatim_without_validation() {
    let token = CodekbGeneration::of_token("bogus-not-a-hash").unwrap();
    assert_eq!(token.as_str(), "bogus-not-a-hash");
    assert_ne!(token, CodekbGeneration::absent());

    // 「ストアが無い」を指す合言葉は、実測した不在と等しくなる。
    assert_eq!(
        CodekbGeneration::of_token("none").unwrap(),
        CodekbGeneration::absent()
    );
    // 実測した世代と同じ綴りなら等しい。
    assert_eq!(
        CodekbGeneration::of_token("sha256:abc123").unwrap(),
        CodekbGeneration::of_tree_hash("abc123")
    );
}

/// 空の合言葉だけは合言葉ではない (upstream も `!expectedStore` で使い方の誤りとして落とす)。
#[test]
fn an_empty_generation_token_is_refused() {
    assert!(CodekbGeneration::of_token("").is_err());
}

// ---------------------------------------------------------------------------
// CodekbSourceFingerprint — 源の指紋 (compare-and-swap のもう片側)
// ---------------------------------------------------------------------------

/// git の作業ツリーから採れたら `git:`、採れなければ木のハッシュで `tree:` (upstream 逐語)。
#[test]
fn a_source_fingerprint_names_which_half_produced_it() {
    assert_eq!(
        CodekbSourceFingerprint::of_git("deadbeef").as_str(),
        "git:deadbeef"
    );
    assert_eq!(
        CodekbSourceFingerprint::of_tree("cafebabe").as_str(),
        "tree:cafebabe"
    );
    assert_ne!(
        CodekbSourceFingerprint::of_git("x"),
        CodekbSourceFingerprint::of_tree("x"),
        "同じハッシュでも由来が違えば別の指紋である"
    );
}

/// 合言葉は世代と同じく逐語で運び、検査しない。
#[test]
fn a_caller_supplied_source_token_is_carried_verbatim_without_validation() {
    let token = CodekbSourceFingerprint::of_token("whatever").unwrap();
    assert_eq!(token.as_str(), "whatever");
    assert_eq!(
        CodekbSourceFingerprint::of_token("git:deadbeef").unwrap(),
        CodekbSourceFingerprint::of_git("deadbeef")
    );
    assert!(CodekbSourceFingerprint::of_token("").is_err());
}

// ---------------------------------------------------------------------------
// 走査範囲のパスと被覆 — 公開の可否を分ける純粋な判断
// ---------------------------------------------------------------------------

use core_command_domain::workspace::{
    CodekbArtifact, CodekbArtifactName, CodekbArtifacts, CodekbCandidate, CodekbScopePath,
    CodekbScopePaths,
};
use core_infrastructure::collections::FirstClassCollection as _;

fn scope_paths(values: &[&str]) -> CodekbScopePaths {
    CodekbScopePaths::of(
        values
            .iter()
            .map(|value| CodekbScopePath::parse(value).expect("フィクスチャのパスは文法内"))
            .collect(),
    )
}

/// 走査範囲のパスは空にできない (綴りはリポジトリ相対のまま保つ)。
#[test]
fn a_scope_path_keeps_its_spelling_and_refuses_to_be_empty() {
    assert_eq!(CodekbScopePath::parse("src/").unwrap().as_str(), "src/");
    assert_eq!(CodekbScopePath::parse("./").unwrap().as_str(), "./");
    assert!(CodekbScopePath::parse("").is_err());
    assert!(CodekbScopePath::parse("   ").is_err());
}

/// 被覆は upstream `scopePathCovered` の逐語 — 完全一致か、`/` で終わる**取込側**の
/// ディレクトリが接頭辞として飲み込むかの 2 つだけである (glob ではない)。
#[test]
fn coverage_is_a_literal_match_or_a_trailing_slash_directory_prefix() {
    let incoming = scope_paths(&["src/"]);
    assert!(incoming.covers(&CodekbScopePath::parse("src/").unwrap()));
    assert!(incoming.covers(&CodekbScopePath::parse("src/payments/").unwrap()));
    assert!(incoming.covers(&CodekbScopePath::parse("src/main.rs").unwrap()));
    assert!(!incoming.covers(&CodekbScopePath::parse("docs/").unwrap()));

    // `/` で終わらない綴りは接頭辞として飲み込まない — 完全一致だけである。
    let bare = scope_paths(&["src"]);
    assert!(bare.covers(&CodekbScopePath::parse("src").unwrap()));
    assert!(
        !bare.covers(&CodekbScopePath::parse("src/main.rs").unwrap()),
        "`src` は `src/main.rs` を飲み込まない"
    );
}

/// リポジトリ根 (`./`) を名乗る走査は、何を分析していても被覆する。
#[test]
fn a_root_claim_covers_every_analyzed_path() {
    assert!(scope_paths(&["./"]).claims_root());
    assert!(!scope_paths(&["src/"]).claims_root());
}

// ---------------------------------------------------------------------------
// 9 成果物 — 過不足のない集合だけが候補になれる
// ---------------------------------------------------------------------------

/// upstream `CODEKB_ARTIFACT_FILES` の 9 綴り。
#[test]
fn the_nine_canonical_artifact_names_are_the_only_ones() {
    let all = CodekbArtifactName::all();
    assert_eq!(all.len(), 9);
    let spelled: Vec<&str> = all.iter().map(CodekbArtifactName::as_str).collect();
    assert_eq!(
        spelled,
        vec![
            "api-documentation.md",
            "architecture.md",
            "business-overview.md",
            "code-quality-assessment.md",
            "code-structure.md",
            "component-inventory.md",
            "dependencies.md",
            "reverse-engineering-timestamp.md",
            "technology-stack.md",
        ],
        "綴りも並びも upstream の逐語 (辞書順)"
    );
    assert!(CodekbArtifactName::parse("architecture.md").is_ok());
    assert!(CodekbArtifactName::parse("README.md").is_err());
    assert!(CodekbArtifactName::parse("").is_err());
}

fn nine_artifacts() -> Vec<CodekbArtifact> {
    CodekbArtifactName::all()
        .iter()
        .map(|name| CodekbArtifact::new(*name, format!("# {}\n", name.as_str()).into_bytes()))
        .collect()
}

/// 9 つちょうどでなければ候補にならない (過不足のどちらも拒否)。
#[test]
fn the_artifact_set_must_be_exactly_the_canonical_nine() {
    let complete = CodekbArtifacts::of(nine_artifacts()).expect("9 つちょうど");
    assert_eq!(complete.len(), 9);

    let mut short = nine_artifacts();
    short.pop();
    assert!(CodekbArtifacts::of(short).is_err(), "8 つでは足りない");

    let mut duplicated = nine_artifacts();
    duplicated.push(CodekbArtifact::new(
        CodekbArtifactName::parse("architecture.md").unwrap(),
        b"dup".to_vec(),
    ));
    assert!(CodekbArtifacts::of(duplicated).is_err(), "重複は過剰である");
}

/// 入力の順序によらず、成果物は常に辞書順で並ぶ (公開のバイトを決定的にする)。
#[test]
fn the_artifacts_are_ordered_canonically_whatever_the_input_order() {
    let mut reversed = nine_artifacts();
    reversed.reverse();
    let artifacts = CodekbArtifacts::of(reversed).expect("9 つちょうど");
    let first = artifacts.at(0).expect("先頭");
    assert_eq!(first.name().as_str(), "api-documentation.md");
    let names = artifacts.fold_left(Vec::new(), |mut acc, artifact| {
        acc.push(artifact.name().as_str().to_string());
        acc
    });
    assert_eq!(
        names,
        CodekbArtifacts::of(nine_artifacts()).unwrap().fold_left(
            Vec::new(),
            |mut acc, artifact| {
                acc.push(artifact.name().as_str().to_string());
                acc
            }
        )
    );
}

// ---------------------------------------------------------------------------
// 候補 — 9 成果物 + 走査範囲の主張
// ---------------------------------------------------------------------------

fn candidate(analyzed: &[&str], fingerprint: Option<&str>) -> CodekbCandidate {
    CodekbCandidate::new(
        CodekbArtifacts::of(nine_artifacts()).expect("9 つちょうど"),
        scope_paths(analyzed),
        fingerprint.map(str::to_string),
    )
}

/// snapshot が取った範囲が候補の主張を覆っていなければ、覆えていない最初のパスを答える。
#[test]
fn the_candidate_names_the_first_analyzed_path_the_snapshot_did_not_cover() {
    let subject = candidate(&["src/", "docs/"], Some("abc"));
    assert_eq!(
        subject
            .uncovered_by(&scope_paths(&["src/"]))
            .map(CodekbScopePath::as_str),
        Some("docs/"),
        "docs/ は src/ に覆われない"
    );
    assert_eq!(subject.uncovered_by(&scope_paths(&["src/", "docs/"])), None);
}

/// snapshot が根 (`./`) を取っていれば、候補の主張は必ず覆われる (upstream の短絡)。
#[test]
fn a_root_snapshot_covers_the_candidate_without_checking_each_path() {
    let subject = candidate(&["src/", "docs/", "anything/"], Some("abc"));
    assert_eq!(subject.uncovered_by(&scope_paths(&["./"])), None);
}

// ---------------------------------------------------------------------------
// 集約 Codekb — 公開の可否 (compare-and-swap) と、snapshot が示す 2 つの世代
// ---------------------------------------------------------------------------

use core_command_domain::workspace::{Codekb, CodekbEvent, CodekbPublishRefusal};

fn repo() -> CodekbRepoId {
    CodekbRepoId::parse("demo-repo").expect("フィクスチャの repo 識別子は文法内")
}

fn store_of(generation: CodekbGeneration) -> Codekb {
    Codekb::observed(repo(), generation)
}

fn source() -> CodekbSourceFingerprint {
    CodekbSourceFingerprint::of_git("3c55f6af")
}

/// snapshot は「ストアの世代」と「源の指紋」の 2 つを、走査対象のパスと一緒に示す。
#[test]
fn a_snapshot_presents_both_generations_with_the_paths_they_were_taken_over() {
    let store = store_of(CodekbGeneration::absent());
    let snapshot = store.take_snapshot(scope_paths(&["src/"]), source());
    assert_eq!(snapshot.repo().as_str(), "demo-repo");
    assert_eq!(snapshot.store_generation().as_str(), "none");
    assert_eq!(snapshot.source_fingerprint().as_str(), "git:3c55f6af");
    assert_eq!(
        snapshot.paths().at(0).map(CodekbScopePath::as_str),
        Some("src/")
    );
}

/// 3 つの合言葉がすべて噛み合えば、公開の事実が 1 件返る (1 コマンド 1 イベント)。
#[test]
fn a_matching_compare_and_swap_yields_one_published_event() {
    let mut store = store_of(CodekbGeneration::absent());
    let event = store
        .publish(
            candidate(&["src/"], Some("3c55f6af")),
            &CodekbGeneration::absent(),
            &source(),
            Some(&source()),
            Some("3c55f6af"),
        )
        .expect("噛み合えば公開できる");
    let CodekbEvent::Published(published) = &event else {
        panic!("公開の事実は Published である");
    };
    assert_eq!(published.aggregate_id(), &repo());
    assert_eq!(published.artifacts().len(), 9, "9 成果物を運ぶ");
    assert!(
        store.describes(&event),
        "集約と事実が同じ内容を語っている (Gateway の対の照合)"
    );
}

/// ストアが誰かに書き換えられていたら公開しない。
#[test]
fn a_changed_store_refuses_the_publication() {
    let mut store = store_of(CodekbGeneration::of_tree_hash("current"));
    let refusal = store
        .publish(
            candidate(&["src/"], Some("3c55f6af")),
            &CodekbGeneration::of_token("sha256:stale").unwrap(),
            &source(),
            Some(&source()),
            Some("3c55f6af"),
        )
        .expect_err("世代が違えば拒否");
    assert!(
        matches!(&refusal, CodekbPublishRefusal::StoreChanged { expected, found }
            if expected.as_str() == "sha256:stale" && found.as_str() == "sha256:current"),
        "{refusal:?}"
    );
}

/// 源が動いていたら公開しない。**指紋が計算できないこと自体も「違う」**である。
#[test]
fn a_changed_or_uncomputable_source_refuses_the_publication() {
    let mut store = store_of(CodekbGeneration::absent());
    let refusal = store
        .publish(
            candidate(&["src/"], Some("3c55f6af")),
            &CodekbGeneration::absent(),
            &source(),
            Some(&CodekbSourceFingerprint::of_git("moved")),
            Some("3c55f6af"),
        )
        .expect_err("源が動けば拒否");
    assert!(
        matches!(&refusal, CodekbPublishRefusal::SourceChanged { found: Some(found), .. }
            if found.as_str() == "git:moved"),
        "{refusal:?}"
    );

    let mut store = store_of(CodekbGeneration::absent());
    let refusal = store
        .publish(
            candidate(&["src/"], Some("3c55f6af")),
            &CodekbGeneration::absent(),
            &source(),
            None,
            Some("3c55f6af"),
        )
        .expect_err("計算できなければ拒否");
    assert!(
        matches!(
            &refusal,
            CodekbPublishRefusal::SourceChanged { found: None, .. }
        ),
        "{refusal:?}"
    );
}

/// 候補の鮮度印が現在の源と食い違っていたら公開しない。
#[test]
fn a_stale_candidate_fingerprint_refuses_the_publication() {
    let mut store = store_of(CodekbGeneration::absent());
    let refusal = store
        .publish(
            candidate(&["src/"], Some("1111111111")),
            &CodekbGeneration::absent(),
            &source(),
            Some(&source()),
            Some("3c55f6af"),
        )
        .expect_err("鮮度印が古ければ拒否");
    assert!(
        matches!(&refusal, CodekbPublishRefusal::CandidateStale { staged, current }
            if staged.as_deref() == Some("1111111111") && current.as_deref() == Some("3c55f6af")),
        "{refusal:?}"
    );
}

/// **どちらも記録が無い**のは食い違いではない (upstream の `null === null` と同じ)。
#[test]
fn two_absent_candidate_fingerprints_are_not_a_mismatch() {
    let mut store = store_of(CodekbGeneration::absent());
    assert!(
        store
            .publish(
                candidate(&["src/"], None),
                &CodekbGeneration::absent(),
                &source(),
                Some(&source()),
                None,
            )
            .is_ok(),
        "指紋を持たない候補は、源も計算できないときに限って通る"
    );
}

/// 検査の順序は upstream 逐語 — ストア → 源 → 候補。3 つとも外れていても最初の 1 つを答える。
#[test]
fn the_three_checks_are_answered_in_the_upstream_order() {
    let mut store = store_of(CodekbGeneration::of_tree_hash("current"));
    let refusal = store
        .publish(
            candidate(&["src/"], Some("stale-candidate")),
            &CodekbGeneration::of_token("sha256:stale").unwrap(),
            &CodekbSourceFingerprint::of_token("git:stale").unwrap(),
            Some(&source()),
            Some("3c55f6af"),
        )
        .expect_err("3 つとも外れている");
    assert!(
        matches!(refusal, CodekbPublishRefusal::StoreChanged { .. }),
        "先に見るのはストアの世代である: {refusal:?}"
    );
}

// ---------------------------------------------------------------------------
// 集約 Codekb — 中断した公開の始末 (畳むのは集約の振る舞い)
// ---------------------------------------------------------------------------

/// 中断した公開を抱えていないストアは、畳む操作で**イベントを生まない**。
///
/// 生んでしまうと、畳むものが無い場面でも Gateway が書込を通ることになる (空振りの書込)。
#[test]
fn a_store_without_an_interrupted_publication_settles_into_no_event() {
    let mut store = store_of(CodekbGeneration::of_tree_hash("live"));
    assert!(
        store.settle_interrupted_publication().is_none(),
        "畳むものが無ければ事実は起きない"
    );
}

/// 中断した公開を抱えて再構成したストアは、畳む操作で事実を 1 件生む (1 コマンド 1 イベント)。
#[test]
fn a_store_holding_an_interrupted_publication_settles_it_with_one_event() {
    let mut store =
        Codekb::observed_with_interrupted_publication(repo(), CodekbGeneration::absent());
    let event = store
        .settle_interrupted_publication()
        .expect("抱えていれば畳める");
    let CodekbEvent::InterruptedPublicationSettled(settled) = &event else {
        panic!("畳んだ事実は InterruptedPublicationSettled である");
    };
    assert_eq!(settled.aggregate_id(), &repo());
    assert!(
        store.describes(&event),
        "集約と事実が同じことを語っている (Gateway の対の照合)"
    );
    assert!(
        store.settle_interrupted_publication().is_none(),
        "一度畳んだら二度目の事実は起きない"
    );
}

/// 畳んでいない集約は、畳んだ事実を**語らない** — 対が食い違えば Gateway が拒める。
#[test]
fn a_store_that_has_not_settled_does_not_describe_a_settlement() {
    let mut holder =
        Codekb::observed_with_interrupted_publication(repo(), CodekbGeneration::absent());
    let event = holder
        .settle_interrupted_publication()
        .expect("抱えていれば畳める");
    let untouched = store_of(CodekbGeneration::absent());
    assert!(
        !untouched.describes(&event),
        "畳んでいない集約は畳んだ事実の相方ではない"
    );
}

/// 畳む操作は**世代を名乗り直さない** — 畳んだあとの世代はディスクから観測する値である。
#[test]
fn settling_does_not_invent_a_generation() {
    let mut store =
        Codekb::observed_with_interrupted_publication(repo(), CodekbGeneration::absent());
    drop(store.settle_interrupted_publication());
    assert_eq!(
        store
            .take_snapshot(scope_paths(&["src/"]), source())
            .store_generation()
            .as_str(),
        "none",
        "畳む前に観測した世代のまま — 集約は木を畳み直せない"
    );
}
