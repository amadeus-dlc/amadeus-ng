# コード生成の要約 — Issue #134 `replace_pipeline` を IMMEDIATE に揃える

## 変更したファイル

| ファイル | 変更 |
| --- | --- |
| `modules/core/read-model-updater/src/orchestration/journal_reader_impl.rs` | `JournalReaderImpl::replace_pipeline` のトランザクション開始を `transaction_with_behavior(TransactionBehavior::Immediate)` に変え、理由のコメントを 2 行足した。再現テスト `replace_pipeline_waits_for_a_write_lock_held_by_another_connection` を 1 本足した |

このほかのアプリケーションソース、テスト・ビルド・カバレッジの設定には触れていない。

## 主な判断

- **トランザクションの開始だけを変えた**（FR2.1）。`path` の持ち方は、ほかの書き込み（`replace_testing` など）と同じ `let path = self.path.clone();` に揃えた。DEFERRED を選べる引数や分岐は作っていない（`no-backward-compatibility.md`）。
- **理由のコメント**は `replace_steering` の語調に合わせ、「読み取ってから書くので最初に書込ロックを取る」「DEFERRED からの書込昇格は busy timeout を待たずに即 `SQLITE_BUSY` になる (Issue #134)」の 2 行にした。
- **busy timeout（`DEFAULT_BUSY_TIMEOUT` = 5000ms）は変えていない**（NFR1）。タイムアウトを超えた場合の失敗の写し方（`at_store` → `JournalReadError::Io { kind: WouldBlock }`）も変えていないので、FR2.3 の振る舞いはほかの IMMEDIATE の書き込みと同じ経路になる。
- **1106 行と 1123 行の長大行には手を入れていない**（TD-6 は射程外）。`cargo fmt` が折り返したのは、新しいテストの中の 1 か所だけである。
- **再現テストのホルダ**は、計画が許す 2 つの形のうち「合図の後に一定時間（HOLD = 200ms）握ってから解放する」形にした。解放指示のチャネルは `recv_timeout(HOLD)` で待つ。主スレッドが送信側を落とせば、ホルダはすぐに解放して終わる。これで次の 2 つが両立する。
  - 失敗の経路でもスレッドを取り残さない（単体テスト手順書の「テストデータの扱い」）。
  - 修正前の赤が HOLD を待たずに即時に終わるので、所要時間でも「待たずに失敗した」ことが見える。
- **行の検証を空振りさせない工夫**: 計画は「0 件の表なら 0 件」を確かめるとしている。何も無い表で 0 件を確かめるだけでは、書き込みが届いたかどうかを判別できない。そこで、事前に同じ `execution_id` の古い行を 1 行置き、置き換え後に 0 件になることを確かめる形にした。
- busy timeout は計画どおり 2000ms とした。HOLD（200ms）が busy timeout より十分短いことは、テストのコメントに書いてある。

## ベースライン（修正前、`main` = `5a2b51d3`）

| コマンド | 通過 | 失敗 | 無視 |
| --- | --- | --- | --- |
| `cargo test -p core-read-model-updater`（lib 342 + 統合テスト 20 本 + doc-test 0） | 655 | 0 | 0 |
| `cargo test -p aidlc --test pipeline_link_contract` | 14 | 0 | 0 |

`concurrent_duplicate_completions_persist_only_one_receipt` は、ベースラインの 1 回では緑だった（タイミング次第のテストなので、結果を記録するだけにした）。

## ランナーの確認（Step 2）

`cargo test -p core-read-model-updater --lib orchestration::journal_reader_impl::tests::a_write_lock_held_by_another_connection_is_reported_as_would_block -- --exact` は、`unit-test-instructions.md` のコマンドと一字一句同じで、1 通過だった。新しいテストの名前は計画の例（`replace_pipeline_waits_for_a_write_lock_held_by_another_connection`）と同じにしたので、手順書のフィルタもそのまま使える。

## 修正前の赤（Step 6、FR3.1 の証拠）

トランザクションの開始を一時的に `self.connection.transaction()` に戻し、新しいテストだけを走らせた結果:

```text
test orchestration::journal_reader_impl::tests::replace_pipeline_waits_for_a_write_lock_held_by_another_connection ... FAILED
assertion `left == right` failed: 書込ロックの解放を待って置き換える
  left: Err(Io { kind: WouldBlock, path: Some("/var/folders/.../aidlc/spaces/default/intents/.aidlc-store.sqlite") })
 right: Ok(())
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 342 filtered out; finished in 0.05s
```

- テストバイナリ全体の所要が 0.05 秒なので、失敗は busy timeout（2000ms）も HOLD（200ms）も待たずに、ほぼ 0 秒で起きている（FR3.2 の「タイミングに依存しない」）。
- IMMEDIATE に戻した後は、同じテストが緑になった（0.26〜0.31 秒。HOLD の 200ms を待ってから成功している）。
- 差し戻しには修正後のファイルの控えを使い、`cmp` でバイト一致を確かめた。`git diff` と `grep` でも、`.transaction()`（DEFERRED）の行がファイルに残っていないことを確かめた。

## 修正後のテスト結果

### 関係するテスト（Step 7）

| コマンド | 通過 | 失敗 | 無視 |
| --- | --- | --- | --- |
| `cargo test -p core-read-model-updater` | 656（ベースラインの 655 に新テスト 1 本を加えた数） | 0 | 0 |
| `cargo test -p aidlc --test pipeline_link_contract`（修正直後の 1 回） | 14 | 0 | 0 |
| 契約テスト 3 本を `--exact` で 20 回（`unit-test-instructions.md` のコマンド） | 20 回とも 3 通過 | 0 | 0 |

20 回の各回の所要は 1.64〜2.26 秒だった。この 3 本は `a_publication_failure_preserves_the_receipt_for_recovery`、`a_failed_pipeline_projection_preserves_the_prior_query_result_until_recovery`、`concurrent_duplicate_completions_persist_only_one_receipt` である。これは NFR2 の補助証拠にとどまる。本番の判定は CI で行う。

### 静的検査と全体テスト（Step 8）

| 検査 | 結果 |
| --- | --- |
| `cargo fmt --all -- --check` | 通過 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 通過 |
| `cargo lint` | 通過 |
| `cargo test --workspace` | **緑にならなかった**。失敗 3 件は、いずれも今回の変更と無関係と判断した（下記） |
| `scripts/coverage.sh` | 走らせていない（任意）。閾値・除外の設定には触れていない |

`cargo test --workspace --no-fail-fast` の結果は、テストバイナリ 140 本・通過 4056・失敗 3・無視 0 だった。失敗は次の 3 件である。

1. `engine_hook_wiring_contract::the_binding_definition_counts_registrations_apart_from_hook_names`（`left: Some(18)`、`right: Some(17)`）
   - 原因は、作業ツリーの `.claude/settings.json` にあるコミットしないローカル変更である。`run-sensors` のフック登録が `aidlc engine hook run-sensors` から `bun ".../aidlc-run-sensors.ts"` に書き換わっている。このテストは作業ツリーの `settings.json` を読んで登録件数を数えるので、件数がずれる。
   - 依頼の制約によりこのローカル変更には触れていない。コミット対象の `settings.json` では起きない。
2. `pipeline_link_contract::public_link_inputs_and_results_match_fixed_upstream`
3. `pipeline_link_contract::single_pipeline_requires_its_own_open_attempt_and_receipts`
   - 2 と 3 はいずれも、子プロセス（`aidlc-log` / `aidlc` のハードリンク）が **SIGKILL**（`ExitStatus(unix_wait_status(9))`、stdout・stderr とも空）で終わっていた。
   - 失敗するテストは回ごとに入れ替わる。単独実行でも断続的に起きる。
   - 最初の `cargo test --workspace` では、`classic_corpus_contract` の 2 本も `intent-create` の失敗で落ちた。同じテストを単独で修正前 1 回・修正後 5 回走らせると、すべて緑だった。

**修正前後の A/B（`cargo test -p aidlc --test pipeline_link_contract` を 8 回ずつ）**

| ソース | 緑の回数 | 落ちた回（落ちたテスト） |
| --- | --- | --- |
| 修正前（`git show HEAD:` の内容に一時的に戻す） | 6/8 | 5 回目 `an_open_gate_cannot_reuse_receipts_after_the_handoff_changes`（`unix_wait_status(9)`）、6 回目 `pipeline_progress_and_gate_refusals_match_the_fixed_upstream_observations`（stdout が空で JSON を読めない） |
| 修正後 | 5/8 | 5〜7 回目 `public_link_inputs_and_results_match_fixed_upstream` |

- 修正前のソースでも、同じ症状（SIGKILL、出力なし）で落ちる。落ちるテストも一定しない。
- 失敗は、ロック待ち（busy timeout）の経路とは無関係な、プロセスの起動直後に起きている。
- 以上から、この失敗は今回の修正が持ち込んだものではなく、以前からある環境由来の不安定さと判断した。
- macOS のクラッシュレポートと統合ログには、この SIGKILL の理由は残っていなかった。
- A/B の後は、修正後のファイルの控えに戻して `cmp` でバイト一致を確かめた。

FR4.2（ワークスペース全体の緑）は、ローカルでは上記の理由で確認できていない。コミット対象の状態での判定は、Build and Test と CI に委ねる。

## 計画からの逸脱

- **Step 8 の `cargo test --workspace` が緑にならなかった**。原因は上記の 3 件で、今回の変更によるものではないと判断した。閾値や設定は緩めていない。
- 切り分けのために、次の検査を計画の外で追加した。いずれもソースは修正後の状態に戻し、バイト一致を確かめた。
  - `cargo test --workspace --no-fail-fast`
  - `classic_corpus_contract` の修正前 1 回・修正後 5 回の単独実行
  - `pipeline_link_contract` の修正前後 8 回ずつの A/B
- 再現テストに、古い行を 1 行置いてから置き換える準備を足した。計画の「0 件の表なら 0 件」を、空振りしない形で確かめるためである。テストの本数・名前・置き場所・コマンドは計画どおりである。

## 射程外の所見（Issue にする文面、FR5.1）

GitHub Issue の起票は進行役が行う。以下はその文面の案である。

### 案 1: RMU の残り 2 か所の「読んでから書込へ昇格する」トランザクションを IMMEDIATE に揃える

- **場所**: `modules/core/read-model-updater/src/orchestration/hook_health_reader.rs:181`、`modules/core/read-model-updater/src/orchestration/workspace_doctor_read_model_updater.rs:183`
- **現象**: どちらも `self.connection.transaction()`（DEFERRED）で始まる。トランザクション内の最初の文は `CREATE TABLE IF NOT EXISTS`（表があれば読むだけ）で、その後に `DELETE` / `INSERT` へ進む。別の接続が書込ロックを握っていると、書込昇格は busy timeout を待たずに即 `SQLITE_BUSY` になりうる。これは Issue #134 と同じ形である。
- **追加の問題**: どちらも SQLite の失敗をすべて `corrupt_error(.., CorruptCause::InvariantViolation)` に写している。そのため、再実行で解けるロック競合が「ストアが壊れている」と誤分類される（コード知識ベースの TD-2）。
- **提案**: 開始を `transaction_with_behavior(TransactionBehavior::Immediate)` に揃える。あわせて、busy を `Corrupt` ではなく `Io { kind: WouldBlock }` に写す。修正前に赤になる再現テストを、#134 の `replace_pipeline_waits_for_a_write_lock_held_by_another_connection` と同じ作りで 1 本ずつ足す。
- **影響**: pipeline link の経路には乗らない。hook-health と doctor の投影を並行して更新したときに現れる。

### 案 2: 投影の失敗文言に、どの段で失敗したかを載せる

- **場所**: `modules/core/read-model-updater/src/orchestration/catch_up_error.rs`（`CatchUpError::Read` の `Display`）
- **現象**: `CatchUpError::Read(JournalReadError)` の表示は `read: io: WouldBlock at <path>` だけである。`prepare_read_model` / `events_after` / `replace_pipeline` / `publish` のどの段で起きたかを運ばないので、#134 の切り分けでは失敗箇所をコードから推定するしかなかった（TD-4）。
- **提案**: 段を表す材料（変種またはフィールド）を `CatchUpError` に足し、`Display` と出す側の `wording` で段を示す。`error-handling.md` に従い、文言ではなく材料で運ぶ。CLI の利用者向け文言（upstream 互換面）を変えるかどうかは、観測互換を確かめたうえで決める。

### 案 3（今回の検証で見つけたもの）: 契約テストの子プロセスが断続的に SIGKILL で終わる

- **場所**: `modules/app/aidlc/tests/pipeline_link_contract.rs`、`modules/app/aidlc/tests/classic_corpus_contract.rs` など、`tests/support/tool_link.rs` のハードリンクで `aidlc` を起動する契約テスト
- **現象**: macOS（Apple Silicon、ローカル）で、子プロセスがときどき `unix_wait_status(9)` で終わる。stdout と stderr は空である。修正前のソースでも 8 回中 2 回起きた。失敗するテストは回ごとに入れ替わる。クラッシュレポートと統合ログに理由は残っていない。
- **提案**: まず CI（Linux）で同じ症状が出るかを確かめる。ローカル限定なら、ハードリンクした実行ファイルの初回起動検査との関係を調べる（`tool_link.rs` の冒頭コメントにある、macOS の実行ファイル検査の件）。
- **注意**: #134 の NFR2（`concurrent_duplicate_completions_persist_only_one_receipt` のフレーク）とは症状が違う。#134 の症状は、終了コード 1 と `WouldBlock` の文言である。

## 改訂 1 — 再現テストの同期を強める（PR #154 のレビュー指摘への対応）

1 回目の再現テストでは、ホルダが「書込ロックを握った」合図の直後から HOLD（200ms）を数えていた。主スレッドが `replace_pipeline` を呼ぶ前にホルダが COMMIT してしまうと、修正前の DEFERRED でもテストが通ってしまう。改訂 1 では、この抜け道をテストから取り除いた。`replace_pipeline` 本体（IMMEDIATE への変更と理由のコメント）は 1 回目のまま変えていない。

### 変更内容

変えたのは、`journal_reader_impl.rs` の再現テスト `replace_pipeline_waits_for_a_write_lock_held_by_another_connection` だけである。名前と置き場所は変えていない。

- **HOLD を数え始める時点を「呼び出しの直前」に移した**。ホルダは `BEGIN IMMEDIATE` で書込ロックを握ったら合図 1 を送り、主スレッドからの合図 2（「これから `replace_pipeline` を呼ぶ」）を待つ。合図 2 を受けてから HOLD だけ握り、COMMIT する。主スレッドは合図 1 を受けたら合図 2 を送り、所要時間の計測を始めてすぐに `replace_pipeline` を呼ぶ。
- **ロック待ちを観測したことを、所要時間で確かめるようにした**。`replace_pipeline` の呼び出しを `std::time::Instant` で囲み、所要が `MIN_OBSERVED_WAIT`（HOLD の半分の 100ms）以上であることを表明する。結果の表明（`Ok(())`）の後に置いたので、修正前は結果の表明で落ちる。万一ホルダが呼び出しより先に放していれば、待ちが生じずに下限を割り、赤になる。
- **スレッドを取り残さない作りは保った**。ホルダは合図 2 を `recv()` で待ち、送信側が落ちたら（主スレッドが呼ぶ前に終わったら）待たずにすぐ解放する。合図 2 を受けた後の待ちは HOLD の `sleep` だけなので、どの経路でも終わる。
- **テストのコメントに理由を書いた**。書いたのは、busy timeout（2000ms）と HOLD（200ms）の関係、100ms の下限を選んだ理由、busy handler のような通知の口を本番の `JournalReaderImpl` に足さない理由（テストのために表現を公開することになる。`abstract-data-type.md`）の 3 点である。

### ランナーの確認（Step 2）と改訂後の緑（Step 4）

- 改訂前のベースライン: `unit-test-instructions.md` の再現テストのコマンドで 1 通過（0.25 秒）。
- 改訂後: 同じコマンドで 1 通過（0.25 秒）。

### 修正前の赤（Step 5、FR3.1 の証拠）

トランザクションの開始を一時的に `self.connection.transaction()`（DEFERRED）に戻し、改訂したテストだけを走らせた結果:

```text
thread 'orchestration::journal_reader_impl::tests::replace_pipeline_waits_for_a_write_lock_held_by_another_connection' panicked at modules/core/read-model-updater/src/orchestration/journal_reader_impl.rs:2079:9:
assertion `left == right` failed: 書込ロックの解放を待って置き換える
  left: Err(Io { kind: WouldBlock, path: Some("/var/folders/.../aidlc/spaces/default/intents/.aidlc-store.sqlite") })
 right: Ok(())
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 342 filtered out; finished in 0.24s
```

- テスト全体の所要（0.24 秒）には、落ちる前の `holder.join()` が待つ時間が入る。改訂後のホルダは合図 2 を受けてから HOLD だけ握るからである。そのため、この数字は「修正前の失敗も待っている」ことを意味しない。
- 呼び出しそのものの所要を確かめるため、`waited` を表示する 1 行を一時的に足して 3 回測った。`replace_pipeline` は **58〜125µs** で `Err(Io { kind: WouldBlock, .. })` を返しており、待ちに入らずに即座に失敗している（FR3.2）。
- 修正後の同じ一時表示では、呼び出しは **209〜257ms**（5 回）待ってから `Ok(())` を返した。下限の 100ms を大きく上回る。
- 差し戻しと一時表示には、修正後のファイルの控え（scratchpad）を使った。戻した後は `cmp` でバイト一致を確かめ、表示行（`TEMP-MEASURE`）が 0 件であることも確かめた。`git diff` でも、`self.connection.transaction()` の行は削除側（`-`）にしか現れず、最終成果物に DEFERRED の行は残っていない。

### 安定性（Step 6）

改訂したテストだけを `--exact` で 30 回続けて走らせた。

| 回数 | 緑 | 各回の所要（テストバイナリの `finished in`） |
| --- | --- | --- |
| 30 | 30 | 0.23〜0.30 秒 |

### 関係するテスト（Step 7）

| コマンド | 通過 | 失敗 | 無視 |
| --- | --- | --- | --- |
| `cargo test -p core-read-model-updater`（テストバイナリ 22 本） | 656 | 0 | 0 |
| 契約テスト 3 本（`unit-test-instructions.md` のコマンド） | 3 | 0 | 0 |

RMU クレートの件数は 1 回目と同じ 656 である（テストを書き換えただけで、本数は増えていない）。契約テスト 3 本の所要は 1.92 秒だった。

### 静的検査（Step 8、NFR5）

| 検査 | 結果 |
| --- | --- |
| `cargo fmt --all -- --check` | 通過（終了コード 0） |
| `cargo clippy --workspace --all-targets -- -D warnings` | 通過（終了コード 0） |
| `cargo lint` | 通過（終了コード 0） |

ワークスペース全体のテストとカバレッジは、計画どおりこの段では走らせていない。ローカルの macOS では子プロセスの SIGKILL のために判定できないからである（1 回目の記録を参照）。Build and Test（作業用のチェックアウト）と PR の CI で確かめる。閾値と設定には触れていない。

### 記録ファイル（Step 9）

- `source-manifest.json` の `writes` は `journal_reader_impl.rs` の 1 件のままなので、書き換えていない。
- `traceability.json` の FR3.1・FR3.2 の対象は、引き続き `journal_reader_impl.rs`（再現テストの置き場所）で正しいので、書き換えていない。

### 計画からの逸脱

- **一時的な計測行を足して測った**。修正前の赤が即時に起きたことを、テスト全体の所要（`holder.join()` の待ちを含む）では示せなかったからである。計測行は修正前 3 回・修正後 5 回の計測にだけ使い、控えから復元して最終成果物には残していない。30 回の安定性確認は、計測行の無い最終のテストで行った。
- 合図 2 の送信と `replace_pipeline` の呼び出しの間には、所要時間の計測開始（`Instant::now()`）だけを置いた。計画の「呼び出しを `Instant` で囲む」に従うために必要な処理であり、計測開始を合図 2 の前に置くと、ホルダが呼び出しより先に放した場合も下限を満たしてしまう。
