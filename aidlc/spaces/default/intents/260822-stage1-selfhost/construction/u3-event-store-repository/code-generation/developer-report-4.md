# developer-report-4 — 委任 4: Quint モデル `journal_protocol.qnt` + ITF 準拠 + quint-gate（U3 / Bolt B5）

> 計画 Step 9〜11（`code-generation-plan.md` §5.4）の実施報告。FD BR3.3 / BR3.4 / BR3.5、ADR 0003 決定 4〜7 の DoD に対応する。
> 所有ファイルのみを変更した（`git add` / `git commit` は行っていない）。

## 成果物

| ファイル | 状態 |
|---|---|
| `formal/orchestration/journal_protocol.qnt` | 新規 |
| `tests/conformance/fixtures/journal_protocol/trace-0x{a1,b2,c3,d4,e5,f6,101,202}.itf.json` | 新規 8 本 |
| `modules/core/interface-adapter/tests/journal_protocol_conformance.rs` | 新規 |
| `scripts/quint-gate.sh` | 変更（journal_protocol のステップ追加のみ。既存 engine_loop / stop_hook のステップは未変更） |
| `formal/README*` | **該当なし**（リポジトリに存在しないため行追加は不要） |

## 1. モデル概要

定数 `WRITERS = 2`。抽象は「集約 1・writer 2・投影 1」。

**var（16 本）**: `journalLen` / `snapVersion` / `snapSeq` / `checkpoint` / `readModelSeq` /
`loadedVersion: int -> int` / `lastAction: str` / `lastActor: int` と、それぞれの前状態
スナップショット `prevJournalLen` / `prevSnapVersion` / `prevSnapSeq` / `prevCheckpoint` /
`prevReadModelSeq` / `prevLoadedVersion` / `prevLastAction` / `prevLastActor`
（engine_loop v2 / 旧 audit_lock v2 と同じ prev スナップショット方式。全アクションが
`all { snapshot, actX }` の形で `snapshot` と合成される）。

**action（6 本）**:

| アクション | ガード | 効果 |
|---|---|---|
| `load(w)` | なし | `loadedVersion[w] := snapVersion` |
| `store_ok(w)` | `loadedVersion[w] == snapVersion` | `journalLen+1`、`snapVersion+1`、`snapSeq := journalLen+1`、`loadedVersion[w] := snapVersion+1` |
| `store_conflict(w)` | `loadedVersion[w] != snapVersion` | 状態不変（`lastAction` のみ） |
| `catchup` | なし | `readModelSeq := journalLen`、`checkpoint := journalLen` |
| `crash` | なし | 状態不変（Tx 済み・投影未反映のマーカー） |
| `idle` | なし | 状態不変（スタッタ） |

**invariant（8 本、状態遷移レベル）**: `conflict_rejected` / `snapshot_tracks_journal` /
`version_equals_journal` / `checkpoint_monotone` / `checkpoint_bounded` /
`projection_idempotent` / `truth_is_journal` / `no_lost_update`。

**witness（4 本、in-module）**: `w_conflict` / `w_crash_then_catchup` /
`w_interleaved_writers` / `w_idempotent_catchup`。

ヘッダには ADR 0003 決定 6 の「モデル型 ↔ Rust Domain Primitive 対応表」と、
「アクション ↔ Rust ポート操作」の対応表、v1 の抽象化（未モデル化事項）、
E4 トレーサビリティ用の定義名一覧を置いた。

## 2. `quint typecheck` / `quint run` の出力

```
$ quint typecheck formal/orchestration/journal_protocol.qnt
（出力なし・exit 0）

$ quint run formal/orchestration/journal_protocol.qnt --seed 0x5e1 \
    --max-samples 3000 --max-steps 50 \
    --invariants conflict_rejected snapshot_tracks_journal version_equals_journal \
      checkpoint_monotone checkpoint_bounded projection_idempotent truth_is_journal \
      no_lost_update
...
[ok] No violation found (777ms at 3861 traces/second).
Trace length statistics: max=51, min=51, average=51.00
```

## 3. mutation 表（ADR 0003 決定 7 の DoD）

named invariant 8 本それぞれについて、対応するガード・遷移を壊した変異モデルを
**一時ディレクトリにだけ**作り（リポジトリには残していない）、その invariant **単独** の
`quint run --seed 0x5e1 --max-samples 5000 --max-steps 50 --invariant <inv>` が
violation を出すことを確認した。単独で走らせているので「別の invariant が先に落ちただけ」
という取り違えは起きない。

| # | invariant | 変異（何を壊したか） | 結果 |
|---|---|---|---|
| m1 | `conflict_rejected` | `store_conflict` がジャーナルに 1 行書いてしまう（rollback 漏れ） | DETECTED |
| m2 | `snapshot_tracks_journal` | `store_ok` がスナップショットの `seq_nr` を進め忘れる | DETECTED |
| m3 | `version_equals_journal` | `store_ok` が楽観 version を進め忘れる | DETECTED |
| m4 | `checkpoint_monotone` | `catchup` がチェックポイントを 1 つ後退させる（`advance_checkpoint` が後退を受理） | DETECTED |
| m5 | `checkpoint_bounded` | `catchup` がチェックポイントをジャーナルより先へ進める | DETECTED |
| m6 | `projection_idempotent` | `catchup` が読むものが無くても投影を 1 つ進める（非冪等） | DETECTED |
| m7 | `truth_is_journal` | `catchup` が投影をジャーナルより 1 つ先へ進める | DETECTED |
| m8 | `no_lost_update` | `store_ok` の楽観 version ガードを外す（stale な writer が上書きできる） | DETECTED |

**8/8 検出**。等価ミュータント（到達不能なガード除去）は選んでいない — m1・m8 は
「ロック退役後に並行制御が実際に壊れる」形の変異であり、m4〜m7 は投影側の
off-by-one という実装で起きやすい形にした。

## 4. witness 負形式 run の結果

`--invariant "not(w_x)"` を実行し、violation（exit code != 0）= 経路実在 = pass と読み替える
反転判定（seed `0x5e1`、`--max-samples 5000 --max-steps 50`）。

| witness | 意味 | 結果 |
|---|---|---|
| `w_conflict` | 衝突が実際に起きる（ロック無しでも並行書込が拒否される経路がある） | PASS |
| `w_crash_then_catchup` | crash の直後の catchup で投影がジャーナルに追いつく | PASS |
| `w_interleaved_writers` | 片方の `store_ok` の直後にもう片方が `store_conflict`（またはその逆） | PASS |
| `w_idempotent_catchup` | 追いついた状態（`prevCheckpoint == prevJournalLen`、`prevJournalLen > 0`）からの再 catchup | PASS |

`w_idempotent_catchup` は `projection_idempotent` の前提が空でないことの証明も兼ねる
（恒真式の不変条件を残さないため）。

## 5. fixture 一覧（seed・アクション網羅）

採取コマンド（各 seed）:

```
quint run formal/orchestration/journal_protocol.qnt --seed <seed> \
  --max-samples 1 --max-steps 40 --out-itf <raw>
```

`#meta` は既存 engine_loop フィクスチャと同じ形（`{"format","source","seed"}` のみ。
`format-description` / `status` / `description` / `timestamp` を除去）に正規化し、
セパレータ無し・末尾改行無しの 1 行 JSON でコミットした。**8 本すべてについて、モデルから
再採取 → 正規化した内容がコミット済みファイルとバイト一致することを確認済み**
（ADR 0003 決定 4 のフィクスチャ鮮度ゲートが通る状態）。

| fixture | 状態数 | journalLen 最大 | load | store_ok | store_conflict | catchup | crash | idle |
|---|---|---|---|---|---|---|---|---|
| `trace-0xa1.itf.json` | 41 | 7 | 5 | 7 | 3 | 6 | 10 | 9 |
| `trace-0xb2.itf.json` | 41 | 8 | 8 | 8 | 3 | 7 | 8 | 6 |
| `trace-0xc3.itf.json` | 41 | 8 | 6 | 8 | 1 | 7 | 10 | 8 |
| `trace-0xd4.itf.json` | 41 | 8 | 8 | 8 | 1 | 8 | 8 | 7 |
| `trace-0xe5.itf.json` | 41 | 5 | 11 | 5 | 0 | 8 | 11 | 5 |
| `trace-0xf6.itf.json` | 41 | 4 | 14 | 4 | 0 | 7 | 9 | 6 |
| `trace-0x101.itf.json` | 41 | 8 | 9 | 8 | 2 | 8 | 8 | 5 |
| `trace-0x202.itf.json` | 41 | 8 | 6 | 8 | 3 | 8 | 9 | 6 |
| **合計** | | | 67 | 56 | 13 | 59 | 73 | 52 |

**6 本以上**（8 本）・**全 6 アクション網羅**（`store_conflict` は 6 本に出現、計 13 回）。
準拠テストが全アクション網羅を assert しているので、稀アクションを含む fixture の
消失退行は CI で落ちる（engine_loop 準拠テストと同型）。

## 6. conformance の射影規則と結果

`modules/core/interface-adapter/tests/journal_protocol_conformance.rs`（`#[tokio::test]` 1 本）。

### 再生先の組み立て

- ストア: `InMemoryEventStore`（`EventStore` + `JournalReader` の共有ハンドル）。
- Repository: `InMemoryWorkflowExecutionRepository::with_store(store.clone())`。
- 投影: `FakeProjection { read_model_seq: u64 }` — モデルと同じく進捗しか持たない。
- writer 2 本: 同じ `IntentId` を別々に再水和した「ロード済み集約」。初期状態は
  両方とも genesis（`version` = 0、未書込の `Started` を保持）— モデルの
  `loadedVersion = [0, 0]`、`snapVersion = 0` と一致する。
- 合成計画は 24 ステージ（索引 0 = initialization 非ゲート、以降 = inception ゲート付き）。
  `--max-steps 40` のフィクスチャは 1 ステップ最大 1 イベントなのでジャーナルは 40 行を
  超えないが、この計画は genesis 1 + 索引 0 の完了 1 + ゲート 2 イベント × 23 = 48 件を
  受け付ける。再生の途中で「ワークフロー完了でコマンドが打てない」状態には入らない。

### 駆動（`lastAction` × `lastActor`）

| lastAction | 実装側の操作 | 追加の assert |
|---|---|---|
| `load` | `repository.find_by_id` → 成功なら writer を差し替え、`NotFound` なら genesis writer に戻す | 再水和した版 == `prevSnapVersion`。`NotFound` は `prevJournalLen == 0` のときだけ |
| `store_ok` | writer の下書き（複製にコマンドを打つ）を `repository.store` → `Ok` | 追記された `event.seq_nr()` == 当該状態の `journalLen` |
| `store_conflict` | 同じ下書きを `store` → `Err` | `RepositoryError::Conflict { expected: prevLoadedVersion[w], actual: prevSnapVersion }` と厳密一致 |
| `catchup` | `checkpoint` → `events_after` → 最後の global を投影に反映 → `advance_checkpoint` | 読むものが無ければ何もしない（＝冪等） |
| `crash` | 同じストアに Repository を**開き直す**（プロセス再起動相当） | `journalLen > 0` なら `find_by_id` の版 == `snapVersion`（Tx 済みの行は落ちない） |
| `idle` | 何もしない | — |

コマンドは writer が握っている集約の**複製**に対して打つ。書込が `Err` でも writer の
`loadedVersion` は 1 ビットも動かず、衝突のたびに再水和し直さずに次の試行ができる —
モデルの `store_conflict` が `loadedVersion` を変えない意味論と一致する。どのコマンドを
打つかは集約の状態だけで決まる（非ゲート → `complete_stage`、ゲート付き in-progress →
`open_gate`、awaiting → `approve_gate`）ので、モデル側に「どのコマンドか」の情報は要らない。

### 射影規則（各ステップで突合）

| モデル変数 | 実装側の観測 |
|---|---|
| `journalLen` | `JournalReader::events_after(ZERO)` の行数（併せて global 通番と `seq_nr` が 1 からの連番であることも確認） |
| `snapVersion` | `EventStore::get_latest_snapshot_by_id` が載せた `version()`（行が無ければ 0） |
| `snapSeq` | 同じ集約の `seq_nr()`（行が無ければ 0） |
| `checkpoint` | `JournalReader::checkpoint(ProjectionName)` の値 |
| `readModelSeq` | フェイク投影が描き終えた最後の global 通番 |
| `loadedVersion[w]` | writer w が握っている集約の `version()` |

### 結果

```
$ cargo test -p core-interface-adapter --test journal_protocol_conformance
test the_store_conforms_to_every_committed_journal_protocol_trace ... ok
test result: ok. 1 passed; 0 failed; ... finished in 0.14s
```

**検出力の確認（空回りでないことの証明）**: フィクスチャ 1 本の 12 番目の状態の
`snapVersion` を +1 だけずらすと、

```
assertion `left == right` failed: trace-0xa1.itf.json step 12 (crash): 再構成の版
  left: 1
 right: 2
```

で落ちることを確認した（確認後、当該 fixture はバイト一致で復元済み — §5 の再採取一致で
裏取り済み）。

## 7. quint-gate の差分と実行結果

追加は次の 4 箇所だけで、既存 engine_loop / stop_hook のステップ・seed・サンプル数は
一切変更していない。

1. 冒頭コメントのモデル一覧に `journal_protocol` を追加
2. `JOURNAL_PROTOCOL="formal/orchestration/journal_protocol.qnt"` を定義
3. typecheck ループの対象に `${JOURNAL_PROTOCOL}` を追加
4. `invariants run: journal_protocol`（`--seed 0x5e1 --max-samples 3000 --max-steps 50`、
   8 不変条件）と witness 4 本の負形式 run を追加

```
$ bash scripts/quint-gate.sh
==> summary
  [PASS] typecheck formal/orchestration/engine_loop.qnt
  [PASS] typecheck formal/orchestration/stop_hook.qnt
  [PASS] typecheck formal/orchestration/journal_protocol.qnt
  [PASS] invariants run: engine_loop
  [PASS] invariants run: stop_hook
  [PASS] invariants run: journal_protocol
  [PASS] witness w_block (formal/orchestration/stop_hook.qnt)
  [PASS] witness w_cap_release_interactive (formal/orchestration/stop_hook.qnt)
  [PASS] witness w_parked_auto_block (formal/orchestration/stop_hook.qnt)
  [PASS] witness w_seed2 (formal/orchestration/stop_hook.qnt)
  [PASS] witness w_sig_reset (formal/orchestration/stop_hook.qnt)
  [PASS] witness w_conflict (formal/orchestration/journal_protocol.qnt)
  [PASS] witness w_crash_then_catchup (formal/orchestration/journal_protocol.qnt)
  [PASS] witness w_interleaved_writers (formal/orchestration/journal_protocol.qnt)
  [PASS] witness w_idempotent_catchup (formal/orchestration/journal_protocol.qnt)
  [PASS] quint test --match 'r_.*' (formal/orchestration/stop_hook.qnt)

[PASS] quint gate: all steps green
```

## 8. その他の検査

- `rustfmt --edition 2024 --check modules/core/interface-adapter/tests/journal_protocol_conformance.rs` → clean
- `cargo clippy -p core-interface-adapter --test journal_protocol_conformance -- -D warnings` → 警告 0
  （初回 `missing_const_for_fn` 2 件を `const fn` 化で解消）
- `cargo test -p core-domain -p core-use-case` → 247 + 1（engine_loop 準拠）+ 48 すべて緑（回帰なし）
- 新規コードは `indexing_slicing` / `panic` を生まない方針に従い、添字は `get()` /
  `get_mut()` 経由。`unwrap` / `expect` / `panic` は integration test の慣例どおり
  file-level `#![allow]` で明示（既存 `workflow_execution_repository_contract.rs` と同型）。
- `cargo test --workspace` は**現時点で赤**だが、原因は委任 3 が作業中の SQLite テスト 4 本
  （`sqlite_event_store_test` / `workflow_execution_repository_impl_test` /
  `crash_reconstruction_test` / `workflow_execution_repository_contract`）が
  `SqliteEventStore` / `StorePath` / `WorkflowExecutionRepositoryImpl` を import しており、
  それらがまだ `orchestration/mod.rs` から `pub use` されていない（E0432）ためである。
  本委任の所有ファイルは 1 件も関与していない。

## 9. 設計質問

1. **フィクスチャ鮮度ゲートが CI に無い（既存の穴、engine_loop と共通）**
   ADR 0003 決定 4 は「`.qnt` 変更を含む PR では ITF フィクスチャ再生成ジョブを走らせ、
   `#meta` 正規化のうえ diff 一致を強制する」と定めているが、`scripts/quint-gate.sh` にも
   `.github/workflows/ci.yml` にもそのステップが無い（engine_loop についても未実装）。
   本委任では手作業で 8 本の再採取一致を確認したが、機械強制されていないため、モデルを
   触ったのに fixture を更新し忘れた PR は検出されない。**journal_protocol 単独ではなく
   engine_loop も含めた横断の作業**になるので、本委任のスコープ（既存ステップを変えない）を
   超えると判断し着手していない。後続 Bolt での採否をお願いしたい。

2. **`store_conflict` の Rust 側の材料は `Conflict` に一本化してよいか**
   モデルの `store_conflict` は「拒否された書込」の抽象で、実装では
   `RepositoryError::Conflict` に写した。実装にはもう 1 つ「呼出側のバグ」を表す
   `Corrupt(SequenceGap)`（版を載せ替えないまま次を書く）があり、これも状態を変えない点は
   同じだが、意味が違う（`Conflict` = 正常な並行制御、`SequenceGap` = 呼出側の不整合）ので
   モデルには入れていない。契約テスト `sequence_gap_is_refused` が別途固定しているため
   穴にはならないと判断したが、モデルに `store_invalid` として持たせるべきかは要裁定。

3. **`crash` の再生を「Repository 開き直し」にしたのは妥当か**
   ブリーフは `crash` → 何もしない、だったが、それだと Rust 側で `crash` が完全な no-op に
   なり、ITF 再生としての検査力がゼロになる。モデルの「状態不変」を破らない範囲で
   意味を持たせるため、同じストアに Repository を開き直し（プロセス再起動相当）、
   `journalLen > 0` なら `find_by_id` の版が `snapVersion` と一致することを assert した。
   `loadedVersion` は触っていないのでモデルとの整合は保たれている。

4. **`WRITERS = 2` の固定**
   モデルは `WRITERS` を定数にしており、3 以上での検査はしていない。2 writer で
   `no_lost_update` / `w_interleaved_writers` は検査できるが、「3 者以上の交錯でのみ現れる
   異常」があれば見逃す。FD BR3.3 が 2 と定めているため従ったが、nightly のランダム実行で
   `WRITERS = 3` も回す価値があるかは要判断。

## 10. 未了

- なし（Step 9 / 10 / 11 とも完了）。ただし §9-1 のフィクスチャ鮮度ゲートは
  **本委任のスコープ外の既知の穴**として残っている。
- `formal/README*` はリポジトリに存在しないため、行追加は行っていない。
- `git add` / `git commit` は行っていない（コンダクタの担当）。
