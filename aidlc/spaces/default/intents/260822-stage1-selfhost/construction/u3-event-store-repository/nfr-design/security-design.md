# security-design — U3 SQLite EventStore と WorkflowExecutionRepository（`u3-event-store-repository`）

> NFR Design（Construction 3.3）成果物（Unit: U3、kind: library、Bolt: B5）。出典: `../nfr-requirements/security-requirements.md`（NFR1.1〜NFR4.6、レビュー所見 1 =
> TOLERANCE 0.01）、`../nfr-requirements/tech-stack-decisions.md`、`../functional-design/functional-spec.md`（§3 フロー / §4 ワイヤ / §5 モデル / §6 退役 / §7 テスト）、
> `../functional-design/rules.md`（BR1.1〜BR5.2）、`../../../inception/contract-design/contract-summary.md`（C3 / C6）、確認事項 `nfr-design-questions.md`（P1〜P5、Looks correct）。

## 1. 設計方針

(a) ストアは信頼しない入力 — ドメイン型に至るまでに 3 段の検査点を置き、どこで失敗しても `Corrupt { cause }`（panic なし）。(b) 書込は 1 Tx に閉じ、競合は
状態を変えずに `Conflict`。(c) 失敗は材料だけを運ぶエラーで呼出側へ（再試行の政策はユースケース）。(d) 退役は一括・後方互換なし・grep で証明。(e) 協定は Quint で
検証し、ITF で実装に縫い付ける。

## 2. 検査点（NFR3.2 / NFR4.3 / NFR4.4）

| 段 | 場所 | 検査 | 失敗の写像 |
|---|---|---|---|
| 1 | `wire/` の復号（serde → ワイヤ構造体） | 列の `schema_version == 1`、`type` タグが 12 語の閉集合、未知フィールド拒否（`deny_unknown_fields` 相当）、JSON の型一致 | `Corrupt(SchemaVersion)` / `Corrupt(UnknownEventType)` / `Corrupt(UndecodablePayload)` |
| 2 | ワイヤ → ドメイン値（parse-don't-validate） | `IntentId::parse`（UUIDv7）、`StageSlug` / `PhaseId` / `PlanAction` / `CheckboxState` / `AutonomyMode` / `Status` / `JumpDirection` / `PhaseBoundary` / `WorkflowDefinitionId` / `DefinitionRevision` の各 parse、`StageIndex` の範囲（`cursor` / `parked_at` は state の長さで検査） | `Corrupt(UndecodablePayload)`（どの値かは `cause` に添える材料 — フィールド名） |
| 3 | ドメイン | `WorkflowExecution::from_state`（長さ整合・cursor 範囲・active 1 つ・gated Completed の承認・parked_at…）、`apply_event`（`SequenceGap` / `UnknownStage` / `InvariantViolation`） | `Corrupt(InvariantViolation)` / `Corrupt(SequenceGap)` |
| 前段 | open | `PRAGMA user_version`（0 → 初期化、1 → OK、他 → Err）、親 dir の存在 | `Schema { found, supported }` / `Io { kind: NotFound, path }` |

`Corrupt` は `aggregate_id`（IntentId）と `seq_nr`（該当行があれば）を材料として持ち、利用者向け文言はアダプタ層（U7 の message-catalog）が描く。

## 3. 書込の原子性と競合（NFR3.3 / NFR3.5）

- `persist_event_and_snapshot` は `BEGIN IMMEDIATE` → journal INSERT → snapshot INSERT / UPDATE（`WHERE version = expected`）→ COMMIT の単一 Tx。rusqlite の
  `Transaction` は drop で rollback するため、途中の `Err` / panic 経路でも半端な状態は残らない（COMMIT 前のクラッシュは何も残さない）。
- 競合の検出点は 2 つ（UNIQUE 違反 / 影響 0 行）で、どちらも rollback 後に `Conflict { expected, actual }`。`actual` は rollback 後に `SELECT version` で読む（無ければ 0）。
- `within_write_transaction` は同じ `BEGIN IMMEDIATE` で登録簿の read-modify-write を包む。`busy_timeout` 5000ms 超過は rusqlite の `Busy` を `Io { kind: WouldBlock }`
  に写す（黙って失敗しない — NFR3.5）。
- 再試行の政策はユースケース（U5: `Conflict` のとき再水和して 1 回）。Repository は再試行しない（C3 ③）。

## 4. 障害ドメインと扱い（P2）

| 障害 | 検出 | 扱い |
|---|---|---|
| ストア I/O（権限・ディスク・親 dir 欠落） | `Io { kind, path }` | 呼出側へ返す。再試行なし。ストアの自動修復はしない |
| 競合 | `Conflict` | ユースケースが再水和 + 1 回再試行（U5） |
| 破損・版不一致 | `Corrupt` / `Schema` | 中断。投影（U4）はジャーナルから冪等に再生成できる。ジャーナル自体の破損は利用者の操作（バックアップからの復元）— 本 Unit は検出まで |
| Busy 超過 | `Io(WouldBlock)` | 中断し、再実行を促す（文言は U7） |

## 5. サプライチェーンと境界（NFR4.1 / NFR4.2 / NFR4.5 / NFR4.6）

- 依存は `rusqlite`（bundled）と `tokio`（rt / macros）を固定版で workspace 依存に、adapter の `md5` を除去。`cargo audit` が CI で検査。`unsafe_code = forbid` は自クレートに
  適用（`libsqlite3-sys` の unsafe は依存として受容）。
- `core-use-case` は外部クレートを足さない（trait の `async fn` は言語機能）。`core-domain` は変更なし（標準ライブラリのみ）。
- パス・Clock は注入（`std::env` を読まない）。ログ出力なし。ストアは umask 既定で作成、親 dir は作らない。

## 6. 退役の安全手順（NFR1.2 / FD BR3.1）

1. ロック系（use-case `workspace/`、adapter `fs_workspace_lock` / `process_probe`、domain `lock_protocol` / `lock_identity`、infra-io `process_probe`、`audit_lock.qnt` +
   fixtures + conformance、lint `reap-decision-locality`、`md5`）を**1 コミット**で削除 → `cargo build --workspace` → grep（BR3.1 の語）= 0 件 → 既存スイート
   （engine_loop ITF / ゴールデン / WorkflowDefinitionRepository）緑。
2. 後方互換の型エイリアス・deprecated・feature flag を作らない。
3. `scripts/quint-gate.sh` は audit_lock ステップを削除し journal_protocol ステップを追加（同一コミットでもよいが、Quint モデル追加のコミットと分ける）。
4. `scripts/coverage.sh` の `TOLERANCE` を 0.05 → 0.01、冒頭コメントを更新（NFR 要求レビュー所見 1）。

## 7. 決定性と協定の維持（NFR2.2 / NFR2.5 / NFR3.1 / NFR3.4）

- 正準 JSON（canon-json）でバイト決定的。`updated_at` は再構成に使わない。
- Quint `journal_protocol.qnt`: 不変条件 8 本は状態遷移レベル（prev → current）で書き、named invariant ごとに 1 変異で検出を確認（表を code-summary に）。witness 4 本は
  in-module で負形式 run。ITF fixture は `#meta` 正規化（既存の engine_loop 採取手順）。
- ITF 準拠テストは `InMemoryEventStore` + フェイク投影に再生（adapter tests）。SQLite 実装は契約テストで InMemory と同値性を保証するため、ITF は InMemory で十分。

## 8. 失敗の扱い（プロセス）

- 受入（FD BR5.2）のいずれかが落ちたら PR を戻す。設計に無い判断が要ったら推測で進めず developer-report の「設計質問」に書く（B3 / B4 の運用）。

## 9. 要求への対応

| 要求 | 設計上の手当て |
|---|---|
| NFR1.1 / NFR1.2 | 逸脱 # 4 のパス確定（BR5.1）、退役手順（§6） |
| NFR2.1〜2.4 | テスト配置（logical-components §4）、TOLERANCE 0.01（§6-4） |
| NFR2.5 | Quint DoD（§7） |
| NFR3.1 | 決定性（§7） |
| NFR3.2 | 3 段の検査点（§2） |
| NFR3.3 | 単一 Tx + 楽観 version、rollback（§3） |
| NFR3.4 | チェックポイント単調性（FD BR1.4、契約テスト — logical-components §4） |
| NFR3.5 | within_write_transaction と Busy の扱い（§3） |
| NFR4.1 / 4.2 | 依存差分と forbid（§5） |
| NFR4.3 | 事前検査 + Err、エラー写像（§2 / §3） |
| NFR4.4 | 3 段の検査点を省略しない（§2） |
| NFR4.5 / 4.6 | 注入・ログなし・umask（§5） |

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-08-23T09:23:55Z
**Iteration:** 1（advisory, unit: u3-event-store-repository）

### Findings

| # | Severity | Location | Finding | Recommendation |
|---|---|---|---|---|
| 1 | Minor | security-design.md §2/§5、`security-requirements.md` NFR4.3 | NFR4.3 requirements 側の合格基準は「`unwrap` / `expect` / `panic!` / `indexing_slicing` を生まない」を clippy deny で機械強制する前提だが、実測（`Cargo.toml` `[workspace.lints.clippy]`）では `unwrap_used` / `expect_used` は deny 済みの一方、`indexing_slicing` と `panic` はいずれも未設定のまま（nfr-requirements レビュー iteration 1 の Major 所見2 として既出）。この所見は `pending-revision.md` に採録された 1 件（TOLERANCE 0.05→0.01、既に security-design §6-4 が反映済み）に含まれておらず、本 nfr-design の §5（サプライチェーンと境界）も `unsafe_code = forbid` と依存差分のみを扱い、この 2 lint の追加/見送りには触れていない。§2 の 3 段検査点はまさにこの種のパニック（範囲外索引・`seq_nr − 1` の減算）が起きやすい箇所であるため、機械強制の空白が実装まで持ち越される。 | 既に一度提示済みの所見であり再提起はしないが、承認ゲートで「意図的に見送る」か「B5 のスコープに `indexing_slicing = "deny"` / `panic = "deny"` の追加を含める」かを一言で確定させておくことを推奨する。 |
| 2 | Minor | `functional-design/entities.md` の `## Review`（iteration 1、08:40:40Z） | 上流 FD 成果物 `entities.md` に埋め込まれた `## Review` はいまも文字どおり `Verdict: NOT-READY`（Critical 1: genesis での `aggregate.version() − 1` の u64 アンダーフロー + 恒常的な偽陽性 Conflict、Major 2 件: C3 `usize`→`u64` の無言変更、`&self`/`&mut self` の内部可変性戦略欠落）を記録している。しかし実コードを突合すると、`rules.md` BR1.3 と `functional-spec.md` §3.1 は既に `expected = aggregate.version()`（減算なし）へ修正済みで、`entities.md` の `EventStore` / `WorkflowExecutionRepositoryImpl.store` の記述も「レビュー所見 2」「本レビュー所見 3」と名指しで参照しながら u64 具体化の理由と `RefCell` 内部可変性戦略を明記済みだった（3 件とも解消を確認）。つまり本 nfr-design（security-design.md §3、functional-spec.md §3.1）が依拠している版はすでに修正後の内容であり、バグそのものは nfr-design には伝播していない。ただし `entities.md` 末尾の `## Review` ブロックだけが iteration 2 として更新されておらず、監査証跡としては「NOT-READY のまま」に読める状態が残っている。 | nfr-design 自体の修正は不要（伝播なしを確認済み）。承認ゲートで `entities.md` の `## Review` を iteration 2 として更新するか、少なくとも「3 件とも本文で解消済み」という一文を追記し、監査証跡の齟齬を解消することを推奨する。 |
| 3 | Minor | security-design.md §5、logical-components.md §1、`contract-summary.md` C3（97-135 行） | C3（所有者 U5/U6、`contract-summary.md` §3）は `EventStore::persist_event` / `get_events_by_id_since_seq_nr` の数値パラメータをいまも `usize` で定義しており、直近の改訂（2026-08-23、C4/ADR-008）でも変更されていない。nfr-design（および依拠元の `entities.md` BR1.1）はこれを一貫して `u64` として扱い、「C3 改訂提案として所有者 U5/U6 へ申し送り — 無言の変更にしない」と明記しており、U3 側の対応としては適切（サイレントな乖離ではない）。しかし `contract-summary.md` 側にはこの申し送りに対応する注記や pending 項目が見当たらず、U5/U6 のどちらの Bolt がこの改訂を引き取るかが共有契約上は未着地のままである。U3（Bolt B5）と U5/U6 のいずれかの Bolt が異なる型で trait 実装を進めた場合、コンパイル時の型不一致リスクが残る。 | 承認ゲートで、この「申し送り」が U5 または U6 のどちらのバックログ / Bolt に着地しているかを一言で確認し、必要なら `contract-summary.md` 側にも pending 注記を残すことを推奨する（U3 側の設計修正は不要）。 |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| `bun .claude/tools/aidlc-sensor-traceability.ts --stage nfr-design --output-path nfr-design/traceability.json` | PASS（`{"pass":true,"gaps":[],"orphans":[],"missing_from_table":[],"missing_from_upstream_ids":[],"invalid_entries":[],"invalid_targets":[]}`） | NFR1.1〜NFR4.6 の 18 件すべてに過不足のない coverage |
| `bun .claude/tools/aidlc-sensor-required-sections.ts`（security-design.md） | PASS（h2_count=9） | §1〜§9 の H2 見出しを検出 |
| `bun .claude/tools/aidlc-sensor-required-sections.ts`（logical-components.md） | PASS（h2_count=5） | §1〜§5 の H2 見出しを検出 |
| 実コード突合（`modules/core/domain/src/orchestration/workflow_execution.rs` の `check_invariants`（873-925 行）/ `apply_event`（744-759 行）/ `from_snapshot`（959-1005 行）） | 一致 | §2 の「段 3」が主張する検査（長さ整合・cursor 範囲・parked_at 範囲・at_most_one_active・no_gate_bypass・SequenceGap・InvariantViolation）はすべて実装済みコードに実在し、記述と食い違いなし |
| 実ファイル突合（`Cargo.toml` `[workspace.lints]`、`scripts/coverage.sh` の `TOLERANCE=0.05`、`scripts/quint-gate.sh` の `audit_lock` ステップ、interface-adapter `Cargo.toml` の `md5 = "0.8"`） | 概ね整合 | `unsafe_code = "forbid"` は既存（新規適用不要、§5 の記述と整合）。`TOLERANCE=0.05` と `audit_lock` ステップ・`md5` 依存は現状値であり、§6 の退役手順（削除・0.05→0.01・audit_lock→journal_protocol 置換）が対象とする現状と一致。`indexing_slicing`/`panic` lint 不在は Finding #1 として記録 |
| rusqlite 実挙動の確認（設計知識ベース） | 整合 | `Transaction` の drop-rollback、`SQLITE_BUSY` の `Busy` エラー変換という§3の前提は rusqlite の実際の API 契約と一致 |

### Summary

security-design.md / logical-components.md / traceability.json は、上流（`security-requirements.md` NFR1.1〜NFR4.6、`functional-spec.md` / `rules.md` の BR1.x〜BR5.x、C3/C6）および実装済みドメインコード（`workflow_execution.rs` の `check_invariants` / `apply_event` / `from_snapshot`）の双方と高い精度で一致しており、traceability・required-sections の両センサーも PASS した。3 段の検査点（NFR3.2/4.3/4.4）・Tx と競合の設計（NFR3.3/3.5、BR1.3/BR2.3/BR2.4 と整合、rusqlite の drop-rollback/Busy 挙動も正確）・障害ドメインと再試行政策（C3 ③ と一致）・退役手順（BR3.1、順序も安全）・Quint DoD（§7）はいずれも実装可能な精度で書かれている。3 件の Minor 所見はいずれも本 nfr-design の設計そのものの欠陥ではなく、(1) 既出だが pending-revision に採録されなかった clippy lint（indexing_slicing/panic）の機械強制空白、(2) 上流 `entities.md` の `## Review` ブロックが実際には解消済みの Critical/Major を NOT-READY のまま表示し続けている監査証跡の齟齬（nfr-design 自体への伝播は無いことを実コード突合で確認済み）、(3) U3 が適切に申し送った C3 の `usize`→`u64` 改訂提案が `contract-summary.md` 側にまだ着地していないという、承認ゲートで一言確認すれば足りる残課題である。Critical 0 / Major 0 のため advisory の READY 閾値を十分に満たす。
