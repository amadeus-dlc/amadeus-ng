//! `CodekbRepository` の実 Gateway の契約 — 群 D。
//!
//! 1. `find_by_id` は**ディスクに在るものから世代を観測**する。ストアが無いことも 1 つの世代
//!    (`none`) であり、失敗ではない。中断した公開が残っていれば、それも**観測結果の一部**として
//!    集約に載せて再構成する。
//! 2. `find_by_id` は**読取だけ**である — 副作用を持たない。畳むかどうかを決めるのは集約
//!    ([`Codekb::settle_interrupted_publication`]) で、実際に畳むのはその事実を受けた `store`
//!    である (upstream `handleCodekbSnapshot` / `handleCodekbPublish` が `withCodekbLock` の
//!    下で `recoverCodekbTransactions` を先に走らせるのと同じ順序が、ロック区間の内側で
//!    「再構成 → 判断 → store」として起きる)。
//! 3. `store` は 9 成果物を**全体で 1 つの操作として**置き換える。読み手が 9 枚のうち数枚だけ
//!    新しい状態を見ることはない。
//! 4. `store` は決着の事実を受けて、中断した公開を upstream `recoverCodekbTransactions` と
//!    同じ結果へ畳む。
//! 5. `store` は (イベント, 集約) の対が食い違っていたら拒む。
//!
//! すべて使い捨ての一時ディレクトリで回し、実 codekb ストアには触れない。
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

use core_command_domain::workspace::{
    Codekb, CodekbArtifact, CodekbArtifactName, CodekbArtifacts, CodekbCandidate, CodekbEvent,
    CodekbGeneration, CodekbRepoId, CodekbScopePath, CodekbScopePaths, CodekbSourceFingerprint,
};
use core_command_interface_adapter::orchestration::CodekbRepositoryImpl;
use core_command_use_case::orchestration::{CodekbRepository, RepositoryError};
use core_infrastructure::tree_hash::hash_tree;

fn repo() -> CodekbRepoId {
    CodekbRepoId::parse("demo-repo").expect("フィクスチャの repo 識別子は文法内")
}

fn source() -> CodekbSourceFingerprint {
    CodekbSourceFingerprint::of_git("3c55f6af")
}

/// 9 成果物ちょうどの候補 (中身の風味で世代が変わることを見るために `flavour` を取る)。
fn candidate(flavour: &str) -> CodekbCandidate {
    let artifacts = CodekbArtifacts::of(
        CodekbArtifactName::all()
            .iter()
            .map(|name| {
                CodekbArtifact::new(
                    *name,
                    format!("# {}\n{flavour}\n", name.as_str()).into_bytes(),
                )
            })
            .collect(),
    )
    .expect("9 つちょうど");
    CodekbCandidate::new(
        artifacts,
        CodekbScopePaths::of(vec![CodekbScopePath::parse("src/").unwrap()]),
        Some("3c55f6af".to_string()),
    )
}

/// 再構成した集約に公開させ、(イベント, 集約) の対を得る。
fn publish(codekb: &mut Codekb, flavour: &str) -> CodekbEvent {
    let expected = codekb.take_snapshot(CodekbScopePaths::of(Vec::new()), source());
    let expected_store = expected.store_generation().clone();
    codekb
        .publish(
            candidate(flavour),
            &expected_store,
            &source(),
            Some(&source()),
            Some("3c55f6af"),
        )
        .expect("噛み合えば公開できる")
}

struct Fixture {
    _root: tempfile::TempDir,
    store: std::path::PathBuf,
    transactions: std::path::PathBuf,
}

fn fixture() -> Fixture {
    let root = tempfile::tempdir().expect("一時ディレクトリ");
    let store = root.path().join("aidlc/spaces/default/codekb/demo-repo");
    let transactions = root
        .path()
        .join("aidlc/spaces/default/intents/.aidlc-codekb-transactions/demo-repo");
    Fixture {
        _root: root,
        store,
        transactions,
    }
}

fn subject(fixture: &Fixture) -> CodekbRepositoryImpl {
    CodekbRepositoryImpl::new(&fixture.store, &fixture.transactions)
}

fn write_nine(dir: &Path, flavour: &str) {
    fs::create_dir_all(dir).unwrap();
    for name in CodekbArtifactName::all() {
        fs::write(
            dir.join(name.as_str()),
            format!("# {}\n{flavour}\n", name.as_str()),
        )
        .unwrap();
    }
}

/// ストアが無いのは失敗ではない — 不在という世代を観測する。
#[tokio::test]
async fn an_absent_store_is_reconstructed_as_the_absent_generation() {
    let fixture = fixture();
    let codekb = subject(&fixture).find_by_id(&repo()).await.expect("引ける");
    assert_eq!(
        codekb
            .take_snapshot(CodekbScopePaths::of(Vec::new()), source())
            .store_generation()
            .as_str(),
        "none"
    );
}

/// 在るストアの世代は、その木を畳んだ値である。
#[tokio::test]
async fn a_present_store_is_reconstructed_from_the_tree_it_holds() {
    let fixture = fixture();
    write_nine(&fixture.store, "old");
    let codekb = subject(&fixture).find_by_id(&repo()).await.expect("引ける");
    let expected = hash_tree(&fixture.store, &["./".to_string()], &[]).expect("畳める");
    assert_eq!(
        codekb
            .take_snapshot(CodekbScopePaths::of(Vec::new()), source())
            .store_generation()
            .as_str(),
        format!("sha256:{expected}")
    );
}

/// 公開は 9 成果物ちょうどを置き、痕跡を残さない。
#[tokio::test]
async fn publishing_lays_down_exactly_the_nine_and_leaves_no_transaction_behind() {
    let fixture = fixture();
    let mut repository = subject(&fixture);
    let mut codekb = repository.find_by_id(&repo()).await.expect("引ける");
    let event = publish(&mut codekb, "new");
    repository.store(&event, &codekb).await.expect("公開できる");

    let mut placed: Vec<String> = fs::read_dir(&fixture.store)
        .expect("ストア")
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    placed.sort();
    let expected: Vec<String> = CodekbArtifactName::all()
        .iter()
        .map(|name| name.as_str().to_string())
        .collect();
    assert_eq!(placed, expected, "9 成果物ちょうど");
    assert!(
        !fixture.transactions.exists(),
        "トランザクションの痕跡を残さない"
    );
}

/// 既存のストアは丸ごと置き換わる — 前の版の余分なファイルが残らない。
#[tokio::test]
async fn publishing_over_an_existing_store_replaces_it_wholesale() {
    let fixture = fixture();
    write_nine(&fixture.store, "old");
    fs::write(fixture.store.join("stray.md"), "残ってはいけない\n").unwrap();

    let mut repository = subject(&fixture);
    let mut codekb = repository.find_by_id(&repo()).await.expect("引ける");
    let event = publish(&mut codekb, "new");
    repository.store(&event, &codekb).await.expect("公開できる");

    assert!(
        !fixture.store.join("stray.md").exists(),
        "前の版の余分は消える"
    );
    assert_eq!(
        fs::read_to_string(fixture.store.join("architecture.md")).unwrap(),
        "# architecture.md\nnew\n"
    );
}

/// `find_by_id` は**読取だけ**である — 中断した公開が残る状態を読んでも、ファイルシステムを
/// 1 バイトも変えない。畳むのは集約が決め、`store` が果たす
/// (`coding-rules/command-query-separation.md`)。
#[tokio::test]
async fn find_by_id_reads_without_touching_the_filesystem() {
    let fixture = fixture();
    // 公開の途中で落ちた形: ストアは差し替え済みで不在、前の中身が取り残されている。
    let backup = fixture.transactions.join("1234-abcd/backup");
    write_nine(&backup, "old");
    assert!(!fixture.store.exists());

    let codekb = subject(&fixture).find_by_id(&repo()).await.expect("引ける");

    assert!(!fixture.store.exists(), "読取はストアを作らない");
    assert!(backup.exists(), "読取は取り残しを動かさない");
    assert_eq!(
        fs::read_to_string(backup.join("architecture.md")).unwrap(),
        "# architecture.md\nold\n",
        "取り残しの中身も変わらない"
    );
    assert_eq!(
        codekb
            .take_snapshot(CodekbScopePaths::of(Vec::new()), source())
            .store_generation()
            .as_str(),
        "none",
        "畳む前の真実をそのまま観測する — ストアは不在である"
    );
}

/// 中断した公開が残っていれば、集約は**それを抱えた姿で**再構成される。
#[tokio::test]
async fn an_interrupted_publication_is_part_of_what_find_by_id_observes() {
    let fixture = fixture();
    write_nine(&fixture.transactions.join("1234-abcd/backup"), "old");

    let mut codekb = subject(&fixture).find_by_id(&repo()).await.expect("引ける");

    assert!(
        codekb.settle_interrupted_publication().is_some(),
        "抱えているなら畳む事実が起きる"
    );
}

/// 集約が決めた決着を `store` が受けると、upstream `recoverCodekbTransactions` と同じ結果へ
/// 畳む — ストアが不在なら取り残しが戻り、痕跡は片付く。
#[tokio::test]
async fn storing_the_settlement_folds_the_interrupted_publication() {
    let fixture = fixture();
    // 公開の途中で落ちた形を作る: ストアは差し替え済みで不在、前の中身が取り残されている。
    write_nine(&fixture.transactions.join("1234-abcd/backup"), "old");
    assert!(!fixture.store.exists());

    let mut repository = subject(&fixture);
    let mut codekb = repository.find_by_id(&repo()).await.expect("引ける");
    let settlement = codekb
        .settle_interrupted_publication()
        .expect("抱えているなら畳める");
    repository
        .store(&settlement, &codekb)
        .await
        .expect("畳める");

    assert!(fixture.store.exists(), "取り残しがストアへ戻る");
    assert_eq!(
        fs::read_to_string(fixture.store.join("architecture.md")).unwrap(),
        "# architecture.md\nold\n"
    );
    assert!(!fixture.transactions.exists(), "畳んだ痕跡は片付ける");

    let folded = repository.find_by_id(&repo()).await.expect("引ける");
    let expected = hash_tree(&fixture.store, &["./".to_string()], &[]).expect("畳める");
    assert_eq!(
        folded
            .take_snapshot(CodekbScopePaths::of(Vec::new()), source())
            .store_generation()
            .as_str(),
        format!("sha256:{expected}"),
        "畳んだ**あと**に読めば、戻った木の世代が観測される"
    );
}

/// 畳むべき中断が無ければ、集約は事実を生まない — **書込は 1 回も起きない**。
#[tokio::test]
async fn a_store_without_an_interrupted_publication_yields_no_settlement() {
    let fixture = fixture();
    write_nine(&fixture.store, "live");

    let mut codekb = subject(&fixture).find_by_id(&repo()).await.expect("引ける");

    assert!(
        codekb.settle_interrupted_publication().is_none(),
        "畳むものが無ければ事実は起きない (空振りの書込を作らない)"
    );
    assert_eq!(
        fs::read_to_string(fixture.store.join("architecture.md")).unwrap(),
        "# architecture.md\nlive\n",
        "在るストアには触らない"
    );
    assert!(!fixture.transactions.exists(), "痕跡も生まれない");
}

/// ストアが在るまま中断していたら、取り残しは戻さない (在るほうが新しい)。
#[tokio::test]
async fn a_leftover_beside_a_live_store_is_discarded_not_restored() {
    let fixture = fixture();
    write_nine(&fixture.store, "live");
    write_nine(&fixture.transactions.join("1234-abcd/backup"), "old");

    let mut repository = subject(&fixture);
    let mut codekb = repository.find_by_id(&repo()).await.expect("引ける");
    let settlement = codekb
        .settle_interrupted_publication()
        .expect("取り残しがあれば畳める");
    repository
        .store(&settlement, &codekb)
        .await
        .expect("畳める");

    assert_eq!(
        fs::read_to_string(fixture.store.join("architecture.md")).unwrap(),
        "# architecture.md\nlive\n",
        "在るストアは触らない"
    );
    assert!(!fixture.transactions.exists(), "痕跡は片付ける");
}

/// 対が食い違う書込は拒む (歴史と保存像が別の内容を語らないようにする)。
#[tokio::test]
async fn a_mismatched_event_and_aggregate_pair_is_refused() {
    let fixture = fixture();
    let mut repository = subject(&fixture);
    let mut published = repository.find_by_id(&repo()).await.expect("引ける");
    let event = publish(&mut published, "new");

    // 公開していない別の集約に、公開の事実だけを渡す。
    let untouched = Codekb::observed(repo(), CodekbGeneration::absent());
    let error = repository
        .store(&event, &untouched)
        .await
        .expect_err("対が食い違えば拒む");
    assert!(
        matches!(error, RepositoryError::Corrupt { .. }),
        "{error:?}"
    );
    assert!(!fixture.store.exists(), "拒んだら何も置かない");
}

/// 決着の事実も同じ照合を受ける — 畳んでいない集約と対にしたら拒み、取り残しに触らない。
#[tokio::test]
async fn a_settlement_paired_with_an_unsettled_aggregate_is_refused() {
    let fixture = fixture();
    let backup = fixture.transactions.join("1234-abcd/backup");
    write_nine(&backup, "old");

    let mut repository = subject(&fixture);
    let mut holder = repository.find_by_id(&repo()).await.expect("引ける");
    let settlement = holder
        .settle_interrupted_publication()
        .expect("抱えているなら畳める");

    let untouched = Codekb::observed(repo(), CodekbGeneration::absent());
    let error = repository
        .store(&settlement, &untouched)
        .await
        .expect_err("対が食い違えば拒む");
    assert!(
        matches!(error, RepositoryError::Corrupt { .. }),
        "{error:?}"
    );
    assert!(backup.exists(), "拒んだら取り残しにも触らない");
    assert!(!fixture.store.exists(), "拒んだら何も戻さない");
}
