# 要件 — Issue #134 並行した pipeline link の完了報告で、記録に成功した側も WouldBlock で失敗する

## Intent Analysis

- **依頼**: Initial description: `Issue #134: 並行した pipeline link の完了報告が、記録に成功した側も WouldBlock で失敗として返る` [desc]
- **種類**: バグ修正（Workflow-selected scope: `bugfix` [scope]）。影響は RMU（`core-read-model-updater`）の 1 メソッドに閉じる。深さは Minimal。
- **目的**: 同じ pipeline link の完了報告が並行して 2 本届いたとき、記録に成功した起動が成功を返すようにする。今は、記録済みなのに後段の投影でロック競合に当たり、終了コード 1 を返している。その結果、CI の merge queue がフレークで止まっている（Issue #134 の 2 例目のコメント）。
- **原因の見立て**: `aidlc/spaces/default/codekb/amadeus-ng/architecture.md` の Interaction Diagrams に、記録から投影までの流れと、並行時の失敗の形をまとめてある。
  - 記録の後、更新動詞は必ず `after_projection` を通る。投影が失敗すると、記録済みでも失敗を返す。
  - 投影の中の `JournalReaderImpl::replace_pipeline`（`journal_reader_impl.rs:1105`）だけが DEFERRED トランザクションで読んでから書込へ切り替える。SQLite はこの切り替えで競合すると busy handler を呼ばずに即 `SQLITE_BUSY` を返すので、5000ms の busy timeout が効かない。
  - 同じファイルのほかの書き込みはすべて `TransactionBehavior::Immediate` である（`code-quality-assessment.md` の TD-1）。
  - この性質は SQLite 単体では実測済みだが、アプリ本体での再現はまだしていない [assumption]。
- **業務上の位置**: pipeline link の受領はステージ承認の前提である（`aidlc/spaces/default/codekb/amadeus-ng/business-overview.md`）。修正の対象ファイルと周辺の配置は `aidlc/spaces/default/codekb/amadeus-ng/code-structure.md` の「Issue #134 の経路にあるファイル」に従う。

## Functional Requirements

### FR1 並行した重複報告の結果

- **FR1.1** 同じ pipeline link の完了報告が並行して 2 本届き、一方が記録に成功したとき、その起動は終了コード 0 で `{"emitted":"PIPELINE_LINK_COMPLETED",...}` を返す。記録済みの完了が、投影の一時的なロック競合で失敗として報告されてはならない。[desc]
  - 合否: 契約テスト `concurrent_duplicate_completions_persist_only_one_receipt`（`modules/app/aidlc/tests/pipeline_link_contract.rs:409`）の「成功した起動の数が 1」のアサーションが通る。
- **FR1.2** もう一方の起動は、これまでどおり `already completed` で拒否される（終了コード 1）。
  - 合否: 同テストが拒否側の文言と終了コードを確かめて通る。
- **FR1.3** 監査の `PIPELINE_LINK_COMPLETED` は 1 件だけ記録され、成功を返した起動の中で公開まで終わっている（既存テストの 2 つ目のアサーション。`code-quality-assessment.md` の R-4）。
  - 合否: 同テストの監査件数のアサーションが、成功を返した直後の時点で通る。

### FR2 `replace_pipeline` のトランザクションの張り方

- **FR2.1** `JournalReaderImpl::replace_pipeline` は、ほかの書き込みと同じ `TransactionBehavior::Immediate` でトランザクションを始め、最初に書込ロックを取る。[Q1]
  - 合否: `replace_pipeline` のトランザクション開始が `transaction_with_behavior(TransactionBehavior::Immediate)` であり、DEFERRED の `transaction()` が残っていない。
- **FR2.2** 別の接続が書込ロックを握っている間に `replace_pipeline` が呼ばれたときは、接続の busy timeout の範囲で待つ。ロックが解放されたら、その後に成功する。[Q1]
  - 合否: FR3.1 の再現テストが成功する。
- **FR2.3** ロックが busy timeout を超えて握られ続けたときは、これまでどおり `JournalReadError::Io(WouldBlock)` で失敗し、更新動詞は記録済みでも終了コード 1 を返す。恒常的な投影失敗の扱いは変えない。[Q1]
  - 合否: FR4.1 の既存テストが通る。

### FR3 再現テスト

- **FR3.1** RMU 層の単体テストを 1 本足す。別の接続が `BEGIN IMMEDIATE` で書込ロックを握り、busy timeout より短い時間で解放する。その状況で `replace_pipeline` を呼び、成功し、`read_pipeline_progress` に期待した行が書かれていることを確かめる。[Q3]
  - 合否: 修正後に通る。修正前のコード（DEFERRED のまま）では即座に `WouldBlock` で失敗することを、同じテストで確かめてから修正を入れる（Issue の受入条件「修正前に失敗し、修正後に通る」）[desc]。
- **FR3.2** 再現テストは、既存の `a_write_lock_held_by_another_connection_is_reported_as_would_block`（`journal_reader_impl.rs:1977`）と同じ作り（`open_with_busy_timeout` と別接続でのロック保持）で決定的に競合を起こし、スリープの偶然に頼らない。[Q3]
  - 合否: ロック保持と解放の順序をテストのコードが制御しており、実行順やマシンの速さで結果が変わらない。

### FR4 既存の振る舞いを保つ

- **FR4.1** 恒常的な公開失敗・投影失敗を固定している既存の契約テスト 2 本、`a_publication_failure_preserves_the_receipt_for_recovery`（`pipeline_link_contract.rs:435`）と `a_failed_pipeline_projection_preserves_the_prior_query_result_until_recovery`（同 532 行）が、変更なしで通る（`code-quality-assessment.md` の R-3）。
  - 合否: 両テストが緑。
- **FR4.2** ワークスペースの既存テストスイート全体が緑のままである（org.md の Testing Posture: `bugfix` は既存スイートの緑を要求する）。
  - 合否: `cargo test --workspace` が成功する。

### FR5 射程外の所見の引き継ぎ

- **FR5.1** 同じ「読んでから書込へ切り替える」形の 2 か所（`hook_health_reader.rs:181`、`workspace_doctor_read_model_updater.rs:183`）と、投影の失敗文言がどの段で失敗したかを運ばない件（`code-quality-assessment.md` の TD-2・TD-4）を、GitHub Issue に記録する。[Q2]
  - 合否: 所見の場所と内容を書いた Issue が存在し、この修正の PR 本文から参照されている。

## Non-Functional Requirements

- **NFR1 待ち時間**: busy timeout は既存の 5000ms（`journal_reader_impl.rs:88`、`223`）を変えない。ロック競合のときに `replace_pipeline` が待つ時間は最大でこの値までとする。
  - 合否: busy timeout の設定値に差分が無い。
- **NFR2 CI の安定性**: 修正後、`concurrent_duplicate_completions_persist_only_one_receipt` が `coverage` ジョブを含む CI で落ちない。[desc]
  - 合否: 修正の PR の CI（`check` と `coverage`）が、再実行なしで緑になる。
- **NFR3 カバレッジの床**: ワークスペースの行カバレッジ 90% の床（`scripts/coverage.sh`）を割らない。
  - 合否: `coverage` ジョブが成功する。
- **NFR4 観測互換**: CLI の出力契約（stdout の JSON 1 行、stderr の診断、終了コード）と監査行の形は変えない。upstream には投影の後段が無いので、この修正は upstream との観測互換に影響しない（`code-quality-assessment.md` の TD-3）。
  - 合否: ゴールデン比較のテストと `aidlc-distribution` ジョブが緑。
- **NFR5 静的検査**: `cargo fmt`、`cargo clippy -D warnings`、`cargo lint` がすべて通る。
  - 合否: CI の `check` ジョブが緑。

## Constraints

- 変更は RMU クレート（`core-read-model-updater`）の中に限る。CQRS の側分割（`coding-rules/cqrs-boundaries.md`）を崩さない。
- `coding-rules/` の規則に従う。特に後方互換の口を残さない（`no-backward-compatibility.md`）。DEFERRED と IMMEDIATE を切り替える引数や、旧実装への分岐は作らない。
- 既存の書き込みの作法（`replace_steering` の doc コメント「読み取ってから書くので `BEGIN IMMEDIATE` で書込ロックを最初に取る (BR2.3)」）にそろえる。
- 深さ Minimal / Test Strategy Minimal。再現テストは FR3 の 1 本に絞り、テストを広げすぎない。

## Assumptions

- 競合相手の書込ロックの保持時間は、busy timeout（5000ms）より十分に短い [assumption]。仮説上の相手は重複した側の `store` で、楽観ロック失敗までの短い間だけ RESERVED ロックを持つ（`code-quality-assessment.md` の R-1）。この前提が崩れると、FR2.3 の経路で失敗が残る。
- 失敗の原因は `replace_pipeline` の DEFERRED 昇格である [assumption]。アプリ本体での再現はまだしていない。失敗文言からは失敗した段を特定できないので（TD-4）、ほかの段で起きている可能性は排除しきれない。FR3.1 で「修正前に即失敗する」ことを確かめるのは、この仮説の検証も兼ねる。

## Out of Scope

- 投影がロック競合で失敗したときの再試行の仕組み（Q1 で B・C を選ばなかった）[Q1]
- 同じ形の 2 か所（`hook_health_reader.rs:181`、`workspace_doctor_read_model_updater.rs:183`）の修正。記録だけする（FR5.1）[Q2]
- 投影の失敗文言にどの段で失敗したかを含める改善。記録だけする（FR5.1）[Q2]
- CLI の起動を跨ぐ契約テストでの決定的な再現（Q3 で B・C を選ばなかった）[Q3]
- ジャーナルモード（WAL）の変更、本家ストアのサポート範囲（NFR3.9）とのずれの解消（`code-quality-assessment.md` の R-6・R-7）
- PR #138 で観測された、macOS ローカルで 1 件だけ落ちる別事象（R-8）

## Open Questions

- FR3.1 の「修正前に失敗する」ことを、どの形で証拠に残すか（テストを先に入れて赤を確認してから修正するコミット順にするか、など）。Code Generation の計画で決める。
- NFR2 の「落ちない」は、修正 PR の CI 1 回分で判断する。フレークの頻度が低いので、それだけでは十分な証拠にならない可能性がある。追加の繰り返し実行が要るかは Build and Test で判断する。

## Sources

- [desc] Initial description: `Issue #134: 並行した pipeline link の完了報告が、記録に成功した側も WouldBlock で失敗として返る`。Issue 本文の「期待する振る舞い」「受入条件」と、2 例目のコメント（2026-09-23、PR #140）を含む。
- [scope] Workflow-selected scope: `bugfix`
- [Q1] 修正の方針 — A（`replace_pipeline` を IMMEDIATE に揃えるだけ）
- [Q2] 直す範囲 — A（`replace_pipeline` だけ。残りは Issue に記録）
- [Q3] 再現テストの置き場 — A（RMU 層の単体テスト）
- [assumption] 上の Assumptions 節の 2 項目
- コード知識ベース: `aidlc/spaces/default/codekb/amadeus-ng/business-overview.md`、`architecture.md`、`code-structure.md`、`code-quality-assessment.md`

## Assumptions & Open Questions

- Assumptions 節の 2 項目は未確認の仮定である。FR3.1 の再現テストで 2 つ目を確かめる。
- Open Questions 節の 2 項目は、後続のステージ（Code Generation、Build and Test）で決める。
