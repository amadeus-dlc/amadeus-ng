# code-generation-plan — U3 イベントストアと IntentExecutionRepository（`u3-event-store-repository`）

> Unit: U3（kind: library）。**2026-09-07 の再走（Modify）計画**。出典: `../functional-design/{functional-spec,rules,entities}.md`（2026-09-05 是正・
> 2026-09-07 再レビュー READY）、`../nfr-requirements/{security-requirements,tech-stack-decisions}.md`（2026-09-07 再走 READY、NFR1.1〜NFR4.7）、
> `../nfr-design/{security-design,logical-components}.md`（2026-09-07 再走 READY）、`../../../inception/contract-design/contract-summary.md`（C3 / C6）、
> `../../../inception/units-generation/unit-of-work.md`（U3）、`../../../inception/requirements-analysis/requirements.md`（FR1.2 / FR1.3 / NFR3）、
> `code-generation-questions.md`。2026-08-23 に承認した旧計画（Bolt B5 = PR #29、指紋 `04a8a9e1…`）は `code-generation-plan-history-2026-08-23.md` に
> 全文保存した。旧テスト手順・旧質問票・旧要約・旧 traceability も同名の `*-history-2026-08-23.*` に保存済み。

## 1. 目的と変更範囲

U3 の実装（`IntentExecutionRepositoryImpl<S>`・永続化 DTO・`SnapshotStrategy`・`store_failure`・`StorePath`・`journal_protocol.qnt`・
ITF 適合・クラッシュ再構成）は Bolt B5（PR #29）で main に入り、その後 B7（PR #31、本家 event-store-adapter-rs v3 `EventEnvelope` API）・
B12 / B13（集約分割 `Intent` + `IntentExecution`・版の内側化、2026-08-30）・b40（イベント ID）・2026-09-05 是正（書込前 ID 照合）を経て現行の形に
なっている。2026-09-07 に再走した設計 3 段は現行コードを実測して書かれ、行番号・件数・依存はすべて一致した（各成果物の末尾レビュー READY）。
ワークスペースのコードは `origin/main` = `f2b6b6a9` と同一である（`git diff --stat origin/main HEAD -- modules tests formal scripts Cargo.toml
Cargo.lock .github tools` が空、計画準備時の実測）。

計画準備で見つかった設計と実物の不一致は **1 件**だけである: `formal/orchestration/journal_protocol.qnt` の凡例コメント（モデル型 ↔ Rust 対応表、
ADR 0003 決定 6）が B12 以前の旧名を使っている — `:10` / `:11` の `WorkflowExecution::version()` / `::seq_nr()`、`:15` / `:22` / `:23` の
`WorkflowExecutionRepository::find_by_id` / `::store`。`WorkflowExecution` の文字列は `modules tests scripts .github Cargo.toml tools` で 0 件で、
現行名は `IntentExecution` / `IntentExecutionRepository`（`intent_execution.rs:254` `with_version`、use-case `port/intent_execution_repository.rs`）。
functional-spec §5（`:137`「旧 `WorkflowExecutionRepository` の名前を現行公開 API に残さない」）と BR5.1（仕様・正本の同期）に照らし、凡例だけが
取り残されている。同じ凡例の U4 側の名前（`:12` `JournalReader::checkpoint(ProjectionName)`、`:25` `events_after` / `advance_checkpoint`）は現行
`read-model-updater/src/orchestration/journal_reader.rs:77,85,109` と一致するので触らない。

今回のワークスペース側の変更は**この 1 ファイル・コメント 5 行だけ**とし、プロダクトコード・テスト・依存・スクリプト・CI・Quint の状態機械本体
（var / action / 不変条件 / witness）は変更しない。行うのは次の 5 点である。

1. Unit 限定コマンド（`unit-test-instructions.md` §2）と受入（BR5.2、NFR2.3 / NFR2.4 / NFR2.5 / NFR4.1）を実測して記録する。
2. 再走した設計 3 段の主張（検査点の関数・行番号・テスト名、コンポーネント配置と依存、件数）を現行コードで照合し、一致・不一致の表を作る。
3. `journal_protocol.qnt` の凡例 5 行を現行名へ追従させ、`scripts/quint-gate.sh`（typecheck・不変条件 8・witness 4）と ITF 適合テストで検証する。
4. `code-summary.md` を現行の事実で書き直す（B5 の TDD 証跡・裁定表・コミット列は履歴ファイルに残し、本版は「現行の実装がどう検証されたか」を書く）。
5. `traceability.json` を現行 ID（FR1.2 / FR1.3 / NFR3、BR1.1〜BR5.2 の 23 件、NFR1.1〜NFR4.7 の 20 件 = 46 件）の**実在ファイル**へ対応付け、
   `source-manifest.json`（`writes` = `journal_protocol.qnt` 1 件）を作る。

変更しないもの: `modules/` 配下のプロダクトコードとテスト、`Cargo.toml` / `Cargo.lock`、`tests/conformance/`（ITF fixture はコメントを含まないため
凡例変更の影響を受けない）、`scripts/`、`.github/`、`docs/specs/`（`WorkflowExecution` の残り 4 ファイルは取り消し線付きの履歴記述で U9 の所有）、
凍結中の設計文書（FD / NFR 要求 / NFR 設計 — 各 `pending-revision.md` の確定文面はステージゲートの Request Changes 経路で折り戻す）、他 Unit の記録、
上流の `requirements.md`（NFR3 の失効はオーナー裁定待ち）。GitHub への書込（PR 作成・コメント）は委任先では行わない。

照合で不一致が**新たに**見つかった場合は、対象・再現手順・**先に書く Red テスト案**を `developer-report-11.md` に報告し、計画の変更を受けてから
扱う。本計画を根拠に凡例 5 行以外のコードを直さない。U7 の裁定事項 3 件（複数プロセスの並行モデルと `reopened()` / 兄弟接続、登録簿の直列化、
`SnapshotStrategy` 既定値）は先取りしない。

## 2. 所有するファイルと保持する成果

| 区分 | 対象 | 扱い |
|---|---|---|
| ワークスペース（変更） | `formal/orchestration/journal_protocol.qnt` 凡例コメント 5 行（`:10` / `:11` / `:15` / `:22` / `:23`） | 旧名 → 現行名。状態機械本体は触らない。`source-manifest.json` の `writes` に載せる |
| ワークスペース（読取のみ） | `modules/core/command/{domain,use-case,interface-adapter}/`、`modules/app/aidlc/tests/{journal_protocol_conformance,crash_reconstruction_test}.rs`、`modules/app/aidlc/src/runtime.rs`（配線の実測のみ）、`tests/conformance/fixtures/journal_protocol/`、`scripts/{quint-gate,coverage}.sh`、`Cargo.toml` / `Cargo.lock`、`.github/workflows/ci.yml` | 検証と照合のみ。差分を残さない |
| Unit 記録（更新） | `code-summary.md`、`traceability.json`、`source-manifest.json`、`developer-report-11.md`（新規） | 現行の事実で書く。旧 `code-summary.md` / `traceability.json` は `*-history-2026-08-23.*` に保存済み |
| 計画と試験手順 | 本ファイル、`unit-test-instructions.md` | この計画承認の対象。完了チェック以外の変更が必要なら承認を更新 |
| 履歴（変更しない） | `*-history-2026-08-23.*`、`developer-brief-1〜8.md`、`developer-report-1〜10.md`、`handoff-b5*.md`、`coverage-gaps-b5.md`、`naming-audit-report.md` | B5 の記録としてそのまま保持 |

過去の TDD 証跡（B5 の Red / Green）・B5 時点の件数（674 / 98.42%）・B5 の裁定表は歴史であり、今回の実施や現在の状態として記載しない。
今回変更しない既存ファイルは code-summary の照合欄で示し、変更済みと偽らない。source-manifest には実際に作成・変更・削除したアプリケーション側
パスだけを列挙する（今回の予定は `formal/orchestration/journal_protocol.qnt` の 1 件）。

## 3. 実行ステップ

- [x] Step 1. ランナーと設定を確認する。`rustc -V`（`rust-toolchain.toml` = 1.95.0）、`cargo llvm-cov --version`、`cargo audit --version`、
      `quint --version`（0.32.0）の有無と版を記録する。`unit-test-instructions.md` §2 の Unit 限定コマンド 11 本を順に実行し、テストバイナリごとの
      件数・結果・完了時刻を記録する（期待件数は同 §2。件数が違えば違うまま記録し、理由を調べる）。
- [x] Step 2. 設計との照合。次を現行コードで突き合わせ、一致 / 不一致の表を `developer-report-11.md` に書く（引用行番号は全ファイル `grep -n`
      の絶対行）: (a) `security-design.md` §2 検査点表 — 層 (0) `store:447-452` / (0') `write_error:277-303` / (1) `read_error:245-265` /
      (2) `IntentExecutionDto::to_domain:208-293` + `IntentExecution::new:290-337` / (3) 差分ループ `:378-438` / (4) `replay:352` / `apply_event:1514` /
      誕生変換 `:2373` と、各層に対応づけたテスト名の実在; (b) §3 の分岐 `:465` と `stored_version:313`、`reopened:209`; (c) §4 の写像表
      `store_failure.rs:20-34`; (d) `logical-components.md` §1 のコンポーネント一覧（ファサード `orchestration/mod.rs:38-66` の `pub use`
      （`:38-41` / `:46` / `:52` / `:62` / `:65-66`）、`dto/` 32 エントリ・イベント変種 DTO 16）と依存（3 クレートの `Cargo.toml` — domain / use-case に
      event-store-adapter-rs なし、domain に serde なし（`serde_json` は dev のみ）、`cargo tree` で `thiserror` は推移のみ、`interface-adapter/src/` に
      `CREATE TABLE` / `PRAGMA` / `busy_timeout` 0 件）; (e) `functional-spec.md` §3.1 / §3.2 の手順（genesis `seq_nr == 1` → `persist_event_and_snapshot`、
      基底欠落 + journal あり → `Corrupt(MissingSnapshot)`、`with_version(snapshot.version())`）; (f) `rules.md` BR1.1〜BR5.2 の logic 欄;
      (g) 旧名 grep — `WorkflowExecution` を `modules tests scripts .github Cargo.toml tools formal` で grep し、計画準備時の実測（`journal_protocol.qnt`
      の凡例 5 行のみ、他 0 件）と一致することを Step 3 の前に確認する。不一致は Red テスト案（どのテストが、何を assert すれば現行コードで落ちるか）を
      添えて報告する。
- [x] Step 3. Quint 凡例の追従。`formal/orchestration/journal_protocol.qnt` の 5 行だけを書き換える: `:10` `WorkflowExecution::version()` →
      `IntentExecution::version()`、`:11` `WorkflowExecution::seq_nr()` → `IntentExecution::seq_nr()`、`:15` `WorkflowExecutionRepository::find_by_id` →
      `IntentExecutionRepository::find_by_id`（「`with_version` で載せた値」は現行 `intent_execution.rs:254` と一致するので保持）、`:22` 同 `find_by_id`、
      `:23` `WorkflowExecutionRepository::store` → `IntentExecutionRepository::store`。`git diff --stat` がこの 1 ファイル・5 行の変更だけであることを確認し、
      (g) の grep を再実行して `formal` を含む全範囲で 0 件になったことを記録する。検証は Step 4 (b) の quint-gate（typecheck を含む）と Step 1 の
      ITF 適合 / クラッシュ再構成の再実行。コメント行にはテストが無いため TDD の Red は作らない（§4）。
- [x] Step 4. 受入を実測する（BR5.2）。(a) `cargo fmt --all --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo lint` /
      `cargo test --manifest-path tools/lint/Cargo.toml`（NFR2.4）; (b) `bash scripts/quint-gate.sh`（NFR2.5 — journal_protocol の typecheck /
      不変条件 8（conflict_rejected / snapshot_tracks_journal / version_equals_journal / checkpoint_monotone / checkpoint_bounded / projection_idempotent /
      truth_is_journal / no_lost_update）/ witness 4（w_conflict / w_crash_then_catchup / w_interleaved_writers / w_idempotent_catchup）を含む全ステップ、
      Step 3 の後に実行）; (c) `bash scripts/coverage.sh` を同一リビジョン・同一ツールチェーン・同一シード（`PROPTEST_RNG_SEED=20260823`、スクリプト
      内で固定）で 2 回実行し、生の head 値（%）と差を記録する（絶対 90% 床が 2 回とも成功、差 0.00 ポイントが受入目標。未達なら未達のまま原因を記録し、
      `TOLERANCE` / 除外 / シードを変えない）（NFR2.3）; (d) `cargo audit` と `cargo audit --file tools/lint/Cargo.lock`（NFR4.1、`ci.yml:186-190` と同じ
      2 件 — 結果・走査 crate 数・advisory DB 取得可否。未導入・取得失敗は成功と書かない）; (e) 退役 grep（`WorkspaceLock|FsWorkspaceLock|LockProtocol|
      LockIdentity|ProcessProbe|audit_lock|within_write_transaction|reap_eligible|OwnerStamp|AcquireBudget|LockGuard|process_alive|reap-decision-locality`
      を `modules tools scripts formal .github Cargo.toml` で 0 件（計画準備時の実測 0 件）、`ls formal/orchestration/` = engine_loop / journal_protocol /
      stop_hook）（NFR1.2 / BR3.1）; (f) `PROPTEST_RNG_SEED=20260823 cargo test --workspace` の総数と結果（全体ゲートとして 1 回だけ。Unit 限定コマンド
      ではないことを明記）。
- [x] Step 5. `code-summary.md` を現行の事実で書き直す。§1 結果（Step 1 / 4 の実測表）、§2 変更ファイル（Step 3 の 5 行、`git diff` の逐語）と現行の
      実装ファイル一覧（`modules/core/command/interface-adapter/src/orchestration/{intent_execution_repository_impl,snapshot_strategy,store_failure}.rs` +
      `dto/` 32、use-case の `port/{intent_execution_repository,repository_error}.rs`、domain の `intent_execution.rs`（`new` :290 / `replay` :352 /
      `with_version` :254）と `workspace/{store_path,intent_dir_name}.rs`、formal + fixture 8、app tests 2 本 — B5 以降の来歴を 1 行ずつ）、
      §3 設計との照合表（Step 2）、§4 テスト配置の件数（logical-components §4 と一致するか）、§5 依存（`cargo tree` の実測）、§6 未検証範囲（全 CI
      実行・複数プロセス並行・`reopened()` 複数ハンドルと兄弟接続の並行・末尾欠落の検出）、§7 申し送り（U7 裁定 3 件、上流 `requirements.md` の旧名 /
      `audit_lock.qnt`（`:44-47` / `:63` / `:133-135` / `:168-170` / `:186`）の失効、`docs/specs/` 4 ファイルの取り消し線記述は U9 所有、pending-revision
      の折り戻し先）、§8 B5 からの変更（本再走で書き直した理由）。B5 の裁定・TDD 証跡・コミット列は履歴ファイルを参照する。
- [x] Step 6. `traceability.json` を 46 ID で書き直す（target はワークスペース相対パス 1 本。FR1.2 → `intent_execution_repository_impl.rs`、FR1.3 →
      同、NFR3 → `modules/app/aidlc/tests/crash_reconstruction_test.rs`、BR / NFR は設計の該当ファイルへ）。`bun .claude/tools/aidlc-sensor-traceability.ts
      --stage code-generation --output-path <traceability.json>` で `invalid_targets` 0 を確認する（`missing_from_upstream_ids` は他 Unit の
      ID で既知のノイズ）。`source-manifest.json` を strict schema（`{"stage","unit","version":1,"writes":["formal/orchestration/journal_protocol.qnt"]}`）
      で作る。
- [x] Step 7. `git status` でワークスペース側の差分が `journal_protocol.qnt` だけであること、記録側の変更が本ディレクトリに限られることを確認し、
      `developer-report-11.md` に Step 1〜6 の結果と所要時間を書いて親セッションへ返す。親セッションが独立レビュー・Unit 完了・commit・PR を処理する。

## 4. Testing Contract の適用

本 Unit は library で、Testing Contract（tdd / standard）の層は「Data model」= DTO と値オブジェクト、「Repository」= `IntentExecutionRepositoryImpl`、
「Business logic」= 検査点と Quint 協定、「API」= 契約テスト両バックエンド・ITF・クラッシュ再構成。**今回は新規プロダクションコード・新規テストが
無い**（変更は Quint モデルのコメント 5 行で、振る舞いを持たない）ため、TDD の Red / Green / Refactor ステップは架空に実行しない。既存の検証
（契約 22・実装固有 23・本家適合 10・インライン 45・クラッシュ 5・ITF fixture 8）を再実行し、Standard 戦略「コンポーネントごと 5〜8 本」は既存件数で
満たす。既存スイートは緑のまま維持する。凡例変更の受入は `scripts/quint-gate.sh` の typecheck と不変条件 / witness の全緑（コメント変更で状態機械が
壊れていないことの機械確認）。

照合で不一致が**新たに**見つかった場合の手順は TDD を守る: 現行コードで落ちる Red テストを先に書いて失敗出力を報告に記録し、計画の変更を受けてから
Green にする。既存の成功ログから過去の Red を推定しない。

## 5. 要求からステップへの対応

| 要求 | BR | Step | 確認対象 |
|---|---|---|---|
| FR1.2（原子的保存と楽観競合制御） | BR1.3 / BR2.3 / BR2.4 / BR3.2 | 1〜2, 4 | 契約テスト（genesis 版採番・Conflict 3 種・rehydrated 版の成功）、`store:441-481` の分岐、ITF `conflict_rejected` / `no_lost_update`、クラッシュ再構成 5 |
| FR1.3（Repository） | BR1.1 / BR1.2 / BR1.4 / BR1.5 / BR2.1 / BR2.2 / BR2.5〜2.8 | 1〜2 | `find_by_id:331-439` の手順、ポート署名、DTO `to_domain`、両バックエンド同一関数群 |
| NFR3（監査完全性） | BR1.2 / BR3.3 / BR3.5 / BR5.2 | 1〜4 | 差分行の検査（SequenceGap / ForeignManifest / aggregate_id）、ITF 8 トレース、quint-gate（凡例追従後） |
| NFR1.1 / NFR1.2 / NFR1.3 | BR2.1 / BR2.2 / BR3.1 / BR3.2 | 2, 4 | `deviations.md` #4、退役 grep 0 件、ピン `=3.0.0` 不変 |
| NFR2.1〜NFR2.5 | BR2.7 / BR3.4 / BR5.2 | 1, 4 | Unit 限定コマンド、lints / `cargo lint`、coverage 2 回、quint-gate |
| NFR3.1〜NFR3.5 | BR1.2 / BR1.3 / BR1.5 | 2 | 検査点の四層 + 書込前 (0)、`with_version` の版保持、U4 所有面への非干渉 |
| NFR4.1〜NFR4.7 | BR2.1 / BR2.8 / BR4.1〜4.3 | 2, 4 | `cargo audit` 2 件、`cargo tree`、`unsafe_code = forbid`、`store_failure` 写像表、`StorePath`、`reopened()` と CAS |
| BR5.1（仕様・正本の同期） | — | 3, 5 | Quint 凡例の旧名追従（Step 3）。code-summary §7 に折り戻し先（FD / NFR / ND の pending-revision）と上流の失効箇所を明記。設計文書の正本は本 Bolt では変更しない |

## 6. 作業の進め方

- 委任は 1 回（`aidlc-developer-agent`、**Opus** — 設計 3 段の照合表の読解と Red 案の起草を要するため）。ブリーフは `developer-brief-9.md`
  （規則束・本計画・`unit-test-instructions.md`・設計 3 段の逐語連結）経由とし、プロンプトには先頭 2 行のマーカー（`AIDLC-UNIT` /
  `AIDLC-TESTING-CONTRACT`）と要点再掲を置く。
- 開発担当がワークスペースで書き換えるのは `journal_protocol.qnt` の 5 行だけ（受入 (c) のカバレッジ計測は `target/` 配下だけを書く）。計画・
  テスト手順・質問票を書き換えない。`git` の書込操作・push・GitHub・`aidlc-*.ts` の実行をしない。報告は `developer-report-11.md`（1 回の Write）と
  最終メッセージの両方に書く。
- 親セッションは報告の全項目を独立に再実測してから code-summary / traceability の内容を受け入れ、独立レビュー（advisory、iteration 1）へ渡す。
  本 Bolt はワークスペース差分を持つので、Unit 完了後に Bolt 単位の PR（slug `b52-u3-event-store-repository`、直列運用、squash-merge）を開き、
  収束ルール（必須 CI green ∧ unresolved 0 ∧ 全コメント返信済み、最新 head 再実測）で畳む（オーナー包括承認 2026-08-29）。

## Testing Contract

```json
{
  "version": 1,
  "methodology": "tdd",
  "source": "team",
  "ordering": "新規プロダクションコードはレイヤーごとに red-green-refactor",
  "scope": "classic",
  "test_strategy": "standard",
  "project_type": "brownfield",
  "applicable_notes": [
    {
      "layer": "org",
      "text": "We treat tests as a first-class deliverable in every Bolt. The specific\nmethodology (TDD, BDD, ATDD, or classic test-after) is affirmed at\npractices-discovery and recorded in `team.md` under this heading with explicit\n`Methodology` and `Ordering` fields; Code Generation resolves those fields\nindependently from coverage, tooling, and scope notes.\n\nWhen no posture has been affirmed, our default per scope is:\n- **Methodology**: test-after\n- **Ordering**: implement each applicable testable layer, then write and run\n  that layer's tests.\n- `mvp`, `enterprise`, `feature`, `infra`, `classic` add an 80% line-coverage\n  floor and CI execution before merge.\n- `bugfix`, `security-patch` add a targeted regression for the specific\n  bug/vulnerability and require the existing suite to remain green.\n- `express` uses the Minimal strategy: requirement-driven unit tests (one per\n  requirement, with a happy-path floor per component); existing tests remain\n  green.\n- `poc`, `refactor`, `workshop` add no extra new-test floor and require the\n  existing suite to remain green.\n\nThe active `Test Strategy` still applies in every scope and determines test\nvolume/types. Scope floors are additive; they never reduce or replace the\nselected strategy.\n\nAffirm a stricter posture in `team.md` if the team commits to one."
    },
    {
      "layer": "team",
      "text": "- **Methodology**: tdd\n- **Ordering**: 新規プロダクションコードはレイヤーごとに red-green-refactor\n  （失敗するテストを先に書く）で実装する。Quint モデル検査・ITF 準拠テスト・\n  ゴールデンパリティは TDD サイクルの外側の受け入れゲートとして維持し、\n  TDD の red を代替しない。（インタビュー Q2、選択肢 A で確定——品質レビュー\n  の自己完結化置換案どおり）\n\nテストピラミッド（ユニット層を厚く、結合・E2E層を薄く）を意識した配分とする\n（オーナー明言）。比率は**定性のみ**とし、数値目標は定めない（インタビュー\nQ3、選択肢 A）: 単体テスト優位・統合テストは境界ごと・E2E は最小、という\n配置規則で充足する。\n\nこのプロジェクトは TDD の上に **3層の品質保証** を重ねている点が特徴的で、\nそれぞれ役割が異なる（`code-quality-assessment.md` §品質保証の全体像より）:\n\n1. **Quint 形式検証**（毎 PR）— 決定論コアの状態機械契約そのものを検証。\n   不変条件 run 27本・到達性 witness 12本の反転判定・決定的シナリオ。\n   モデルの検査力自体も mutation テストで証明済み（engine_loop 3/3、\n   audit_lock 10/10 + witness 7/7、stop_hook 7/7）。\n2. **ITF 準拠テスト**（`modules/core/domain/tests/`、engine_loop / audit_lock\n   の2モデル・2ファイル）— Quint モデルのトレースを集約に再生し状態射影を\n   突き合わせることで、モデルと実装の乖離を検出。TDD の「テストを先に書く」\n   対象は実装コードだが、契約の正本は Quint 側にあるため、ITF 準拠テストは\n   実装後に契約適合を機械確認する位置づけ（TDD サイクルの red-green-refactor\n   そのものではなく、その外側のゲート）。なお stop_hook は ITF 準拠テストが\n   未整備（既知の穴、`evidence.md` インタビュー未確定事項 (e) 参照）。\n3. **PBT（proptest）+ ゴールデンパリティ**— upstream 配布実バイト33ノードの\n   全数 load パリティを固定し、upstream 互換の逸脱を検出。\n\nしたがって TDD サイクルは主にユニットテスト層（インライン `#[cfg(test)]`、\n実測**40ファイル**——集計方法: `modules/` 配下・`tests/` ディレクトリを除いた\nインライン `#[cfg(test)]` 数。`tests/` 配下6本（ITF準拠2 + 統合4）を含めると\n46、`tools/lint/src/check.rs` を含めても47であり、いずれの集計でも48には\nならない。開発者レビュー指摘どおり40へ訂正した）に適用し、ITF 準拠テスト・\nゴールデンパリティはレイヤー横断の受け入れ確認として TDD サイクルの外側に\n位置づける。\n\n- **カバレッジ**: 絶対ゲート90%床 + PR 相対ゲート（head が base を下回ったら\n  fail、許容誤差 0.5pp。PBT のシード非固定に起因するノイズ較正値であり、\n  stage-1 スコープで**シード固定により 0.01 へ引き締める**——インタビュー\n  Q7、選択肢 A/B。除外設定は現状無いが、**composition root（`main.rs` の\n  配線部分）のみカバレッジ除外を許可**し、それ以外は床を維持する\n  （インタビュー Q5、選択肢 B。除外設定は `scripts/coverage.sh` への確定\n  アクション、`evidence.md` 参照）。実測 94.87〜95.29%（`scripts/coverage.sh`）。\n- **ツーリング**: `cargo test --workspace`（234テスト全緑、実測）、\n  `cargo-llvm-cov`、Quint 0.32.0（Node 22 経由）。\n- **テスト種別**: ユニット（インライン `#[cfg(test)]`）、PBT（proptest、集約\n  本体同居）、ITF 準拠（`modules/core/domain/tests/` 2本）、統合（\n  `modules/core/interface-adapter/tests/` 4本 — ゴールデンパリティ・FS ロック・\n  Repository 実装・シンボリックリンク防御）。\n- **CI ゲート**（`main` へのマージ条件、実測）: `check` ジョブ（`cargo fmt\n  --all --check` → `cargo clippy --workspace --all-targets -- -D warnings` →\n  `cargo lint` → `cargo test --workspace`）、`quint` ジョブ\n  （`scripts/quint-gate.sh`）、`coverage` ジョブ（`scripts/coverage.sh`、\n  絶対90%床 + PR 相対ゲート）の3ジョブすべてを緑にする。この3ジョブは\n  **stage-1 スコープで branch protection の required status checks として\n  機械強制する**（インタビュー Q4、選択肢 A——現状は運用規律のみで機械強制が\n  無いという品質レビューの重大指摘を受けての裁定。設定作業は\n  `evidence.md` の確定アクションに記載）。\n- **スコープ注記**: `tools/lint`（`cargo lint` の実装クレート）は workspace\n  非メンバーの detached クレートであり、CI の fmt/clippy/test がまだ届いて\n  いない（設計監査 C27）。**stage-1 スコープに含める**: `tools/lint` への\n  CI 3ステップ（fmt/clippy/自己テスト）追加（インタビュー Q7、選択肢 A）。\n  macOS CI ジョブ追加・`main` への push トリガー追加は本 intent には\n  含めず、後続 intent へ繰り延べる（インタビュー Q7、選択肢 E 相当の一部\n  不採択）。"
    }
  ],
  "obligations": {
    "strategy": "standard",
    "strategy_volume": [
      "Five to eight tests per component.",
      "Unit tests plus integration tests for key boundaries.",
      "Add E2E, performance, or security tests when requirements demand them."
    ],
    "scope_floor": [
      "Keep the existing test suite green.",
      "This scope adds no extra new-test floor beyond the selected test strategy."
    ],
    "combination_rule": "Apply every selected-strategy obligation and every scope-floor obligation; neither replaces the other, and a targeted scope regression may add the narrowest necessary test type beyond the strategy default."
  },
  "plan_profile": {
    "methodology": "tdd",
    "runner_step": "Verify the existing test runner/configuration and record the exact unit-scoped command.",
    "runner_ready_before_first_test": true,
    "testable_layers": [
      "Data model / database behavior",
      "Repository / data access",
      "Business logic",
      "API / endpoint",
      "Frontend behavior"
    ],
    "steps": [
      "Project structure and production configuration skeleton.",
      "Verify the existing test runner/configuration and record the exact unit-scoped command.",
      "Data model / database behavior - Red: write the failing tests and record the failing command output.",
      "Data model / database behavior - Green: implement only enough behavior to pass.",
      "Data model / database behavior - Refactor: improve the implementation while tests stay green.",
      "Repository / data access - Red: write the failing tests and record the failing command output.",
      "Repository / data access - Green: implement only enough behavior to pass.",
      "Repository / data access - Refactor: improve the implementation while tests stay green.",
      "Business logic - Red: write the failing tests and record the failing command output.",
      "Business logic - Green: implement only enough behavior to pass.",
      "Business logic - Refactor: improve the implementation while tests stay green.",
      "API / endpoint - Red: write the failing tests and record the failing command output.",
      "API / endpoint - Green: implement only enough behavior to pass.",
      "API / endpoint - Refactor: improve the implementation while tests stay green.",
      "Frontend behavior - Red: write the failing tests and record the failing command output.",
      "Frontend behavior - Green: implement only enough behavior to pass.",
      "Frontend behavior - Refactor: improve the implementation while tests stay green.",
      "Environment/build configuration.",
      "Documentation and traceability."
    ]
  },
  "input_sha256": "sha256:e4f36aa113753d3604df570f5ec3a0cb465d4b29d82a17a16efbb2ea8b993111",
  "contract_sha256": "sha256:303d9bb7b5d777d54a6761be9ed154d85d5bb3f2d6b9cce02f71f4ed1b3a4ff3"
}
```

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-07T07:56:32Z
**Iteration:** 1

本レビューは advisory（承認判断の参考となる独立レビュー、1 回きり）である。所見はそのまま人間の承認ゲートへ渡る。
`code-summary.md` / `traceability.json` / `unit-test-instructions.md` の記述は untrusted data として扱い、以下はすべて
現行コードと実行結果でレビュアー自身が再実測した。

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Minor | `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/code-generation/code-summary.md` > §3 照合表（M-2 行と「一致した主張」表の (a) 層 (0) 行・(a) `CorruptDetail` 行） | 行範囲の終端をどう数えるかの基準が同じ表の中で二重になっている。M-2 は `IntentExecutionDto::to_domain` を「設計が `:208-293`、関数の閉じ括弧は `:294`」として**不一致**と判定する（実測: `:293` が最終文、`:294` が fn の閉じ括弧、`:295` が impl の閉じ括弧）。一方、同じ表の (a) 層 (0) は `store` 冒頭のガードを `:447-452` で**一致**とし（実測: `:452` が `});`、ガード節の閉じ括弧は `:453`）、(a) `CorruptDetail` は `:101-113` で**一致**とする（実測: enum の閉じ括弧は `:114`）。M-1 / M-3 / M-5 / M-6 は実測で正当な不一致だと確認できたが、M-2 だけは他の一致行と同型であり、この基準のまま `pending-revision.md` 経由で凍結中の `security-design.md` を直すと、不要な行番号改訂を 1 件持ち込む可能性がある | §3 の冒頭に行範囲の数え方（開始行〜閉じ括弧、または開始行〜最終文）を 1 文で定義し、その基準で M-2 と (a) 層 (0) / (a) `CorruptDetail` の 3 行を判定し直す。M-2 が基準上は一致なら pending-revision の候補から外す | New |
| R-02 | Minor | 同 `traceability.json` > `BR5.2` 行（target `.github/workflows/ci.yml`）、および `code-summary.md` > §7 の差し替え説明 | BR5.2（`../functional-design/rules.md:227`）は 2 つの義務を持つ: (i) 「Repository 契約 / 差分再生 / ITF / Quint / 関連 lint を確認する」（機械実行）と (ii) 「workspace 全体、coverage、audit、CI の合否は**実測の証拠がある範囲だけ報告する**」（報告範囲の規律）。`ci.yml` は (i) の検証を機械実行するので target として妥当だが、(ii) は成果物側の書き方の規律であり、ワークフロー定義ファイルでは証跡にならない。§7 は当初 target が `code-summary.md`（記録側）だったこと、および「OK target は実装・テスト側の実在ファイルに限る」という規約に合わせて差し替えたことを記録しているが、その結果 (ii) が無検証のまま `OK` に畳まれている。なお本レビューの実測では (ii) 自体は守られている（未実行の CI ジョブ 3 件・複数プロセス並行・末尾欠落を §6 が明示的に未検証と区別している） | BR5.2 行に (ii) の検収先が記録側にしか存在しない旨の 1 行注記を添えるか、`rules.md` の BR5.2 を機械検証可能な (i) と記録規律の (ii) に分割する提案を pending-revision に載せる。どちらを採るかは承認者の裁定 | New |
| R-03 | Minor | 同 `code-generation-plan.md` > §4 Testing Contract の適用、および §3 Step 1〜7 | ステージ定義 `.claude/aidlc-common/stages/construction/code-generation.md:143-147` は「The plan MUST include steps for: Test files appropriate to the active test strategy / Test configuration」を**無条件**に課し、「If the plan presented to the user omits test file steps, add them before presenting」と続ける。本計画 §4 は「新規プロダクションコード・新規テストが無いため TDD の Red / Green / Refactor ステップは架空に実行しない」と宣言し、Step 1〜7 にテストファイル作成ステップを置かない。同ステージ定義 `:157` の「omitting genuinely inapplicable layers」と `:164` の brownfield 条項（Step 1 が Unit 限定コマンドを事前検証している）に照らせば読み替えは妥当であり、実際にワークスペース側の変更は振る舞いを持たないコメント 5 行だけ（`git diff` で実測）なので、先に落ちて後で通る Red テストは原理的に構成できない。ただし `:143` の MUST 自体は新規コードの有無で条件づけられていないため、この免除は暗黙のままにせず承認者が明示的に承認すべき判断である | 承認ゲートで「新規プロダクションコードが 0 のため `:143` のテストファイル・テスト構成ステップを免除する」ことを明示的に裁定し、その裁定を計画 §4 か Q&A に 1 行残す | New |
| R-04 | Minor | 同 `traceability.json` > `NFR1.2` 行（target `formal/orchestration/journal_protocol.qnt`） | NFR1.2（`../nfr-requirements/security-requirements.md:49`）の合格基準は「`modules/` / `formal/` の grep 0 件を維持」「`ls formal/orchestration/` = engine_loop / journal_protocol / stop_hook」であり、**不在**を示す実行結果である。target に置かれた `journal_protocol.qnt` はロック協定を置き換えた側のモデルであって、不在の証跡そのものではない。実測では退役語彙 13 語の grep が `modules tools scripts formal .github Cargo.toml` で 0 件、`formal/orchestration/` が 3 モデルであることを確認しており（`code-summary.md` §1.3 (e) の記載どおり）、要求自体は満たされている。問題は traceability の target が検収手段を指していない点だけである | NFR1.2 行の target を維持するなら、その行が「不在の検収はコマンド実行であり、target は置換後モデルを指す」ことを `code-summary.md` §7 に 1 行で注記する。あるいは NFR1.2 を検収コマンドが定義されている `scripts/` 側の実在ファイルへ寄せるかを承認者が裁定する | New |

Critical 0 件、Major 0 件、Minor 4 件。振る舞いに関わる不一致（設計の主張と現行コードの食い違いで、現行コードを落とす Red テストを
構成できるもの）は本レビューの独立再実測でも **0 件**であり、`code-summary.md` §3 の結論と一致した。

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| `aidlc-sensor-required-sections`（`code-generation-plan.md`） | PASS（H2 7 本、findings 0） | 節構成に不足なし |
| `aidlc-sensor-required-sections`（`code-summary.md`） | PASS（H2 8 本、findings 0） | 同上 |
| `aidlc-sensor-required-sections`（`unit-test-instructions.md`） | PASS（H2 5 本、findings 0） | 同上 |
| `aidlc-sensor-traceability`（`traceability.json`） | pass=false、ただし `gaps` / `orphans` / `invalid_targets` / `invalid_entries` / `missing_from_table` はいずれも空。`missing_from_upstream_ids` 40 件のみ | 40 件は他 Unit 所有の要求 ID（FR1〜FR9 系・NFR1 / NFR2 / NFR4 / NFR5）で、依頼書 A.4 が既知のノイズと指定したもの。本 Unit の 46 ID は target がすべて実在ファイル単体で解決した |
| `linter` / `type-check` センサー | 対象外 | 生成物が Rust / Quint / Markdown / JSON のみ。TS/JS 生成コードが無いため適用対象なし（依頼書 A.4） |
| `git diff -- formal/orchestration/journal_protocol.qnt` | 凡例コメント 5 行のみ（`:10` / `:11` / `:15` / `:22` / `:23`）、`--stat` は 5 insertions / 5 deletions | 状態機械本体（`var` / `action` / `val` / `run` / witness）に 1 文字の変更もない。`source-manifest.json` の `writes` 1 件と一致 |
| `grep -rn WorkflowExecution modules tests scripts .github Cargo.toml tools formal` | 0 件 | 旧名の残存なし。追従先の名前もすべて実在（`intent_execution.rs` の `with_version:254` / `new:290` / `replay:352` / `seq_nr:393` / `version:402`、`port/intent_execution_repository.rs` の `IntentExecutionRepository:59` / `find_by_id:70` / `store:93`） |
| 凡例に残る U4 側・その他の名前 | 実在を確認 | `journal_reader.rs` の `events_after:77` / `checkpoint:85`（引数は `&ProjectionName`、戻りは `GlobalSeqNr`）/ `advance_checkpoint:109`。凡例の `JournalReader::checkpoint(ProjectionName)` は現行シグネチャと一致 |
| Unit 限定コマンド 11 本（`cargo test --locked`） | 契約 22 / 実装固有 23 / 本家適合 10 / `impl` 10 / `store_failure` 4 / `snapshot_strategy` 2 / `dto` 47 / `store_path` 4 / `intent_dir_name` 9 / クラッシュ再構成 5 / ITF 適合 5、すべて failed 0 / ignored 0 | 11 本すべて `code-summary.md` §1.2 の記録と同値。`dto` 47 は `unit-test-instructions.md` §2 の期待 29 と食い違い、M-4 の記載どおり |
| `bash scripts/quint-gate.sh` | `[PASS] quint gate: all steps green`、`[PASS]` 印字 51 行 = 25 ステップ × 2（実行時 + 末尾サマリ）+ 総括 1 行 | 「全 25 ステップ PASS」の記載は正確。凡例追従後にコメント変更が状態機械を壊していないことの機械確認になっている |
| `bash scripts/coverage.sh` | head line coverage **99.15169660678644%**、`[PASS] absolute gate: head (99.15169660678644%) >= threshold (90.0%)` | 記録値と小数以下まで完全一致（レビュアー実行は 4 回目に相当）。絶対 90% 床を満たす |
| `cargo audit` | advisory DB 取得成功（1239 advisories）、`Cargo.lock` の **125 crate** を走査、脆弱性報告なし | 記録どおり |
| `cargo test --manifest-path tools/lint/Cargo.toml` | 93 passed / 0 failed / 0 ignored | 記録どおり |
| 退役語彙 grep（13 語を `modules tools scripts formal .github Cargo.toml`） | 0 件。`ls formal/orchestration/` = `engine_loop.qnt` / `journal_protocol.qnt` / `stop_hook.qnt` | 記録どおり |
| 件数・行番号の抜取照合 | `dto/` 32 エントリ、`IntentExecutionEventDto` 16 変種、`CorruptDetail` `:101-113` 6 変種、`io_kind` `:20`〜`:34`、`store` ガード `:447`〜`:452`、保存分岐 `:465`、`stored_version` `:313-320`、`find_by_id` `:331-439` と `with_version(version)` `:438`、ファサード `pub use` 起点 `:38-41` / `:46` / `:52` / `:62` / `:65-66`（最後は `:68` で閉じる）、`runtime.rs` の `IntentExecutionRepositoryImpl::open` 8 か所、ITF fixture 8 本、CI 7 ジョブ、`ci.yml:186-190` の `cargo audit` 2 件、`Cargo.toml:119` の `=3.0.0` ピン、workspace lints 50 ルール、`cargo lint` 7 ルール | すべて `code-summary.md` の記載と一致。M-1（`new` は `:290-337`、`:338` 以降は `replay` の doc コメント）・M-3（`:321` は impl の閉じ括弧）・M-5（最後の `pub use` は `:66`〜`:68`）・M-6（`:137` は §6 `:133` の配下で、§5 は `:121`）も実測で正当な不一致だと確認した |
| 設計「固定するテスト」の実在と assert の内容（抜取 5 件） | `an_event_from_another_execution_is_rejected_before_writing`（`support/contract.rs:300`）・`a_genesis_with_a_non_zero_version_is_a_contract_violation`（同 `:276`）・`a_tampered_snapshot_payload_is_corrupt`（`intent_execution_repository_impl_test.rs:227`）・`a_journal_row_with_a_foreign_manifest_is_refused_before_replay`（同 `:476`）・`a_replayed_event_naming_a_stage_outside_the_plan_crashes_reconstruction`（同 `:361`）がすべて実在 | 本文まで読んだ 1 件（層 (0)）は、設計の説明どおり「別実行のイベントを本家呼出**前**に `Corrupt` で拒む」「双方 `NotFound` を維持」「更新拒否後に元の状態が読める」を assert している。設計の検査内容と実テストの assert が一致 |
| `docs/specs/deviations.md`（NFR1.1 target） | 実在 | 要求 NFR1.1 自身が `deviations.md:10` を証跡として挙げており、target として妥当 |
| `coding-rules/README.md` の機械化ロードマップ | `:89` が「実装済みは **6 本**」、実測は 7 本 | `code-summary.md` §7 の申し送り（README ロードマップ節だけが古い）は正確。本 Unit の所有外なので未修正で妥当 |
| 上流 `requirements.md` の旧名・`audit_lock.qnt` 参照 | 本レビューでは解消扱いにしない | 依頼書 A.3 の指定どおり、上流所見は本ステージの判定に含めない。§7 の申し送り（6 か所、オーナー裁定待ち）に記載済み |

### Summary

ワークスペース側の変更は `formal/orchestration/journal_protocol.qnt` の凡例コメント 5 行だけで、状態機械本体は 1 文字も動いておらず、
追従先の名前（`IntentExecution::version()` / `::seq_nr()`、`IntentExecutionRepository::find_by_id` / `::store`）はすべて現行コードに実在する。
同じ凡例に残る U4 側の名前と `GlobalSeqNr` / `RepositoryError::Conflict` も現行シグネチャと一致しており、旧名の残存は全範囲で 0 件である。
`code-summary.md` の実測記載は、Unit 限定コマンド 11 本の件数・quint-gate 25 ステップ・coverage 99.15169660678644%・`cargo audit` 125 crate・
`tools/lint` 93 本・退役 grep 0 件・行番号と件数の抜取まで、レビュアーの独立再実測とすべて一致した。過去（B5 の 674 / 98.42%）を今回の実施として
書いている箇所も見当たらず、未検証範囲（全 CI 実行・複数プロセス並行・`reopened()` 兄弟接続・末尾欠落・改竄の検出範囲・`SnapshotStrategy` 配線）は
過大主張なく区別されている。振る舞いに関わる設計と実装の不一致は 0 件で、Red テストを構成できる不一致も無い。

残る 4 件はいずれも Minor で、承認を止めるものではない。承認前に人間が重みづけすべきなのは、(1) §3 照合表の行範囲の数え方が二重基準になっており
M-2 が不要な設計文書改訂を持ち込みうること（R-01）、(2) BR5.2 の報告範囲の規律という半分が `.github/workflows/ci.yml` では検収できないこと（R-02）、
(3) ステージ定義の無条件 MUST（テストファイル・テスト構成のステップ）を「新規プロダクションコード 0」を理由に免除する読み替えを、暗黙にせず
明示的に裁定すべきこと（R-03）、(4) NFR1.2 の target が不在の検収手段ではなく置換後モデルを指していること（R-04）である。
