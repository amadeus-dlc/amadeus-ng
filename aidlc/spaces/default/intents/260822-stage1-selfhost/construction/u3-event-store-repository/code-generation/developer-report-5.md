# developer-report-5 — 委任 5: 仕様・正本の同期（U3 / Bolt B5）

> Code Generation（Construction 3.5）委任 5 の報告。所有ファイル: `docs/specs/{01-domain-model,10-orchestration,11-workspace,deviations}.md`。
> 出典: `code-generation-plan.md`（§5.5 Step 12）、`functional-design/{rules,functional-spec,functional-design-questions}.md`（BR3.3〜BR5.2、Q1〜Q4 = A）、
> `nfr-design/security-design.md`（§3/§6）、`inception/domain-design/decisions.md`（ADR-003/007）、B4 の作法（`u9-canon-docs/nfr-design/security-design.md` §2）。

## 1. 改訂一覧（ファイル:節 → 内容 → 出典注記）

### `docs/specs/10-orchestration.md`

| 節 | 内容 | 出典注記 |
|---|---|---|
| §3 ポート表（`WorkflowExecutionRepository` 実装欄） | `SqliteEventStore` を型名で明示し、ストアファイルパス `aidlc/spaces/<space>/intents/.aidlc-store.sqlite` を追加。登録簿 `intents.json` の直列化に `SqliteEventStore::within_write_transaction` を追記 | U3 FD Q1 = A, Q2 = A |
| §6 I14 行 | 「監査 emit が状態書込に先行…（audit-first）」→「各遷移は原子的 — ジャーナル追記とスナップショット更新を同一 Tx、楽観 version で直列化、投影はチェックポイントから冪等」。E4 定義名を `journal_protocol::{conflict_rejected, snapshot_tracks_journal, version_equals_journal, no_lost_update}` に差し替え | ADR-007 / Bolt B5 |
| §6 I14 直前の E4 定義名段落 | I14 の参照先を `formal/workspace/audit_lock.qnt` → `formal/orchestration/journal_protocol.qnt` に変更。B4 が入れた「注記（2026-08-23）: …B5 で差し替える」を削除（本文に反映済みのため） | ADR-007 / Bolt B5 |
| §6 I11 行 | 「E4 は audit_lock モデル（workspace 第一陣）と合流」が退役モデルへの言及のまま残っていたため、「旧 audit_lock…改訂されたため未定 — 再検討課題」に修正（journal_protocol.qnt は HUMAN_TURN の同秒判定を扱わないため、置換ではなく未決化） | Bolt B5（設計質問 1、下記） |
| §10 S2 行 | 「`audit_lock.qnt` は協定モデルへ改訂 — Bolt B5」（予告）→「`audit_lock.qnt` は退役し、協定モデル `formal/orchestration/journal_protocol.qnt` へ置換」（確定） | ADR-001 / ADR-003 / ADR-007, Bolt B5 |

### `docs/specs/11-workspace.md`

| 節 | 内容 | 出典注記 |
|---|---|---|
| §2.1 `Space` 集約行 | 「§2.2 `LockIdentity` の keying 規範は維持」→「単一 DB = 単一バケットに集約する（§2.2、ADR-007 / Bolt B5）」に短縮（LockIdentity 名を落とす） | ADR-007 / Bolt B5 |
| §2.2 `LockIdentity` 行 | 1 行に縮退: 「登録簿の直列化は `SqliteEventStore::within_write_transaction`（同一 DB の `BEGIN IMMEDIATE`）に置換。旧 keying 規範は『単一 DB = 単一バケット』で満たされる」 | U3 FD Q2 = A |
| §3 ポート表（`WorkflowExecutionRepository` 実装欄） | 10 号 §3 と同内容の追記（`SqliteEventStore` 型名・パス・`within_write_transaction`） | U3 FD Q1 = A, Q2 = A |
| §3「ポートではないもの」 | `Clock` / `ProcessProbe` / `Tmpdir` → `Clock` / `Tmpdir`（`ProcessProbe` は退役、reap 機構の消滅に伴う） | ADR-007 / Bolt B5 |
| §4 配置の規範 | `Clock` / `ProcessProbe` → `Clock`（`ProcessProbe` は退役の注記を追加） | ADR-007 / Bolt B5 |
| §6 E4 定義名段落 | W1〜W5 → J1〜J6 に対応する参照へ書き換え、`formal/orchestration/journal_protocol.qnt`（不変条件 8 / witness 4）を明記 | ADR-007 / Bolt B5 |
| §6 W1〜W5 行 | J1〜J6 の 6 行へ置換（conflict_rejected / snapshot_tracks_journal / version_equals_journal / checkpoint_monotone+checkpoint_bounded / projection_idempotent+truth_is_journal / no_lost_update）。強制はすべて E3+E4、E4 定義名は `journal_protocol::*` | U3 FD BR3.3, ADR-007 / Bolt B5 |
| §6 W13 行 | E4 定義名を「`LockIdentity` の構成関数が唯一の入口」→「`SqliteEventStore::within_write_transaction`（旧: `LockIdentity`…－ADR-007 / Bolt B5 で置換）」に更新。本文の機構記述も合わせて更新 | ADR-007 / Bolt B5（W1〜W5 と同じ理由での付随修正、下記「検査」参照） |
| §7 手順 3 | 「機構の `Clock` / `ProcessProbe` の fake」→「機構の `Clock` の fake」 | ADR-007 / Bolt B5 |
| §7-4 ITF 準拠 | `audit_lock.qnt`（改訂予定）→ `formal/orchestration/journal_protocol.qnt`（退役して置換済み）。再生先を `InMemoryEventStore` ＋フェイク投影と明記 | U3 FD BR3.5, Bolt B5 |
| §8 Quint 記録 | 末尾に「退役と置換（2026-08-23、Bolt B5）」を追加: `audit_lock.qnt`（v3）は ADR-007 で退役し `journal_protocol.qnt`（不変条件 8 / witness 4）へ置換。既存の v1〜v3 記録は退役前の実験履歴として保持する旨を明記 | ADR-007 / Bolt B5 |
| §9 実装ノート | 「stage-0/1 併用期の相互排他は未確定であり、§10 の未決事項として立てる」→「担保しない（stage-1 は単一クローン運用、§10 確定）」に、§10 の確定内容と整合させて更新 | U3 FD P7 / Q1 = A（付随修正） |
| §10 未決事項 2 件 | 「stage-0/1 併用期の相互排他」→ 確定（担保しない、stage-1 は単一クローン運用、併用は後続 intent）。「`intents.json` の直列化機構」→ 確定（`within_write_transaction`） | U3 FD P7, Q1 = A, Q2 = A |
| §10 未決事項（ロック keying の取消線行） | 「mkdir ロックの退役により audit_lock.qnt を協定モデルへ改訂する」（予告）→「確定: 退役し journal_protocol.qnt（不変条件 8 / witness 4）へ置換した」 | U3 FD Q4 = A（付随修正） |

### `docs/specs/01-domain-model.md`

| 節 | 内容 | 出典注記 |
|---|---|---|
| §3.3 代表不変条件の段落 | 「監査 emit が state 書込に先行…（旧 mkdir ロックモデル、B5 で差し替え予定）」を削除し、協定モデルの不変条件 3 本（真実源はジャーナル・楽観 version による直列化と競合拒否・チェックポイント単調＋投影冪等）に置換。「生きている閾値未満のロック保持者からは決して奪わない」（reap 系、退役済み）も落とした | ADR-007 / Bolt B5（付随修正、下記「検査」参照） |
| §3.3 状態機械の行 | `audit_lock.qnt` の改訂言及 → `formal/orchestration/journal_protocol.qnt`（ADR-007 により退役して置換、Bolt B5）に確定 | ADR-007 / Bolt B5 |
| §6 第一陣リスト（Audit lock lifecycle 行） | 「mkdir ロックの退役に伴う `audit_lock.qnt` の改訂で…」（予告）→「journal_protocol（`formal/orchestration/journal_protocol.qnt`、Bolt B5 で実装・ITF 準拠）」に確定 | Bolt B5 |

### `docs/specs/deviations.md`

| 節 | 内容 | 出典注記 |
|---|---|---|
| # 4 行（amadeus-ng の挙動欄） | SQLite ジャーナルの説明にストアファイルパス `aidlc/spaces/<space>/intents/.aidlc-store.sqlite` を明記 | U3 FD Q1 = A |
| # 4 行（理由欄） | 「`.aidlc-store.sqlite` 相当」→ 確定パス（space 単位 1 ファイル、既存 `.gitignore` で git 管理外である旨を明記） | U3 FD Q1 = A |
| # 4 行（記録欄） | 「最終パスは U3 で確定するため『相当』と記す — 確定時に本行を更新する」→「2026-08-23 Bolt B5 で確定」＋確定パスの再掲 | 2026-08-23 Bolt B5 |

## 2. 検査

- **表の列数検査**（B4 の `unit-test-instructions.md` §2 のスクリプトを対象 4 ファイルに適用して実行）: `tables ok`（不一致 0 件）。
- **見出し重複**: 4 ファイルとも `grep -n '^#' | sort | uniq -d` = 0 件（重複見出しなし）。
- **research/ 不変**: `git diff --stat -- docs/specs/research/` は空（変更ゼロ、逐語契約の保護を維持）。
- **`git add` / `git commit`**: 実行していない（作業ツリーへの変更のみ）。
- **退役語 grep**（`grep -nE 'WorkspaceLock|LockProtocol|LockIdentity|reap_eligible|audit_lock|ProcessProbe|withAuditLock' docs/specs/{01-domain-model,10-orchestration,11-workspace,deviations}.md`）: 21 件。全件を確認し、以下のいずれかに分類できる（未分類・規範として現役の記述は 0 件）。
  1. **明示的な「退役」注記**（本文に「退役」の語を伴う）: 01 号:101, 109, 218 / 10 号:109, 218 / 11 号:34, 53, 87, 105, 114, 141, 152, 167 — 計 13 件。
  2. **明示的な「旧」明記**（比較文の形で残す規約どおり）: 10 号:123（「旧 `audit_lock`」＋再検討課題） / 11 号:131（「旧: `LockIdentity`…」） / 11 号:166（「旧 `LockIdentity` の keying 規範」）— 計 3 件。
  3. **upstream 自身の機構への言及**（amadeus-ng 側の退役対象ではなく、upstream との比較文脈。10 号 §10 S2 も同様の文だが「退役」注記があるため上の 1 に計上済み）: 10 号:41（`withAuditLock` は upstream の機構、比較のため保持）— 1 件。
  4. **日付付き試行記録（10 号 §9 「Quint ゲート実験 — 記録」/ 11 号 §8 「Quint ゲート実験 — 第一陣 3 号の記録」）内の、当時の状態を記す過去形の記述**: 10 号:196, 209 / 11 号:146, 151 — 計 4 件。これらは時系列の実験ログであり、11 号 §8 末尾に追加した「退役と置換（2026-08-23、Bolt B5）」の注記（表の 1 の 11 号:152 に計上）が「上記の記録は退役前の実験履歴として保持する」と明記して位置づけを固定している。逐語の「旧」タグは付けていない（比較表形式ではなく時系列ログのため、B4 の作法における「旧→新の比較表」規約の対象外と判断した — 下記「設計質問」参照）。
  - 内訳合計: 13 + 3 + 1 + 4 = 21 件、報告書「作業」節の想定（「退役注記・履歴（旧）明記の行だけ」）と整合。
- **BR3.1 の grep 拡張（コード側、参考実行）**: `entities.md` の `RetiredLockMachinery` 記載どおり、`modules tools scripts formal .github Cargo.toml` に対する同 grep はコード側の委任（1〜4）の担当であり、本委任では実行対象外（`.claude/` のツールを含め、所有外ファイルへの変更・実行はしていない）。

## 3. 設計質問

1. **10 号 §6 I11 行の E4 化先が空白になった**: I11（同秒×別シャードの HUMAN_TURN フェイルクローズ）の E4 化は元々「audit_lock モデル（workspace 第一陣）と合流」の想定だったが、その audit_lock.qnt は ADR-007 で退役し、置換後の `journal_protocol.qnt`（BR3.3）はジャーナル/スナップショット/version/チェックポイントの協定のみを扱い、HUMAN_TURN の同秒判定は対象外。私の判断で「旧 audit_lock…改訂されたため未定 — 再検討課題」という事実どおりの記述に修正したが、I11 の E4 化を今後どのモデルで担うか（新規 slice 2 モデルを起こすか、E3 のまま据え置くか）は本委任のスコープ外の設計判断のため、コンダクタ裁定を仰ぎたい。
2. **§9/§8 の日付付き試行記録内の残存語**（上記「検査」4）: B4 の作法（`u9-canon-docs` §2）は「旧記述は『旧→新』の比較表に限り、見出しか1列目に『旧』と明記」だが、10 号 §9・11 号 §8 は比較表ではなく時系列の実験ログ（各エントリが「第◯回記録（日付）」の過去形の記述）である。これらは改訂対象ではなく既存の記録として保持し、逐語の「旧」タグは付けなかった。この扱いで問題なければ確認不要、比較表形式への変換が必要であれば追加委任として指示されたい。
3. **01 号 §3.3 の 4 番目の代表不変条件を削除した判断**: 「生きている閾値未満のロック保持者からは決して奪わない」（reap 系、E4）は brief の作業一覧に明示されていなかったが、ADR-007 のロック退役後は成立しない不変条件のため、協定モデルの不変条件（楽観 version・競合拒否）に置き換えた。文言列挙（真実源・楽観version・チェックポイント単調・投影冪等の4点）は brief の指示どおりに反映済み。

## 4. 未了

- なし（brief の作業一覧「10 号 / 11 号 / 01 号 / deviations.md」はすべて反映済み）。上記「設計質問」3 件はコンダクタ確認待ちだが、いずれも私の判断で暫定反映済みであり、追加の書き換えが必要な場合は再委任で対応する。
