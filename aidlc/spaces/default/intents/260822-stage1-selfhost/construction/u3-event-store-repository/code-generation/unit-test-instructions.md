# unit-test-instructions — U3 イベントストアと IntentExecutionRepository（`u3-event-store-repository`）

> 対象: u3-event-store-repository（library）。現行 `code-generation-plan.md`（2026-09-07 再走）と Testing Contract、
> `../nfr-requirements/security-requirements.md` NFR2.1〜NFR2.5 / NFR4.1〜NFR4.7、`../nfr-design/logical-components.md` §4、
> `../functional-design/rules.md` BR5.2 に従う。以下はすべて本 Unit に限定する。2026-08-23 の旧手順（旧クレート名・旧テスト名）は
> `unit-test-instructions-history-2026-08-23.md` に全文保存した。

## 1. ランナーと設定

`cargo test`（Rust 1.95.0、`rust-toolchain.toml` で固定）。非同期ポートの契約テストは `tokio`（dev-dependency、`current_thread`）で回す。
SQLite バックエンドは `tempfile` の一時ディレクトリに `intents/.aidlc-store.sqlite` を作る。PBT はアダプタに無い（`proptest` を持つのは
`core-command-domain` / `core-infrastructure` / `core-query/use-case`）。ワークスペース全体のシード固定 `PROPTEST_RNG_SEED=20260823` は
本 Unit のコマンドには効かないが、受入 (f) の `cargo test --workspace` では必ず付ける。追加のランナー・モック・設定ファイルは導入しない。

## 2. Unit 限定コマンド

ワークスペースルートで実行する。`--locked` で `Cargo.lock` を変えない。

```sh
cargo test --locked -p core-command-interface-adapter --test intent_execution_repository_contract   # 契約 11 関数 × memory / SQLite
cargo test --locked -p core-command-interface-adapter --test intent_execution_repository_impl_test  # 実装固有（基底 + 差分・破損境界・I/O）
cargo test --locked -p core-command-interface-adapter --test upstream_event_store_conformance       # 本家適合 5 関数 × 2 バックエンド
cargo test --locked -p core-command-interface-adapter --lib orchestration::intent_execution_repository_impl  # インライン（CorruptDetail・封筒・写像・reopened）
cargo test --locked -p core-command-interface-adapter --lib orchestration::store_failure            # rusqlite code → ErrorKind の写像
cargo test --locked -p core-command-interface-adapter --lib orchestration::snapshot_strategy        # 既定 10 と任意間隔
cargo test --locked -p core-command-interface-adapter --lib orchestration::dto                      # DTO の往復・拒否
cargo test --locked -p core-command-domain --lib workspace::store_path                              # StorePath::for_space
cargo test --locked -p core-command-domain --lib workspace::intent_dir_name                         # IntentDirName の文法
cargo test --locked -p aidlc --test crash_reconstruction_test                                       # クラッシュ再構成（新接続で同値ほか）
cargo test --locked -p aidlc --test journal_protocol_conformance                                    # ITF 8 トレースの再生（every(1) 明示）
```

計画準備時（2026-09-07、ワークスペースは `origin/main` = `f2b6b6a9` と同一）の実測: 契約 22 / 実装固有 23 / 本家適合 10 / クラッシュ再構成 5、
いずれも PASS。`#[test]` + `#[tokio::test]` 属性の合計は impl 10（5 + 5）/ store_failure 4 / snapshot_strategy 2 / dto 29 / store_path 4 /
intent_dir_name 9 / journal_protocol_conformance 5（2 + 3）/ crash_reconstruction 5（契約と本家適合はマクロ展開のため属性数 2 と実行件数が異なる）。
実行担当は上記を再実行し、バイナリごとの件数・結果・完了時刻を `developer-report-11.md` へ残す。ITF 適合とクラッシュ再構成は、計画 Step 3
（Quint 凡例コメント 5 行の追従）の**後**にもう一度実行し、両方の結果を残す。ワークスペース全体の `cargo test --workspace` は CI の品質ゲートで
あり、本ファイルの Unit 限定コマンドではない。

## 3. 合格基準と検証範囲

- §2 の 11 コマンドがすべて終了コード 0、`failed` 0、`ignored` 0。
- 件数が計画準備時と違う場合は違うまま記録し、原因（テスト追加・削除・フィルタ不一致）を `git log -p` で調べて報告する。件数を合わせるために
  テストを足したり消したりしない。
- 受入（Unit 限定コマンドの外側、計画 Step 4）は次を実測して記録する。設定の存在と実働の成功を区別する。
  - `cargo fmt --all --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo lint` / `cargo test --manifest-path tools/lint/Cargo.toml`
    がすべて終了コード 0（`tools/lint` の自己テスト件数は出力から記録）。
  - `bash scripts/quint-gate.sh` が `[PASS] quint gate: all steps green`（journal_protocol の typecheck / 不変条件 8 / witness 4 を含む。計画 Step 3
    の凡例追従の**後**に実行し、コメント変更で状態機械が壊れていないことの機械確認とする）。
  - `bash scripts/coverage.sh` を同一リビジョン・同一ツールチェーン・同一シードで 2 回実行し、生の head 値（%）と差を記録する。絶対ゲート 90% が
    2 回とも成功すること。差 0.00 ポイントは受入目標であり、未達なら未達のまま原因を記録し、`TOLERANCE` / 除外 / シードを変えない。
  - `cargo audit`（workspace）と `cargo audit --file tools/lint/Cargo.lock` の結果、走査 crate 数、advisory DB 取得可否。未導入・取得失敗は
    成功と書かない。
  - 退役 grep（計画 Step 4 (e) の語彙）が `modules tools scripts formal .github Cargo.toml` で 0 件、`ls formal/orchestration/` が
    engine_loop / journal_protocol / stop_hook の 3 つ。
  - 旧名 grep（`WorkflowExecution`）が計画 Step 3 の前は `formal/orchestration/journal_protocol.qnt` の凡例 5 行（`:10` / `:11` / `:15` / `:22` /
    `:23`）だけ、Step 3 の後は `modules tests scripts .github Cargo.toml tools formal` で 0 件。`git diff --stat` がその 1 ファイル・5 行だけ。
  - `PROPTEST_RNG_SEED=20260823 cargo test --workspace` の総数と結果（全体ゲートとして 1 回。所要時間も記録）。
  - `rustc -V` が 1.95.0。

Unit 限定コマンドと上記実測の成功は、全 CI 実行・マージキューの完走・複数プロセスの並行書込・`reopened()` 複数ハンドルと兄弟接続の並行・
末尾欠落の検出の代替ではない（設計が未検証範囲として明記しているもの）。全体検証を Unit ごとに繰り返すコマンドはここへ置かない。

## 4. データとテスト支援

テストデータは各テストファイルのフィクスチャ（`IntentExecution::start` による genesis、`StageEntries` の合成計画、UUIDv7 リテラル）と
`tests/conformance/fixtures/journal_protocol/*.itf.json`（8 本、`#meta` 正規化済み）。テストダブルは使わず、`IntentExecutionRepositoryImpl::in_memory()`
（本家 memory バックエンド）と `open(&StorePath)`（本家 SQLite）を同じ契約で走らせる（BR2.7）。ネットワークは使わない（`cargo audit` の advisory DB
取得を除く）。認証トークン・環境変数を記録へ混ぜない。

## 5. 失敗時

失敗したコマンド・テスト名・出力を `developer-report-11.md` に記録する。計画 Step 3 の凡例追従以外で設計と実装の不一致が見つかれば、現行コードで
落ちる Red テスト案（対象ファイル・assert の内容・期待する失敗出力）を親セッションへ返し、計画を更新してからコードを変える。今回の記録現行化の
ためにコードを壊して人工的な Red を作らない。閾値・シード・除外・依存を変えて成功させない。
