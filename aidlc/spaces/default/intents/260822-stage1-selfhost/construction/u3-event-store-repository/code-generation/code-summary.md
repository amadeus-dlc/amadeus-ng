# code-summary — U3 イベントストアと実行 Repository（`u3-event-store-repository`、Bolt B5）

> Code Generation（Construction 3.5）の完了報告。出典: `code-generation-plan.md`、`unit-test-instructions.md`、`developer-report-1..7.md`、
> `../../code-generation/memory.md`（裁定の記録）。ブランチ `bolt/b5-u3-event-store-repository`、起点 `origin/main`（db6c0a1）。

## 1. 結果

| 項目 | 実測 |
| --- | --- |
| `cargo test --workspace` | **664 passed / 0 failed**（基線 471 → 委任 1 で 448（退役 −37・是正 +14）→ 549 → 623 → 664） |
| `cargo test --manifest-path tools/lint/Cargo.toml` | 25 passed / 0 failed（ルール削除で 31 → 25） |
| `cargo fmt --all --check` | exit 0（出力なし） |
| `cargo clippy --workspace --all-targets -- -D warnings` | 警告 0、exit 0 |
| `cargo lint` | exit 0（出力なし） |
| `bash scripts/quint-gate.sh` | `[PASS] quint gate: all steps green`（invariants 3 モデル + witness 12 本 + 決定的シナリオ） |
| `bash scripts/coverage.sh --base origin/main` | `[PASS] absolute gate` **98.42%** ≥ 90.0%、`[PASS] relative gate` head 98.42% ≥ base 97.39% − 0.01 |
| `cargo audit` / `cargo audit --file tools/lint/Cargo.lock` | 脆弱性 **0 件**（advisory DB 1225 件、100 crates / 5 crates） |
| 退役語 grep（`WorkspaceLock` ほか 13 語 / `aidlc-lock`） | **出力 0**（`modules tools scripts formal .github Cargo.toml`） |
| `Snapshot` grep（`modules/core/domain/src/orchestration`） | **出力 0**（`WorkflowExecutionState` へ改名済み、BR4.3） |

受入 BR5.2 は全項目 PASS。委任 7 の時点で赤だった相対ゲート（head 96.81% < base 97.39%）は、退役で消えたテスト 37 本の分の
分母比と新規アダプタコードのエラー経路未カバーが原因だった。テスト 41 本を足して head 98.42%（+1.03pt）で回復した。

## 2. 作成・変更ファイル（`git diff --stat origin/main..HEAD`、99 ファイル / +10,042 / −2,890）

### 2.1 退役（削除、委任 1）— ADR-007「ロック退役」

`formal/workspace/audit_lock.qnt`(−366) と ITF fixture 7 本、`modules/core/domain/src/workspace/lock_protocol.rs`(−600) /
`lock_identity.rs`(−74)、`modules/core/interface-adapter/src/workspace/fs_workspace_lock.rs`(−810) / `process_probe.rs`(−87)、
`modules/core/use-case/src/workspace/workspace_lock.rs`(−104)、`modules/infra-io/src/process_probe.rs`(−62)、
`modules/core/domain/tests/audit_lock_conformance.rs`(−148)、`tests/fs_workspace_lock_test.rs`(−220) ほか計 17 ファイル。
ファサード・`scripts/quint-gate.sh`・仕様正本も同期。

### 2.2 U2 是正（委任 1）— BR4.1〜4.3

`intent_id.rs`（kebab → UUIDv7、+253/−…）、`workspace/intent_dir_name.rs`（新規 +246、`<slug>-<id8>` の綴り規則）、
`workflow_execution_snapshot.rs` → `workflow_execution_state.rs`、`snapshot_error.rs` → `state_error.rs`、
`workflow_execution.rs`（+183）。

### 2.3 use-case 層（新規、委任 2）— ポート・値・エラー

`orchestration/event_store.rs`(+230) / `event_store_error.rs`(+270) / `journal_reader.rs`(+210) /
`workflow_execution_repository.rs`(+171) / `repository_error.rs`(+307) / `global_seq_nr.rs`(+90) / `projection_name.rs`(+196)。

### 2.4 アダプタ層（新規、委任 2・3）

- ワイヤ: `wire/event_wire.rs`(+835) / `wire/state_wire.rs`(+568) / `wire/mod.rs`(+498)
- InMemory: `memory/in_memory_event_store.rs`(+703) / `memory/workflow_execution_repository.rs`(+140)
- SQLite: `event_store_impl.rs`(+971) / `schema.rs`(+179) / `store_path.rs`(+91) / `workflow_execution_repository_impl.rs`(+149)

### 2.5 形式検証（新規、委任 4）

`formal/orchestration/journal_protocol.qnt`(+254)、ITF fixture 8 本（`tests/conformance/fixtures/journal_protocol/`）、
`tests/journal_protocol_conformance.rs`(+474)、`scripts/quint-gate.sh`(+35/−…)。

### 2.6 テスト

`tests/event_store_impl_test.rs`(+1,008) / `workflow_execution_repository_impl_test.rs`(+434) /
`workflow_execution_repository_contract.rs`(+145) / `support/contract.rs`(+307) / `support/mod.rs`(+108) /
`crash_reconstruction_test.rs`(+202) / `in_memory_workflow_execution_repository_test.rs`(+200、委任 7)。

### 2.7 lint 昇格（委任 6）と仕様同期（委任 5）

`Cargo.toml`（`indexing_slicing` / `panic` を workspace deny へ昇格）と src 5 ファイル 8 箇所の是正、
`docs/specs/01-domain-model.md` / `10-orchestration.md` / `11-workspace.md` / `deviations.md`。

## 3. TDD の記録（各 Red の失敗出力 — 詳細は developer-report-1 §B-1 / -2 §A / -3 §A）

| 委任 | Red | Green |
| --- | --- | --- |
| 1（U2 是正） | `cargo test -p core-domain` — `E0432 unresolved import IntentDirName` ほか **36 errors**。`IntentId` 実装後の 2 段目で kebab リテラルを使う既存テスト 15 件が実行時に落ち、置換対象が機械的に洗い出された | 448 全緑 |
| 2（ポート・ワイヤ・InMemory・契約） | レイヤーごとに 3 回。Step 3 は `cargo test -p core-use-case` の `E0432` **8 件**（`EventStore` / `JournalReader` / `WorkflowExecutionRepository` / `GlobalSeqNr` / `ProjectionName` / `CorruptCause` + `EventStoreError` / `ProjectionNameError` / `RepositoryError`）、Step 4 はワイヤ、Step 5 は契約テスト | 549 全緑 |
| 3（SQLite ストア + Repository 実装） | `cargo test -p core-interface-adapter` — 4 テストターゲットすべてが `E0432`（`SqliteEventStore` / `StorePath` / `WorkflowExecutionRepositoryImpl` が存在しない） | 623 全緑 |
| 7（カバレッジ回復） | 既存実装に対する追加テストのため Red-first は適用外（挙動の追加ではなく未踏経路の固定） | 664 全緑 |

Rust では新規型がコンパイルエラーになり `test result: FAILED` 行が出ないため、U1 で採用した「コンパイルエラーの実測出力を Red の証跡とする」
方式（memory.md 2026-08-22T13:35:00Z）を踏襲した。

## 4. 主要な実装判断（設計との差分 — 設計側の反映対象）

裁定はすべて `../../code-generation/memory.md` の U3 エントリに記録済み。FD pending 番号は `../functional-design/` の追補待ち項目。

| # | 差分 | 裁定 |
| --- | --- | --- |
| 1 | **`SqliteEventStore` → `EventStoreImpl` に改名**。設計（entities.md / functional-spec §1 / 計画 §2）は `SqliteEventStore` を明示していたが、coding-rules `gateway-taxonomy.md` §5「技術接頭辞禁止 — 格納形式は実装の内部詳細」に抵触し、`Sqlite` がファサードの公開 API に出ていた | 改名を採用（ADR-003 の語に合わせる）。仕様 10/11 号・`components.md` も同期（FD pending 7） |
| 2 | **`open_with_busy_timeout` を公開面に追加**。設計は `open(path, clock)` のみ。`busy_timeout` 超過の `Io(WouldBlock)` を現実的な時間で観測するには待たされる側の timeout を短くするしかない（既定 5000ms のままだと 1 本 5 秒） | 公開を受容。`open` は `DEFAULT_BUSY_TIMEOUT = 5000ms` に委譲するので BR2.1 は満たす（FD pending 5） |
| 3 | **`UPDATE snapshot` の SET に `schema_version` を含めた**。BR2.3 の SET 一覧には無い | 含める。現状 `SCHEMA_VERSION` は 1 固定で観測差ゼロだが、将来版を上げたとき「payload は新版・列は旧版」の静かな破損経路が残るため（FD pending 6） |
| 4 | **スナップショット payload の `version` は新 version に揃える**。委任 2 の設計質問 4 | 新 version に統一（InMemory / SQLite 双方、FD pending 4） |
| 5 | **`persist_event` に version 検査を入れた**。BR2.3 の逐語は「(1) のみ」 | 両実装が同じ意味論であることを記録に残す（FD pending 8） |
| 6 | **`phase_boundary` の入れ子**（委任 2 の設計質問 2） | FD pending 3 |
| 7 | ~~**`EventStoreImpl` を `Rc<RefCell<Connection>>` の共有ハンドル**にする**~~ → **撤回**。`EventStoreImpl` は `Connection` を**直接所有**し、内部可変性も手書き `Clone` も持たない。書込は `&mut self`、読取は `&self` | **2026-08-23 撤回・差替（オーナー裁定）**。当初の採用理由は「`await` を跨いで `RefCell` の借用を持たない」ことだったが、`&mut self` で設計すればその問題は最初から生じない。`&self` の裏に可変性を隠すのは「`&self` への偽装」であり禁止 — 正本 `coding-rules/interior-mutability.md` / `command-query-separation.md`。委任 8 で是正（`developer-report-8.md`）|
| 8 | **`from_event_store` の写像**（Repository が EventStore の観測を `RepositoryError` へ写す規則） | 採用 |
| 9 | **委任 1 は 2 コミット分割を諦めて 1 コミット**（`mod.rs` の依存で中間状態がビルド不能） | 受容 |
| 10 | **`ErrorCode` → `ErrorKind` の写像**を明示（`DatabaseBusy`/`DatabaseLocked` → `WouldBlock`、`CannotOpen`/`NotFound` → `NotFound`、`PermissionDenied`/`ReadOnly`/`AuthorizationForStatementDenied` → `PermissionDenied`、`DatabaseCorrupt`/`NotADatabase` → `InvalidData`、`OperationInterrupted` → `Interrupted`、他 → `Other`） | 採用（`DiskFull` に対応する安定 `ErrorKind` が無いため `Other`） |
| 11 | **ISO 8601 整形は自前の純関数**（`chrono` / `time` を足さない、NFR4.1 依存最小化） | 採用 |
| 12 | **`journal_mode` は既定（`delete`）のまま**。WAL は `-wal` / `-shm` を増やし逸脱台帳 #4 のパスが 1 本で済まなくなる | 採用（BR2.1 どおり） |

## 5. テスト

- **単体**（インライン `#[cfg(test)]`）: ワイヤの型拒否・エラー `Display`・写像・整形の純関数など。
- **契約テスト**（`workflow_execution_repository_contract.rs` + `support/contract.rs`）: **12 本**を InMemory / SQLite の
  両実装に対して同一に走らせ、片方だけ通る経路を残さない（BR2.7）。実装に破壊用フックを開けない方針（BR2.8）は維持。
- **SQLite 固有**: 27 本（スキーマ刻印、Tx 手順、楽観 version、`busy_timeout`、Io 写像）+ `Impl` 10 本 + **クラッシュ再構成 5 本**。
- **PBT**（proptest 1.11.0）: ワイヤの往復。`PROPTEST_RNG_SEED` 固定で決定的。
- **ITF 準拠**（`journal_protocol_conformance.rs`）: Quint トレース 8 本を集約に再生して状態射影を突合。
- **カバレッジ**: 絶対 98.42%（床 90.0%）、相対 +1.03pt。除外は追加していない（adapter に除外なし）。

### 5.1 Quint モデル `journal_protocol.qnt` の検査力（ADR 0003 決定 7 の DoD）

named invariant 8 本それぞれに変異モデルを一時ディレクトリで作り、その invariant **単独**の
`quint run --seed 0x5e1 --max-samples 5000 --max-steps 50 --invariant <inv>` が violation を出すことを確認した — **8/8 検出**。

| # | invariant | 変異 | 結果 |
| --- | --- | --- | --- |
| m1 | `conflict_rejected` | `store_conflict` がジャーナルに 1 行書く（rollback 漏れ） | DETECTED |
| m2 | `snapshot_tracks_journal` | `store_ok` がスナップショットの `seq_nr` を進め忘れる | DETECTED |
| m3 | `version_equals_journal` | `store_ok` が楽観 version を進め忘れる | DETECTED |
| m4 | `checkpoint_monotone` | `catchup` がチェックポイントを 1 つ後退させる | DETECTED |
| m5 | `checkpoint_bounded` | `catchup` がチェックポイントをジャーナルより先へ進める | DETECTED |
| m6 | `projection_idempotent` | `catchup` が読むものが無くても投影を 1 つ進める | DETECTED |
| m7 | `truth_is_journal` | `catchup` が投影をジャーナルより 1 つ先へ進める | DETECTED |
| m8 | `no_lost_update` | `store_ok` の楽観 version ガードを外す | DETECTED |

witness 4 本（`w_conflict` / `w_crash_then_catchup` / `w_interleaved_writers` / `w_idempotent_catchup`）は負形式 run で経路実在を確認。

### 5.2 ITF fixture（8 本、`tests/conformance/fixtures/journal_protocol/`）

| fixture | 状態数 | load | store_ok | store_conflict | catchup | crash | idle |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `trace-0xa1` | 41 | 5 | 7 | 3 | 6 | 10 | 9 |
| `trace-0xb2` | 41 | 8 | 8 | 3 | 7 | 8 | 6 |
| `trace-0xc3` | 41 | 6 | 8 | 1 | 7 | 10 | 8 |
| `trace-0xd4` | 41 | 8 | 8 | 1 | 8 | 8 | 7 |
| `trace-0xe5` | 41 | 11 | 5 | 0 | 8 | 11 | 5 |
| `trace-0xf6` | 41 | 14 | 4 | 0 | 7 | 9 | 6 |
| `trace-0x101` | 41 | 9 | 8 | 2 | 8 | 8 | 5 |
| `trace-0x202` | 41 | 6 | 8 | 3 | 8 | 9 | 6 |
| **合計** | | 67 | 56 | 13 | 59 | 73 | 52 |

**6 本以上**（8 本）・**全 6 アクション網羅**。準拠テストが全アクション網羅を assert しているので、稀アクションを含む fixture の
消失退行は CI で落ちる。8 本すべてモデルから再採取 → 正規化した内容がコミット済みファイルとバイト一致することを確認済み。

### 5.3 lint 昇格の是正（委任 6）

`clippy::indexing_slicing` / `clippy::panic` を `[workspace.lints.clippy]` の deny へ昇格。プロダクトコードは
**5 ファイル 8 箇所**を是正（`canon-json/{canonical,digest,writer}.rs` 各 2、`infra-io/append_only.rs` 1、
`domain/workspace/state_writers.rs` 1）。いずれも `unwrap`/`expect`/`panic!` を使わず `.get()` + `if let` / `let-else` /
安全な既定値（到達しない旨のコメント付き）で処理し、canon-json はゴールデン（バイト互換）緑を確認。
テスト側は **19 スコープ**の file/mod 単位 `#![allow(...)]` に理由コメントを添えた。

### 5.4 カバレッジの残り（委任 7、到達不能 12 行）

`workflow_definition_repository_impl.rs` 563（`serialize_grid` の else — `scope_names()` は同じ `columns` の鍵集合）、
596-597 / 599-602（`compute_revision` の `map_err` 2 本 — 入力に失敗経路が無く `sha256:<hex64>` は必ず parse 可）、
749（非 UTF-8 ファイル名 — macOS/APFS は作成自体を拒否）、`wire/event_wire.rs` 425（`EVENT_TYPES.contains` 通過後の `_` 腕）、
510 と `wire/state_wire.rs` 257 / 273（テストヘルパ自身の `panic!` 分岐）。
それ以外の 8 ファイルは**未カバー 0 行**。

## 6. 計画からの逸脱

1. **委任 1 を 2 コミットに分けられなかった**（退役 → U2 是正）。`workspace/mod.rs` の `pub use` 依存で中間状態がビルド不能になるため 1 コミットにした。
2. **委任 3・4・5 を並行ディスパッチした**（計画は直列）。書込スコープが重ならない（実装 / formal / docs）ことを確認したうえでの短縮。
3. **`SqliteEventStore` の改名は委任 3 完了後にコンダクタが統合として実施**（委任内の作業ではない）。
4. **委任 7（カバレッジ回復）は計画に無い追加委任**。相対ゲートの許容誤差を本 Bolt で 0.5pp → 0.01 に引き締めた結果として必要になった。
5. **委任 2 と 7 のあいだで park / 再開が 1 回入った**（コンテキスト都合、`handoff-b5.md`）。成果物への影響は無い。

## 7. 申し送り

1. **fixture 鮮度ゲートが未実装**（ADR 0003 決定 4）。今回は手作業で「再採取 → 正規化 → バイト一致」を確認したが、機械強制が無い。
   `engine_loop` / `journal_protocol` 双方に効く横断の穴として後続 Bolt へ。
2. **C3 の `usize` → `u64`**（`GlobalSeqNr` 周辺の桁幅）— 契約側の確定待ち。
3. **`within_write_transaction` が `rusqlite::Transaction` を公開面に露出させる**。設計どおりの署名だが、利用者（U7 の登録簿処理）が
   `rusqlite` を直接名指しすることになる。**U7 の設計時に再確認**。
4. **U4 の `reset_checkpoint`** — 投影のリセット口はまだポートに無い。U4（read model updater）で扱う。
5. **U5 の `Conflict` 再試行**方針（楽観 version 衝突時のリトライ回数・バックオフ）は U5 で決める。
6. **相対ゲートの許容誤差 0.01pp は base 側の実測ゆらぎより狭い可能性がある**。同じ `origin/main` を同じシードで 3 回計測して
   最大差 0.012pp（97.39995 / 97.38797 / 97.38797）。PBT のシードは固定済みなので、残るゆらぎ源は `busy_timeout` 超過や FS 待ちのような
   タイミング依存テストと推測される。今回は +1.03pt なので影響しないが、head と base が拮抗した Bolt では偽陽性の赤を出しうる。
   ゆらぎ源の特定を後続 intent へ申し送る。
7. **`cargo llvm-cov` は `src/**` のインライン `#[cfg(test)] mod tests` も計測対象に含む**。テストヘルパに未実行の分岐（`panic!` する else 腕）を
   作るとカバレッジを下げる副作用がある。`scripts/coverage.sh` の除外方針（composition root のみ）を見直すなら論点になる。
8. **`scope_file_paths` は名前だけを見てディレクトリも候補に入れる**。`aidlc-x.md` という名のディレクトリがあると `read_to_string` が失敗し
   `GraphReadError::ScopeFile` で致命になる（今回テストで固定）。upstream 側の態度は未確認（`load_scopes` の `TODO(spec: 12 §11)` と同性質）。
9. **`workflow_definition_repository_impl.rs:749`（非 UTF-8 ファイル名）は未カバーのまま**。CI（ubuntu）でだけ走る
   `#[cfg(target_os = "linux")]` テストを足す案はあるが、「ローカルで実行されないテスト」を増やす是非はオーナー裁定が要る。

## 8. 依存（版・`cargo audit`）

- 追加: `rusqlite 0.40.2`（`features = ["bundled"]`、`cargo search` 実測の最新安定版）→ `libsqlite3-sys 0.38.2`（同梱ビルド）。
- 既存: `tokio 1.53.1`（委任 2 が dev-dependency として追加済み）、`proptest 1.11.0`。
- ホストターゲットで実際にコンパイルされる推移依存（`cargo tree -e normal --target <host>` 実測）:
  `bitflags 2.13.1` / `fallible-iterator 0.3.0` / `fallible-streaming-iterator 0.1.9` / `hashlink 0.12.1` /
  `hashbrown 0.17.1`（既存）/ `libsqlite3-sys 0.38.2` / `smallvec 1.15.2`。ビルド依存は `cc` / `pkg-config` / `vcpkg` / `find-msvc-tools`。
- `Cargo.lock` に増える `thiserror` / `wasm-bindgen*` / `js-sys` / `sqlite-wasm-rs` / `rsqlite-vfs` は **wasm ターゲット専用**で
  本リポジトリのビルドには入らない（coding-rules の「thiserror 不使用」に抵触しない — 自前のエラー型は従来どおり手実装 enum）。
- `cargo audit`: **exit 0 / 0 件**（advisory DB 1225 件、workspace 100 crates）。`--file tools/lint/Cargo.lock` も 5 crates で **0 件**。

## 9. コミット（ブランチ `bolt/b5-u3-event-store-repository`、`origin/main` 起点）

| コミット | 内容 |
| --- | --- |
| `69f7f4e` | ロック系の退役 + U2 是正（委任 1） |
| `3f6db5a` | ポート・値・エラー（use-case）、ワイヤ、InMemory ストア / Repository、契約テスト（委任 2） |
| `8cfc36e` | 仕様・正本の同期（委任 5）— 10/11/01 号の不変条件表を journal_protocol へ、LockIdentity / ProcessProbe 退役、未決 2 件確定、deviations #4 パス確定 |
| `249d831` | 仕様の残注記を整理（State 改名完了の注記除去、責務記述のロック退役注記） |
| `6ce9e83` | journal_protocol.qnt（不変条件 8 / witness 4、mutation 8/8）+ ITF fixture 8 本 + conformance + quint-gate（委任 4） |
| `d96aa38` | SQLite ストア（C6 逐語、BEGIN IMMEDIATE、楽観 version、JournalReader、`within_write_transaction`）+ `WorkflowExecutionRepositoryImpl` + `StorePath` + rusqlite 0.40.2（委任 3） |
| `ac204a7` | 委任 4/5 の報告と裁定（aidlc 記録） |
| `48a0a2a` | 委任 3 の報告と裁定（`EventStoreImpl` 改名を予約） |
| `071f175` | clippy `indexing_slicing` / `panic` を workspace deny へ昇格し既存コードを是正（委任 6） |
| `93de46d` | `SqliteEventStore` → `EventStoreImpl` に改名（gateway-taxonomy §5 / ADR-003）— 仕様 10/11 号・`components.md` も同期 |
| `328efc9` | `journal_protocol.qnt` の冒頭コメントから退役語を除去（BR3.1 grep 0 件） |
| `dae1d3d` | カバレッジ穴のマップ、developer-brief-7、traceability.json |
| `989b2ae` | park（handoff-b5.md、委任 6 報告、状態） |
| `190dcb2` | カバレッジ相対ゲートの回復 — 未カバー 161 行中 149 行をテストで固定（委任 7） |

上表はコード・仕様のコミット（10 本）。ほかに aidlc 記録のコミットが 22 本あり、ブランチ全体では 32 コミット /
99 ファイル / +10,042 / −2,890（`git diff --stat origin/main..HEAD -- modules tests formal scripts .github Cargo.toml Cargo.lock docs`）。

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-08-23T12:21:52Z
**Iteration:** 1（advisory, unit: u3-event-store-repository）

### Findings

| # | Severity | Location | Finding | Recommendation |
|---|---|---|---|---|
| 1 | Major | `inception/contract-design/contract-summary.md:118,120`（`EventStore<AID, A, E>::persist_event` / `get_events_by_id_since_seq_nr` の数値パラメータ型）vs `modules/core/use-case/src/orchestration/event_store.rs:33,60`（実測） | 共有契約 C3（U5/U6 所有、U3 が準拠する側）は数値パラメータを `usize` で定義しているが、マージ済みコードは `u64` で実装している。`entities.md`/`rules.md` BR1.1/rustdoc（`event_store.rs` 冒頭）は「C3 改訂提案として所有者 U5/U6 へ申し送り」と明記して無言の変更にはしていないが、**`contract-summary.md` 自体は現時点でも `usize` のまま未改訂**である。このため今の単一の正本（C3）を読んで実装に着手する将来のユニット（U5/U6）は、字面どおりなら型不一致でコンパイルが通らない実装をしてしまうリスクが残る。 | U5/U6 着手前に、`contract-summary.md` の型を `u64` へ改訂するか、少なくとも「U3 が `u64` へ具体化・改訂待ち」の注記をC3本文に追記して、契約と実装の食い違いを次工程が読む前に解消する。 |
| 2 | Major | `modules/core/interface-adapter/src/orchestration/event_store_impl.rs:298-311`（`within_write_transaction`） | `pub fn within_write_transaction<T, F>(&mut self, f: F) where F: FnOnce(&Transaction<'_>) -> Result<T, EventStoreError>` は、SQLite 固有の `rusqlite::Transaction` 型をこの Gateway の公開 API シグネチャにそのまま露出している。`coding-rules/gateway-taxonomy.md`（格納形式はアダプタの内部詳細）の精神からは、将来この API を呼ぶ側（設計どおりの想定利用先は U7 の `intents.json` read-modify-write）は `rusqlite` に直接依存せざるを得なくなる、層境界の漏れである。`code-summary.md` §7 項目3 で自己申告済み・`pending-revision.md` #8 でも「U7 の設計で再確認」と申し送り済みであり隠れた欠陥ではないが、公開 API の形として既にコミットされてしまっているため、U7 側で気付いてからのシグネチャ変更は本 Unit の破壊的変更になる。 | U7 のキックオフ前に、クロージャ引数を独自の抽象型（例: 本 Unit が定義する薄いラッパー）でラップし直すか、現状のまま行くことを明示的に裁定する。 |
| 3 | Minor | `functional-design/entities.md:108-216`、`functional-spec.md:14,30-31`、`rules.md`（`SqliteEventStore`/`sqlite_event_store.rs` を使う各所） | 本 Unit 自身の functional-design 正本（entities/functional-spec/rules）は、コミット `93de46d` で実施済みの `SqliteEventStore` → `EventStoreImpl` 改名（`gateway-taxonomy.md` §5 準拠、`sqlite_event_store.rs` → `event_store_impl.rs`）を反映しておらず、旧名のまま残っている。`docs/specs/10-orchestration.md`/`11-workspace.md`/`components.md` は同期済み（`git diff` で確認）だが、U3 の FD 正本自体は未同期。`pending-revision.md` #7 が「entities / functional-spec / … の表記を同期（ステージゲートで処理）」として既に明記しており隠れた乖離ではない。 | ゲートで FD 正本を実コードの名前に同期するか、「歴史的記録として旧名のまま残す」ことを明示的に裁定する。 |
| 4 | Minor | `unit-test-instructions.md:23-37`（§2 ゲート） | code-generation ステージ規定は「本ファイルの実行コマンドはすべて本 Unit にスコープすること（`npm test` のようなプロジェクト全体コマンドは不可 — Build and Test が Unit ごとに再実行するため）」と定めているが、§2 には `cargo test --workspace` / `cargo clippy --workspace --all-targets` などプロジェクト全体コマンドがそのまま列挙されている。B5 は本 Unit 1 本のみの Bolt なので実害はないが、複数 Unit を含む将来の Bolt で同じ書式を踏襲すると、Build and Test がユニットの数だけ全体テストを再実行する非効率を生む。 | §2 を「Bolt 全体の受入ゲート（Build and Test は 1 回だけ実行）」と明示する注記を添えるか、§1 のみを Build and Test が消費する対象と明記する。 |

### 検証（上流 functional-design レビューの Critical 所見に対する独立確認・追加アクション不要）

`entities.md` 末尾の既存 `## Review`（functional-design 段、iteration 1、NOT-READY）は Critical 所見1として「BR1.3 の楽観 version 算出式（`aggregate.version() − 1`）が genesis で u64 アンダーフロー、以降は恒常的な偽陽性 Conflict になる」と指摘している。これをマージ済みコードに対して独立検証した:

- `modules/core/interface-adapter/src/orchestration/workflow_execution_repository_impl.rs:69-89`（`check_preconditions`）は `aggregate.version() != event.seq_nr() - 1` を**検査するだけ**（`aggregate.version()` 自体から 1 を引いてはいない）。
- `modules/core/interface-adapter/src/orchestration/event_store_impl.rs:508` は `let expected = aggregate.version();` — 引き算をしていない。
- genesis（`aggregate.version() == 0`）は `event_store_impl.rs:537` の `if expected == 0 { INSERT … }` 分岐に正しく回り、`UPDATE … WHERE version = expected` 分岐（アンダーフロー/偽陽性 Conflict の懸念があった経路）には入らない。

これは `rules.md` BR1.3 / `functional-spec.md` §3.1 の文言（「期待 version = `aggregate.version()`」— 引き算なし）どおりであり、実装に当該欠陥は再現しなかった。`pending-revision.md` #1 のとおり本文修正済み・レビュー予算切れで verdict 行のみ歴史的に NOT-READY が残っている状態と整合する。人間ゲートでは「この Critical 懸念は実装レベルで再現しないことを本レビューで確認済み」という前提で扱ってよい（FD 段の Major 所見2・3 は、上表所見1・2として本レビューでも独立に確認済みで未解消のまま残っている）。

### Validation Tool Results

| Tool / Check | Result | Interpretation |
|---|---|---|
| `cargo fmt --all --check` | exit 0（出力なし） | code-summary の実測と一致 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 警告 0、exit 0 | 一致 |
| `cargo lint` | exit 0（出力なし） | 一致 |
| `cargo test --workspace` | 664 passed / 0 failed（`test result:` 行を合計して独立集計） | 一致 |
| `cargo test --manifest-path tools/lint/Cargo.toml` | 25 passed / 0 failed | 一致 |
| `bash scripts/quint-gate.sh` | `[PASS] quint gate: all steps green`（journal_protocol の typecheck / invariants / witness 4 本を含む全 16 ステップ PASS） | 一致 |
| `bash scripts/coverage.sh --base origin/main`（2 回実測） | 1 回目: head 98.42091176732983% / base 97.38797028516655% → PASS。2 回目: head 同値 / base 97.39995207284927% → PASS | head は完全に再現。base は 2 回の実測で 0.012pp 変動し、code-summary §7 項目6 が申告する「base 実測ゆらぎ（3 回計測 97.39995/97.38797/97.38797%）」と符合 — 自己申告どおりの既知のゆらぎであり新規の懸念ではない |
| `cargo audit` / `cargo audit --file tools/lint/Cargo.lock` | 脆弱性 0 件（advisory DB 1225 件、100 crates / 5 crates） | 一致 |
| BR3.1/BR3.2 退役 grep（`WorkspaceLock` ほか 13 語 / `aidlc-lock`） | 出力 0 | 一致 |
| `bun .claude/tools/aidlc-sensor-traceability.ts --stage code-generation`（`traceability.json` 44 エントリ） | `"pass":false` だが `gaps:[]`/`orphans:[]`/`missing_from_table:[]`/`invalid_entries:[]`/`invalid_targets:[]`。`missing_from_upstream_ids` は U3 が担当しない FR1〜FR9/NFR1/2/4/5 系 40 件のみ | 実害となる項目はすべて空。全 44 件の `target` をファイルシステムに対しても独立に存在確認済み（python スクリプト、欠落 0） |
| `bun .claude/tools/aidlc-sensor-required-sections.ts`（`code-summary.md` / `unit-test-instructions.md`） | 両方とも `"pass":true`（H2 見出し 9 / 3 本） | 一致 |
| clean architecture 層依存（`core-use-case`/`core-domain` の `Cargo.toml`） | `rusqlite` は `core-interface-adapter` にのみ存在。use-case/domain には無し | ユースケースが SQLite を知らない、という設計どおりの層境界を確認 |
| 製品コードの `unwrap`/`expect`/`panic!` 不在（`event_store_impl.rs` を代表確認） | すべての `.expect(...)` は `mod tests`（772 行目以降）の内側のみ | ALWAYS ルール（プロダクトコードで unwrap/expect 禁止）順守を確認 |
| `EventStoreImpl` の内部可変性（`Rc<RefCell<Connection>>` 共有ハンドル、手動 `Clone`） | `event_store_impl.rs:204-219` で確認。`borrow().clone()` 後に await する形で借用が await をまたがない | code-summary 決定7 の記述どおり |
| （2026-08-23 追記: 上記行はレビュー時点の実態であり、その後の**オーナー裁定で決定 7 ごと撤回**された。`EventStoreImpl` は `Connection` を直接所有し、`Rc<RefCell<_>>` も手書き `Clone` も存在しない。本レビューの判定・所見本文は改変していない — この行は監査証跡として残す） | — | §4 決定 7 と §10 を参照 |
| エラーコード写像（`ErrorCode` → `std::io::ErrorKind`） | `event_store_impl.rs:102-116` が code-summary 決定10 の表と完全一致 | 一致 |

### Summary

Critical 所見は 0 件。上流 functional-design レビューが Critical と判定していた楽観 version 算出式の懸念は、マージ済みコードに対する独立検証の結果、実装には再現しないことを確認した（設計文書側の記述と実装は一致しており、当時のレビュー所見は設計文言の読み違いに基づくものと考えられる）。数値実測（`cargo test` 664/0、`quint-gate` 全緑、カバレッジ絶対/相対ゲート、`cargo audit` 0 件）はすべて独立に再現し、traceability.json の 44 エントリも全件ファイル実在を確認した。クリーンアーキテクチャの層境界（use-case が rusqlite を知らない）、退役対象の grep 0 件、product code の unwrap/expect/panic 不在も確認済み。残る懸念は Major 2 件（C3 契約の `usize`→`u64` の無断変更が共有契約側で未反映のまま残っている点、`within_write_transaction` が `rusqlite::Transaction` を公開面に露出している点）で、いずれも実装者自身が code-summary/pending-revision で開示済みであり本 Unit 単体の受入基準 BR5.2 を妨げるものではないが、次工程（U5/U6/U7）のキックオフ前に解消しておくべき事項として承認ゲートで重み付けされたい。
