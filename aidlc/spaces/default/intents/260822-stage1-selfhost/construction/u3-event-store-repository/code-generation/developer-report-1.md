# developer-report-1 — 委任 1: ロック系の退役 + U2 是正（U3 / Bolt B5）

> 開発エージェント（aidlc-developer-agent）の作業報告。ブランチ `bolt/b5-u3-event-store-repository`。
> 出典: `developer-brief-1.md`、`code-generation-plan.md` §5.1 Step 1〜2、`../functional-design/{rules,entities,functional-spec}.md`（BR3.1 / BR3.2 / BR4.1 / BR4.2 / BR4.3）、
> `../nfr-design/security-design.md` §6、`unit-test-instructions.md`。

## 基線（着手時、`bolt/b5-u3-event-store-repository` HEAD）

- `cargo test --workspace`: **471 passed / 0 failed**
- `cargo test --manifest-path tools/lint/Cargo.toml`: **31 passed / 0 failed**

## §A 退役（作業 A、計画 Step 1）

### A-1. 削除したファイル（17 ファイル）

| 層 / 区分 | パス | 備考 |
|---|---|---|
| use-case | `modules/core/use-case/src/workspace/mod.rs` | mod ごと削除（104 行の `workspace_lock.rs` と併せて dir 消滅） |
| use-case | `modules/core/use-case/src/workspace/workspace_lock.rs` | `WorkspaceLock` / `AcquireBudget` / `LockGuard` / `AcquireError` |
| adapter | `modules/core/interface-adapter/src/workspace/fs_workspace_lock.rs` | 810 行・インライン 14 テスト |
| adapter | `modules/core/interface-adapter/src/process_probe.rs` | 87 行・インライン 2 テスト |
| adapter tests | `modules/core/interface-adapter/tests/fs_workspace_lock_test.rs` | 220 行・6 テスト |
| domain | `modules/core/domain/src/workspace/lock_protocol.rs` | 600 行・インライン 8 テスト（`reap_eligible` / `LockProtocol` / `LockError`） |
| domain | `modules/core/domain/src/workspace/lock_identity.rs` | 74 行・インライン 3 テスト（`LockIdentity` / `WORKSPACE_LOCK_SENTINEL`） |
| domain tests | `modules/core/domain/tests/audit_lock_conformance.rs` | 148 行・1 テスト（fixture 7 本を駆動） |
| infra-io | `modules/infra-io/src/process_probe.rs` | 62 行・インライン 3 テスト（`process_alive`） |
| formal | `formal/workspace/audit_lock.qnt` | `formal/workspace/` が空になったため dir ごと削除 |
| fixtures | `tests/conformance/fixtures/audit_lock/`（7 ファイル） | dir ごと削除 |

`modules/core/interface-adapter/src/workspace/state_file_io.rs` は**維持**（ブリーフ指示どおり）。

### A-2. 編集したファイル（ファサード・機構・依存・スクリプト・正本）

| ファイル | 変更 |
|---|---|
| `modules/core/use-case/src/lib.rs` | `pub mod workspace;` を削除 |
| `modules/core/domain/src/workspace/mod.rs` | `mod lock_identity` / `mod lock_protocol` と `pub use`（`LockIdentity` / `LockProtocol` / `reap_eligible` / `LockError` / `WORKSPACE_LOCK_SENTINEL`）を削除 |
| `modules/core/interface-adapter/src/lib.rs` | `mod process_probe;` と `pub use process_probe::{FakeProcessProbe, OsProcessProbe, ProcessProbe};` を削除。冒頭 rustdoc の機構モジュール列挙を `clock` のみに |
| `modules/core/interface-adapter/src/workspace/mod.rs` | `mod fs_workspace_lock` と `pub use FsWorkspaceLock` / `DEFAULT_LOCK_STALE_MS` / `DEFAULT_UNSTAMPED_GRACE_MS` を削除。rustdoc を現状（公開型なし・`state_file_io` は内部部品）に更新 |
| `modules/core/interface-adapter/src/clock.rs` | rustdoc の存在理由を `FsWorkspaceLock` の stale 判定 → 時刻依存 Gateway 挙動の決定的検証へ更新（`FsWorkspaceLock` は BR3.1 の grep 語） |
| `modules/core/interface-adapter/src/workspace/state_file_io.rs` | rustdoc の「利用制約」から audit ロック区間の前提を除去し、直列化の担い手を SQLite 書込トランザクション + 楽観バージョン（ADR-007）へ更新 |
| `modules/infra-io/src/lib.rs` | `pub mod process_probe;` を削除。rustdoc の「reap 政策」文言を除去 |
| `modules/core/interface-adapter/Cargo.toml` | `md5 = "0.8"` を削除（`md5` の他利用は grep で 0 件。`definition_revision.rs` の `"md5:abcd"` は `DefinitionRevision::parse` のテスト用リテラルであり crate 利用ではない） |
| `tools/lint/src/check.rs` | `reap-decision-locality` を全面削除（`RULE_REAP_DECISION_LOCALITY` / `REAP_HELP` / `REAP_IDENTS` / `INTERFACE_ADAPTER_ROOT` / `reap_rule` フィールド / `push_reap` / `visit_expr_binary` / `mentions_reap_state` / `IdentSearch` / `is_ordering_op`、および R2 赤例 4 本・緑例 2 本）。`checkbox-vocabulary` / `no-public-fields` は維持 |
| `tools/lint/src/check.rs`（テスト定数） | `ADAPTER_PATH` / `ADAPTER_TEST_PATH` が削除済みファイルを指していたため、実在する `orchestration/workflow_definition_repository_impl.rs` と同名の統合テストへ再指定（R3 テストが引き続き使用するため） |
| `scripts/quint-gate.sh` | `AUDIT_LOCK` 変数、typecheck ループの該当モデル、`invariants run: audit_lock`、witness ループ（`w_threshold_reap` 〜 `w_recovery_after_mid_txn_crash` の 7 本）を除去。冒頭コメントの typecheck 列挙を `(engine_loop / stop_hook)` に更新（`journal_protocol` の追加は委任 4） |
| `scripts/coverage.sh` | `TOLERANCE=0.05` → `0.01`、直前コメントを「U3 のロック退役（ADR-007、Bolt B5）でジッタ源（並行ロックテスト）が消えたため 0.01 に引き締めた（team.md Testing Posture）」に置換 |
| `coding-rules/tell-dont-ask.md` | 適用例と本文の `reap_eligible` に「（退役済み — ADR-007 / Bolt B5。以後は履歴としての例）」を注記。規範の文は `CheckboxState` 分類述語（`checkbox-vocabulary`）の例で自立するよう書き換え。機械強制欄から `reap-decision-locality` を除去 |
| `coding-rules/README.md` | tell-dont-ask 行の機械強制を `cargo lint`（checkbox-vocabulary）に |
| `coding-rules/gateway-taxonomy.md` | §1 の機構モジュール例を `core_interface_adapter::{clock, process_probe}` → `{clock}` に。『適用の帰結』表の `FsWorkspaceLock` 言及は旧列（履歴）なので保持 |

`tools/lint/src/main.rs` は `reap` への言及・登録が元から無く（grep 0 件）、変更不要だった。

### A-3. 検査結果

```
$ cargo build --workspace
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.53s

$ cargo test --workspace
TOTAL PASSED: 434   (0 failed)          # 基線 471 → 434（−37）

$ cargo test --manifest-path tools/lint/Cargo.toml
test result: ok. 25 passed; 0 failed    # 基線 31 → 25（−6 = R2 赤例 4 + 緑例 2）

$ cargo clippy --manifest-path tools/lint/Cargo.toml --all-targets -- -D warnings
    Finished `dev` profile                # 警告 0

$ cargo fmt --all --check
（出力なし・exit 0）

$ cargo clippy --workspace --all-targets -- -D warnings
（error / warning 行 0・exit 0）

$ cargo lint
（出力なし・exit 0）

$ bash scripts/quint-gate.sh
[PASS] quint gate: all steps green      # 10 ステップ（typecheck 2 / invariants 2 / witness 5 / 決定的シナリオ 1）
```

**テスト数の差分の説明（−37、全数一致）**: `fs_workspace_lock.rs` 14 + `fs_workspace_lock_test.rs` 6 + `lock_protocol.rs` 8 + `lock_identity.rs` 3 +
`infra-io/process_probe.rs` 3 + `adapter/process_probe.rs` 2 + `audit_lock_conformance.rs` 1 = 37。`workspace_lock.rs` はインラインテスト 0 本。
退役以外のテストは 1 本も落ちていない。

**BR3.1 の grep（合格条件そのもの）**

```
$ grep -rnE 'WorkspaceLock|FsWorkspaceLock|LockProtocol|LockIdentity|reap_eligible|OwnerStamp|AcquireBudget|LockGuard|LockError|process_alive|ProcessProbe|audit_lock|reap-decision-locality' \
    modules tools scripts formal .github Cargo.toml   # tools/lint/target を除外
（出力なし = 0 件）

$ grep -rn 'aidlc-lock' modules tools scripts
（出力なし = 0 件）
```

後方互換の残置（型エイリアス・deprecated・feature flag・空の旧 mod）は作っていない。

## §B U2 是正（作業 B、計画 Step 2）

TDD で Red → Green → Refactor の順に進めた。

### B-1. Red（失敗を先に固定した）

`IntentId` のテストを UUIDv7 形式へ全面的に書き換え、`IntentDirName` はテストモジュールだけの
ファイル（実装なし）＋ `workspace/mod.rs` のファサード宣言を先に置いた。

```
$ cargo test -p core-domain
error[E0432]: unresolved import `intent_dir_name::IntentDirName`
error[E0432]: unresolved import `intent_dir_name::IntentDirNameError`
error[E0599]: no variant named `Length` found for enum `intent_id::IntentIdError`   （× 5）
error[E0599]: no variant named `Format` found for enum `intent_id::IntentIdError`   （× 6）
error[E0599]: no variant named `Version` found for enum `intent_id::IntentIdError`  （× 3）
error[E0599]: no variant named `Variant` found for enum `intent_id::IntentIdError`  （× 3）
error[E0433]: cannot find type `IntentDirName` in this scope                        （× 8）
error[E0433]: cannot find type `IntentDirNameError` in this scope
error: could not compile `core-domain` (lib test) due to 36 previous errors
```

続いて `IntentId` だけを実装した中間段階では、既存の kebab リテラルを使うテストが実行時に落ち、
置換対象が機械的に洗い出された（Red の 2 段目）。

```
---- orchestration::workflow_execution::tests::… （15 件）
thread '…' panicked at modules/core/domain/src/orchestration/workflow_execution.rs:1159:51
---- orchestration::workflow_execution_state::tests::the_identity_attributes_are_carried_verbatim
assertion `left == right` failed
```

### B-2. Green

**`IntentId`（BR4.1、`modules/core/domain/src/orchestration/intent_id.rs`）**

- 受理形を UUIDv7 の正準表記に限定した — 小文字 36 字、`-` は 0 始まり位置 8 / 13 / 18 / 23、
  version nibble（位置 14 = 16 進 13 桁目）は `7`、variant nibble（位置 19 = 16 進 17 桁目）は
  `8` / `9` / `a` / `b`（RFC の `10xx`）。kebab の受理は廃止。
- `IntentIdError` を `Empty` / `Length { actual }` / `Format { position }` / `Version { found }` /
  `Variant { found }` の 5 変種に置き換えた（材料のみ・手実装 `Display` + `Error` —
  `coding-rules/error-handling.md`）。
- 標準ライブラリのみ。正規表現クレートは足していない。添字アクセスを使わず `chars().enumerate()`
  の 1 パスで検査するので `indexing_slicing` を生まない。`unwrap` / `expect` / `panic!` なし。
- 検査順は「左から最初の違反が勝つ」— Empty → Length → 位置ごとの Format → その位置の
  Version / Variant。前後の空白の trim は**既存挙動を維持**した（BR4.1 は形式だけを変える規則で
  あり、既存テスト `surrounding_whitespace_is_trimmed_before_validation` を落とす理由がない）。
- テスト 8 本 → **13 本**（受理 4 種の variant 網羅、空、長さ 35 / 37、kebab 拒否、大文字、
  ハイフン位置違い 2 種、非 16 進、version 2 種、variant 2 種、辞書順 = 作成順、Map/Set キー、
  Display の材料）。

**`IntentDirName`（BR4.2、`modules/core/domain/src/workspace/intent_dir_name.rs` — 新規）**

- `<YYMMDD>-<slug>` の kebab 表記、全体 64 字以下。`IntentDirNameError` は
  `Empty` / `Length { actual }` / `Format { position }` / `EmptySegment { position }`（材料のみ、
  手実装 `Display` + `Error`）。
- 正規化は一切しない（`SpaceName` と同じ方針 — パスセグメントを型で保証するのが役目）。
  予約ラベルの拒否は birth（U7）の責務として実装しない（BR4.2 のとおり）。
- `workspace/mod.rs` のファサードに `pub use intent_dir_name::{IntentDirName, IntentDirNameError};`
  を追加（module-visibility の私有 mod + 選択的 `pub use`）。テスト **9 本**。
- **連続ハイフンは拒否**した。FD の正規表現との差異は「設計質問 1」に記載。

### B-3. Refactor（改名、BR4.3）

`git mv` でファイルを移し、型・メソッド・モジュールパス・rustdoc を一括で改名した。旧名の
再エクスポート・型エイリアス・deprecated は残していない。

| 旧 | 新 |
|---|---|
| `orchestration/workflow_execution_snapshot.rs` | `orchestration/workflow_execution_state.rs` |
| `orchestration/snapshot_error.rs` | `orchestration/state_error.rs` |
| `WorkflowExecutionSnapshot` | `WorkflowExecutionState` |
| `WorkflowExecutionSnapshotBuilder` | `WorkflowExecutionStateBuilder` |
| `SnapshotError` | `StateError` |
| `WorkflowExecution::snapshot()` | `WorkflowExecution::state()` |
| `WorkflowExecution::from_snapshot()` | `WorkflowExecution::from_state()` |
| ビルダー内部フィールド `snapshot` / ローカル変数・テスト名の `snapshot` | `state` |
| rustdoc の「スナップショット」 | 「状態の写し (memento)」 |

`orchestration/mod.rs` の `mod` 宣言と `pub use` を新名で並べ直した（アルファベット順を維持）。
C6 の `snapshot` テーブルはアダプタ層の用語なのでドメイン層には持ち込んでいない。
`workflow_definition/stage_node.rs` の「スナップショット」は センサー適用宣言の逐語写しを指す別語義
のため（BR4.3 の grep 範囲外）そのままにした。

**IntentId リテラルの置換（5 か所）**

| ファイル | 旧 | 新 |
|---|---|---|
| `orchestration/workflow_execution.rs:1159` | `260822-stage1-selfhost` | `01a02785-1bd8-76eb-aeea-5aa303ebd5b6`（intents.json 実データ） |
| `orchestration/workflow_execution_event.rs:556` | `260822-stage1-selfhost` | 同上 |
| `orchestration/workflow_execution_state.rs:310` / `:344` | `260822-stage1-selfhost` | 同上 |
| `orchestration/workflow_execution_state.rs:402` | `u2` | `018f3b2c-4d5e-7f60-8abc-def012345678` |
| `tests/engine_loop_conformance.rs:236` | `itf-engine-loop` | `0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000` |

### B-4. 検査結果

```
$ grep -rn 'Snapshot' modules/core/domain/src/orchestration     # BR4.3
（出力なし = 0 件）
$ grep -rn 'snapshot' modules/core/domain/src/orchestration     # 小文字も 0 件
（出力なし = 0 件）

$ cargo test --workspace
TOTAL PASSED: 448   (0 failed)          # §A 後 434 → 448（+14 = IntentId +5 / IntentDirName +9）
  core-domain lib 247 / engine_loop_conformance 1 / core-interface-adapter lib 29 /
  golden_parity 9 / workflow_definition_repository_impl 27 / canon-json 87 / ほか

$ cargo fmt --all --check
（出力なし・exit 0）

$ cargo clippy --workspace --all-targets -- -D warnings
（error / warning 行 0・exit 0）

$ cargo lint
（出力なし・exit 0）

$ cargo test --manifest-path tools/lint/Cargo.toml
test result: ok. 25 passed; 0 failed

$ bash scripts/quint-gate.sh
[PASS] quint gate: all steps green

$ bash scripts/coverage.sh
head line coverage: 97.174993460633%
[PASS] absolute gate: head (97.174993460633%) >= threshold (90.0%)
```

BR3.1 の grep（§A）も改名後に再実行して 0 件を再確認した。

## 設計質問

1. **BR4.2 の正規表現が連続ハイフンを受理してしまう（実装は拒否側を採った）**
   FD `rules.md` BR4.2 の `^[0-9]{6}-[a-z0-9](?:[a-z0-9-]*[a-z0-9])?$` は、中間の
   `[a-z0-9-]*` により `260822-a--b` を**受理**する。一方ブリーフの拒否リストは「連続ハイフン」を
   明示的に拒否と指定しており、正本 `docs/specs/11-workspace.md` §2.2 も「`<YYMMDD>-<slug(label,24)>`
   の **kebab 表記**」と書いている。退役前の `IntentId`（kebab 版）も `--` を `EmptySegment` で
   拒否していた。三者のうち二者（ブリーフ・正本・既存の家内規約）が一致するため、**連続ハイフンを
   拒否する読み**（`^[0-9]{6}-[a-z0-9]+(?:-[a-z0-9]+)*$` 相当）で実装した。FD の正規表現を
   この形に是正するのが妥当と考えるが、裁定を求める。実装を変える場合は
   `a_segment_may_not_be_empty` テストの `--` ケースを外すだけで済む。

2. **`IntentDirName` は正規化しない（前後空白を trim しない）ことにした**
   `IntentId` は既存挙動どおり trim を維持したが、`IntentDirName` はパスセグメントであり、
   `SpaceName`（「正規化は一切しない — 受理か拒否のみ」）の先例に倣って trim しない設計にした。
   両者で扱いが違うことの是非について裁定が要るなら知らせてほしい。

3. **`Cargo.lock` が所有ファイル外だが変更された**
   `modules/core/interface-adapter/Cargo.toml` から `md5` を除いた機械的帰結として、`cargo build`
   が `Cargo.lock` から `md5 0.8.1` の 2 か所を落とした（差分はその 2 か所のみ、確認済み）。
   計画では `Cargo.lock` は委任 3 の所有だが、独自判断の編集ではなくビルドの副作用である。

4. **`modules/shared/message-catalog/src/lib.rs` に退役済み型への参照が残っている（所有外）**
   60 行目の doc コメントが `AcquireError::Exhausted` に言及している。`message_catalog::lock`
   モジュール自体は upstream 逐語文言の写しなので**残すのが正しい**と判断したが、doc コメントの
   型参照だけは退役済みで宙に浮く。BR3.1 の grep 語には含まれないため合格条件は満たすが、
   同期先として拾うか裁定が要る。

## 未了

- **coding-rules の 3 ファイルが同期の割り当てから漏れている（所有外）**。BR5.1 と本ブリーフは
  `tell-dont-ask.md` / `README.md` / `gateway-taxonomy.md` の 3 本だけを名指ししているが、実測で
  次の 3 か所にも退役済みの記述が残っている。いずれも BR3.1 の grep 範囲外（`modules tools scripts
  formal .github Cargo.toml`）なので合格条件は満たすが、BR5.1 の「coding-rules を加えて 0 件
  （履歴注記を除く）」を厳密に読むなら履歴注記が要る:
  - `use-case-rules.md:32` — 「実証例: `reap_eligible`」（ユースケース間呼出禁止の根拠として引用）
  - `module-visibility.md:12` — `infra_io::{atomic, append_only, fs_meta, process_probe}`（`process_probe` は削除済み）
  - `domain-equality.md:4` — 適用例 `OwnerStamp`（退役済み型）
- `scripts/quint-gate.sh` への `journal_protocol` ステップ追加は**委任 4** の担当（ブリーフの指示
  どおり、ここでは audit_lock の除去だけを行い増やしていない）。
- `scripts/coverage.sh` の**相対ゲート**（`--base <ref>`）は未実行。絶対ゲート（97.17% ≥ 90%）
  のみローカルで確認した。相対ゲートは base の採取が要るためコンダクタ / CI 側に委ねる。
- `git add` / `git commit` は行っていない（コンダクタが「退役」「是正」の 2 コミットに区切る）。
