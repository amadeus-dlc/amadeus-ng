# Code Generation 計画 — Issue #134 `replace_pipeline` を IMMEDIATE に揃える（改訂 1）

## 改訂 1 の理由

1 回目の実装は PR [#154](https://github.com/amadeus-dlc/amadeus-ng/pull/154) で CI の全ジョブが緑になった（行カバレッジ 97.44%）。しかし、CodeRabbit のレビューで再現テストの同期に指摘を受けた。

- **指摘**: ホルダは「書込ロックを握った」合図の直後から 200ms（HOLD）を数える。主スレッドが `replace_pipeline` を呼ぶ前にホルダが COMMIT してしまうと、修正前の DEFERRED でもテストが通る。そうなると、テストが修正の効果を見分けられない。
- **判断（人、Deployment Execution）**: テストを直す。Code Generation へ差し戻し、この改訂計画の承認をやり直す。

`replace_pipeline` 本体の修正（IMMEDIATE への変更）は変えない。変えるのは再現テストだけである。

## 変更の概要

- **対象**: `modules/core/read-model-updater/src/orchestration/journal_reader_impl.rs` の `JournalReaderImpl::replace_pipeline`（1100〜1127 行）
- **原因**: このメソッドだけが DEFERRED のトランザクション（`self.connection.transaction()`、1105 行）で始まっている。`SELECT … FROM read_pipeline_progress` の後に `DELETE` / `INSERT` へ書込昇格するので、別の接続が書込ロックを握っていると、SQLite は busy handler を呼ばずに即 `SQLITE_BUSY` を返す。
  - その結果 `JournalReadError::Io(WouldBlock)` → `CatchUpError::Read` → 記録済みの更新動詞が終了コード 1、という経路で失敗する。
  - 同じファイルのほかの書き込みは、すべて `transaction_with_behavior(TransactionBehavior::Immediate)` で始まっている。
- **修正**: トランザクションの開始を `transaction_with_behavior(TransactionBehavior::Immediate)` に変える（ほかの書き込みと同じ作法）。これで最初に書込ロックを取るので、既存の busy timeout（5000ms）の範囲で待てるようになる。
  - 既存の `replace_steering` と同じ形で、理由を 1〜2 行のコメントに残す。
  - DEFERRED を選べる引数や分岐は作らない（`no-backward-compatibility.md`）。
- **再現テスト**: RMU 層の単体テストを 1 本足す。修正前は即 `WouldBlock`、修正後は待ってから成功することで、両者を判別する。

## 要件との対応

| 計画のステップ | 要件 ID |
| --- | --- |
| Step 2（ランナーの確認） | FR4.2 |
| Step 3（実装は 1 回目のまま） | FR2.1、FR2.2、FR2.3、NFR1 |
| Step 4（再現テストの同期を強める） | FR3.1、FR3.2、FR2.2 |
| Step 5（修正前に赤になることの確認） | FR3.1 |
| Step 6（改訂したテストの安定性） | FR3.2 |
| Step 7（関係する既存テストと契約テスト） | FR1.1、FR1.2、FR1.3、FR4.1 |
| Step 8（静的検査） | NFR5 |
| Step 9（要約・マニフェスト・トレーサビリティ） | 全 ID の追跡 |

FR5.1 は 1 回目で満たした（#151・#152 を起票済み）。

NFR2（CI の安定性）は、修正 PR の CI で確かめる（Build and Test 以降）。

## Testing Contract

```json
{
  "version": 1,
  "methodology": "test-after",
  "source": "org",
  "ordering": "implement each applicable testable layer, then write and run",
  "scope": "bugfix",
  "test_strategy": "minimal",
  "project_type": "brownfield",
  "applicable_notes": [
    {
      "layer": "org",
      "text": "We treat tests as a first-class deliverable in every Bolt. The specific\nmethodology (TDD, BDD, ATDD, or classic test-after) is affirmed at\npractices-discovery and recorded in `team.md` under this heading with explicit\n`Methodology` and `Ordering` fields; Code Generation resolves those fields\nindependently from coverage, tooling, and scope notes.\n\nWhen no posture has been affirmed, our default per scope is:\n- **Methodology**: test-after\n- **Ordering**: implement each applicable testable layer, then write and run\n  that layer's tests.\n- `mvp`, `enterprise`, `feature`, `infra`, `classic` add an 80% line-coverage\n  floor and CI execution before merge.\n- `bugfix`, `security-patch` add a targeted regression for the specific\n  bug/vulnerability and require the existing suite to remain green.\n- `express` uses the Minimal strategy: requirement-driven unit tests (one per\n  requirement, with a happy-path floor per component); existing tests remain\n  green.\n- `poc`, `refactor`, `workshop` add no extra new-test floor and require the\n  existing suite to remain green.\n\nThe active `Test Strategy` still applies in every scope and determines test\nvolume/types. Scope floors are additive; they never reduce or replace the\nselected strategy.\n\nBuild and Test verifies defined coverage floors and affirmed quality targets;\nthey may not be weakened to make a step pass.\n\nAffirm a stricter posture in `team.md` if the team commits to one."
    }
  ],
  "obligations": {
    "strategy": "minimal",
    "strategy_volume": [
      "One verifiable test per requirement at the narrowest effective level.",
      "At least one happy-path unit test per component.",
      "Unit tests are the default; a bugfix/security scope floor may require an integration or E2E regression when that is the narrowest level that reproduces the defect."
    ],
    "scope_floor": [
      "Include a targeted regression for the bug or vulnerability.",
      "Keep the existing test suite green."
    ],
    "combination_rule": "Apply every selected-strategy obligation and every scope-floor obligation; neither replaces the other, and a targeted scope regression may add the narrowest necessary test type beyond the strategy default."
  },
  "plan_profile": {
    "methodology": "test-after",
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
      "Data model / database behavior - implement.",
      "Data model / database behavior - write and run its tests after implementation.",
      "Repository / data access - implement.",
      "Repository / data access - write and run its tests after implementation.",
      "Business logic - implement.",
      "Business logic - write and run its tests after implementation.",
      "API / endpoint - implement.",
      "API / endpoint - write and run its tests after implementation.",
      "Frontend behavior - implement.",
      "Frontend behavior - write and run its tests after implementation.",
      "Environment/build configuration.",
      "Documentation and traceability."
    ]
  },
  "input_sha256": "sha256:933f32e0a09112bec79e6ad9d9b07d8fa79b601a7b325fbe1f9a283f191ae54c",
  "contract_sha256": "sha256:99478f464d6c420b35c11685f3bcb32edde86a8c85b8236e74f7cf898a3796b8"
}
```

### 層の当てはめ

方法論は **test-after**（その層を実装してから、その層のテストを書いて走らせる）。

- 該当する層は **Repository / data access** だけ。`JournalReaderImpl` は RMU の SQLite アクセスの実装である。
- Data model / database behavior は該当しない。表の DDL（`read_tables/sql.rs`）は変えないので、スキーマの変更は無い。
- Business logic、API / endpoint、Frontend behavior も該当しない。
  - 集約・ユースケース・投影核の判断は変えない。
  - CLI の出力契約は変えない（NFR4）。
  - UI は存在しない。
- Project structure と Environment/build configuration も該当しない。既存クレートの 1 メソッドの修正であり、依存・設定・ビルド構成は変えない。

## Steps

### Step 1: 作業の前提を確かめる

- [x] 本体の作業ツリーで `git diff -- modules` を確かめる。差分が `journal_reader_impl.rs` の 1 回目の変更（IMMEDIATE への変更と再現テスト）だけであることを確認する。
  - 本体の作業ツリーには、コミットしないローカル変更（`.claude/settings.json` と `.codex/hooks.json` の削除）がある。これらには触れない。

### Step 2: 既存のテストランナーを確かめ、絞ったコマンドを記録する（runner_ready_before_first_test）

- [x] `unit-test-instructions.md` の再現テストのコマンドが走り、1 回目のテストが緑であることを確かめる（改訂前のベースライン）。

### Step 3: Repository / data access — 実装は変えない（FR2.1〜FR2.3、NFR1）

- [x] `replace_pipeline` の IMMEDIATE への変更と理由のコメントは 1 回目のまま残す。本体のコードには手を入れない。

### Step 4: Repository / data access — 再現テストの同期を強める（FR3.1、FR3.2、FR2.2）

テスト名 `replace_pipeline_waits_for_a_write_lock_held_by_another_connection` と置き場所は変えない。次の 2 点を変える。

- [x] **解放のタイマーを「呼ぶ直前」から数える**。
  - ホルダは `BEGIN IMMEDIATE` で書込ロックを握ったら、合図 1 を主スレッドへ送る。その後は合図 2（「これから `replace_pipeline` を呼ぶ」）を待つ。合図 2 を受けてから HOLD（200ms）の間だけ握り、COMMIT する。
  - 主スレッドは合図 1 を受けたら、合図 2 を送った直後に `replace_pipeline` を呼ぶ。合図 2 から呼び出しまでの間には、スレッドの切り替え以外の処理を挟まない。
  - これで、ホルダの解放は必ず「呼び出しの直前」から HOLD 後になる。主スレッドの呼び出しより先にホルダが放してしまう窓は、合図 2 の送信から呼び出しまでの間（マイクロ秒の単位）だけになる。
- [x] **待ちを観測したことを所要時間で確かめる**。
  - `replace_pipeline` の呼び出しを `std::time::Instant` で囲み、所要時間が HOLD の半分（100ms）以上であることを表明する。
  - これで、万一ホルダが呼び出しより先に放していた場合でも、テストは「修正前でも通る」形にはならない。ロック待ちを観測しなかったことが、赤として表に出る。
  - 修正前（DEFERRED）は即 `Err(WouldBlock)` なので、結果の表明で落ちる。
  - ロック取得の試行そのものを通知する口（busy handler など）は本番の `JournalReaderImpl` に無い。テストのためだけにそれを足すと、`abstract-data-type.md`（テストのために表現を公開しない）に反する。そのため所要時間で観測する。この理由をテストのコメントに書く。
- [x] 失敗の経路でもスレッドを取り残さない作りは保つ。合図 2 の送信側が落ちたら、ホルダはすぐ解放して終わる。
- [x] busy timeout（2000ms）と HOLD（200ms）の関係と、100ms の下限の理由をテストのコメントに書く。
- [x] `unit-test-instructions.md` のコマンドで、このテストだけを走らせて緑を確かめる。所要時間も記録する。

### Step 5: 修正前に赤になることを確かめ直す（FR3.1 の証拠）

- [x] `replace_pipeline` のトランザクション開始を、一時的に元の `self.connection.transaction()` に戻す。
- [x] 改訂したテストだけを走らせ、`Err(Io { kind: WouldBlock, .. })` で即座に失敗する出力を `code-summary.md` に記録する。
- [x] 差し戻しを元に戻し（IMMEDIATE）、同じテストが緑になることを確かめる。
- [x] `git diff` で、最終成果物に DEFERRED の行が残っていないことを確かめる。**一時的な差し戻しは最終成果物に残さない。**

### Step 6: 改訂したテストの安定性を確かめる

- [x] 改訂したテストだけを `--exact` で 30 回続けて走らせ、すべて緑であることと、各回の所要時間の範囲を記録する。

### Step 7: 関係する既存テストと契約テストを走らせる（FR1、FR4.1）

- [x] `cargo test -p core-read-model-updater` を走らせる。失敗 0 を確かめる。
- [x] 契約テスト 3 本（`unit-test-instructions.md` のコマンド）を走らせ、緑を確かめる。

### Step 8: 静的検査（NFR5）

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --workspace --all-targets -- -D warnings`
- [x] `cargo lint`
- [x] ワークスペース全体のテストとカバレッジは、ローカルの macOS では SIGKILL のために判定できない（1 回目の Build and Test）。そのため、この段では走らせない。Build and Test（作業用のチェックアウト）と PR の CI で確かめる。

### Step 9: Documentation and traceability

- [x] `code-summary.md` に「改訂 1」の節を足す。変更内容、修正前の赤、30 回の結果、所要時間を書く。1 回目の記録は消さない。
- [x] `source-manifest.json` の `writes` が `journal_reader_impl.rs` の 1 件のままであることを確かめる（変更が要らなければ書き換えない）。
- [x] `traceability.json` の FR3.1・FR3.2 の対象が引き続き正しいことを確かめる（変更が要らなければ書き換えない）。
- [x] この計画のチェックボックスを、終えたステップから順に `[x]` にする。

## 変更するファイル

| ファイル | 変更 | 影響 |
| --- | --- | --- |
| `modules/core/read-model-updater/src/orchestration/journal_reader_impl.rs` | `replace_pipeline` のトランザクション開始 1 行とコメント、テスト 1 本の追加 | 低。`replace_pipeline` の呼び手は `read_model_updater.rs:62` の `catch_up_pipeline` だけで、振る舞いは「待てるようになる」だけ |

## 変更しないと決めたもの

- `hook_health_reader.rs:181`、`workspace_doctor_read_model_updater.rs:183`、`CatchUpError` の文言（Q2 = A。Issue に記録）
- 投影の再試行の仕組み（Q1 = A）
- CLI を跨ぐ契約テストの追加（Q3 = A）
- busy timeout の値、ジャーナルモード、本家ストアの使い方
- `runtime.rs` の `after_projection` の失敗境界。恒常的な失敗の扱いは変えない（FR2.3、FR4.1）
