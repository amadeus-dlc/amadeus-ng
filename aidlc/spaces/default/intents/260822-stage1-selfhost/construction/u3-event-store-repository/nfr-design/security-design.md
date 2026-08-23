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
