# functional-spec — U3 イベントストアと IntentExecutionRepository

> 2026-09-05 是正。本文は現行の裁定・ポート・実装に同期した。末尾の Review は過去の記録として保持する。
> 出典: `../../../inception/units-generation/unit-of-work.md`、
> `../../../inception/units-generation/unit-of-work-story-map.md`、
> `../../../inception/requirements-analysis/requirements.md`、
> `../../../inception/contract-design/contract-summary.md`（C3 / C6）、
> `../../../inception/domain-design/decisions.md`（ADR-001 / ADR-007 / ADR-010）、
> `entities.md`、`rules.md`、`traceability.json`。
> 既存 Review への対応と実測は `../correction-report.md` に記録する。

## 1. 責務・配置・要求対応

U3 はイベントジャーナルへの書込と `IntentExecution` の再構成を担う。
FR1 の主担当として子要求の対応を集約し、FR1.2（原子的保存と楽観競合制御）・FR1.3（Repository）を実装する。
FR1.1 の監査投影・横断読取は U4 の担当であり、親要求への対応追加によって U3 へ移管しない。
NFR3 の再構成は U3、投影の再生成は U4 が担う。

| 所有 | 現行の型・ファイル | 責務 |
|---|---|---|
| core-command-use-case | orchestration/port/intent_execution_repository.rs、repository_error.rs | 集約 Repository の署名と共通エラー |
| core-command-domain | IntentExecution / IntentExecutionEvent / IntentExecutionId、workspace の StorePath / IntentDirName | ドメイン遷移・識別・検査付き再構成 |
| core-command-interface-adapter | orchestration/intent_execution_repository_impl.rs、snapshot_strategy.rs、dto/ | 本家ストアの利用、永続化DTO、差分再生への変換 |
| core-read-model-updater | orchestration/journal_reader.rs、journal_reader_impl.rs、JournalBatch / JournalReadError / GlobalSeqNr / ProjectionName | U4 所有の横断読取・投影公開・チェックポイント |
| 本家 event-store-adapter-rs =3.0.0 | EventStore / EventEnvelope / SnapshotEnvelope、SQLite / memory | journal・snapshot の格納、Tx、CAS、版の採番 |
| app/aidlc のテスト | tests/journal_protocol_conformance.rs | 書込側とRMUを結合するITF適合検証 |

C3 の古い `WorkflowExecutionRepository` / `RehydratedWorkflowExecution` / 裸の `expected_version` を現行署名と読み替えない。
B8 の RMU 移動と、現行ポート `modules/core/command/use-case/src/orchestration/port/intent_execution_repository.rs`
の「楽観 version は集約が運ぶ」に記録された2026-08-30の裁定を以下に反映した。版の所有の根拠をC3のB13追記とはしない。
自前 EventStore、旧 SQL 手順、`within_write_transaction`、domain の serde-memento は後継設計に含めない。

## 2. 公開契約と版の持ち回り

```rust
pub trait IntentExecutionRepository {
    async fn find_by_id(
        &self,
        id: &IntentExecutionId,
    ) -> Result<IntentExecution, RepositoryError<IntentExecutionId>>;

    async fn store(
        &mut self,
        event: &IntentExecutionEvent,
        aggregate: &IntentExecution,
    ) -> Result<(), RepositoryError<IntentExecutionId>>;
}
```

実装はストアを単一所有し、書込に `&mut self` を使う。追加の内部可変性は不要である。
版の正本は `SnapshotEnvelope::version()`。読取で集約へ載せ、コマンド適用後も同じ版を保持し、
`store` が `aggregate.version()` を期待値として本家へ渡す。
`version` と `seq_nr` はともに `usize` だが意味が異なり、版を通番から導かない。
書込直前の版の読み直しは楽観競合検出を失わせるため行わない。
`store` の引数は参照なので、保存成功後に続けて保存する場合も `find_by_id` から取り直す。

`RepositoryError<Id>` は `NotFound { id }`、`Conflict { expected, actual }`、
`Io { kind, path }`、`Corrupt { id, seq_nr, source }`。
書込ポートの公開 `CorruptCause` は廃止され、原因は `Error::source` で連鎖する。
RMU の `JournalReadError::Corrupt { cause }` は別契約であり混同しない。
`JournalReader` の全署名は RMU の現行 trait を参照する。

## 3. 保存と再構成

### 3.1 store（BR1.3 / BR2.3 / BR2.6）

1. 呼出側は同一コマンドで得た単一イベントと適用後集約を渡す。イベントは `aggregate_id` を持つため、
   別実行のイベントも同じ Rust 型で渡せる。「型により構成不能」という B7 の説明は撤回する。
2. Repository は `event.aggregate_id() != aggregate.id()` を書込前に検査する。
   不一致なら `Corrupt`（アダプタ私有の `WriteContract` を source に持つ）として I/O 前に拒否する。
   この検査は ID の一致を保証する。同じ ID の任意イベントと集約状態が意味的に対応することまで証明するものではない。
3. 本家封筒の外側 ID・通番・発生時刻を集約から、payload をイベント DTO から組む。
   manifest は `intent-execution-event/1`、期待版は `aggregate.version()`。
4. `seq_nr == 1` は必ず `persist_event_and_snapshot`。genesis の期待版は0で、本家が初期版を採番する。
   以後は `SnapshotStrategy::wants_snapshot(seq_nr)` が真なら同関数、それ以外は `persist_event` を呼ぶ。
5. 本家が Tx と CAS を実行する。成功しても呼出側の集約は変更しない。
   競合は `Conflict` に写し、actual は診断のためだけに読取る（読めなければ0）。
   符号化失敗・本家の書込契約違反は `Corrupt`、I/O は `Io`。再試行政策はユースケースに置く。

`SnapshotStrategy::every(NonZeroUsize)` の既定値は10。
初回は設定に関係なく基底を作り、以後は通番が間隔の倍数であるときに基底を更新する。
イベントのみ保存しても snapshot 行の楽観版は進むが、基底 payload と通番は維持される。
間隔2で3イベント保存した場合、基底通番2・行の版3からイベント3を再生する。
既定10では3イベント後も基底通番1であり、イベント2・3が差分になる。

### 3.2 find_by_id（BR1.2 / BR1.5）

1. 要求された `IntentExecutionId` で最新スナップショットを読む。
2. 基底がない場合だけ、通番1を包含下限として同集約の journal を読む。
   行がなければ `NotFound`、行が残っていれば `Corrupt`（MissingSnapshot）。
   この照会自体が失敗した場合は、ストアの失敗を `Io` / `Corrupt` に写す。
3. 基底 DTO の `to_domain()` を通し、検査付き再構成コンストラクタで復元する。
   復号や基底の不変条件検査に失敗したら `Corrupt`。旧 `from_state` / domain serde 経路は使わない。
4. `base.seq_nr() + 1` を包含下限として後続イベントだけを読み、昇順に検査する。
   戻された差分の通番が連続していること、manifest が一致すること、
   DTO がドメインに復号できること、payload の `aggregate_id` が要求 ID に一致することを確認する。
   分類可能な違反は `Corrupt` とし、部分集約を返さない。
5. `IntentExecution::replay(base, events).with_version(snapshot.version())` で再構成する。
   DTO として成立したイベントでも未知ステージ等の壊れた遷移はドメインがクラッシュで停止する。
   この境界を全て `Corrupt` に変換するとは約束しない。

読取・検査の範囲は最新基底と、その通番を超える差分である。
基底以前の journal を再検査・全履歴リプレイしない。
戻された差分の途中欠落は検出するが、別の終端記録を使った末尾欠落の完全検出は行わない。
journal を全削除した場合に古い基底だけが返るという既存試験は、この検出範囲を示す。
ジャーナルの削除や欠落を許容したという新しい承認、監査完全性の証明には使わない。

## 4. 永続化表現と読取側の境界

集約・イベント・キーの永続化 DTO はアダプタが所有し、本家が serde で格納する。
属性の正本は `modules/core/command/interface-adapter/src/orchestration/dto/` にある。
ドメインは永続化知識から中立とし、DTO 復号を検査付きドメイン構築へ変換する。
楽観版を集約 payload に複製せず、snapshot 行の列から受け渡す。

旧「イベントに識別子はない」「状態は16/17属性のmemento」「未知フィールドは全て拒否」という表を
現行ワイヤ契約へ流用しない。現在のイベントにはイベント ID と集約 ID があり、
追加属性の省略時意味などは各 DTO の具体的な検査に従う。
格納 payload と、U1 が担う upstream 観測面の正準 JSON は異なる契約である。
RMU は専用 DTO と `JournalBatch` を使い、コマンド側の永続化 DTO を共有しない。

## 5. 検証モデルの適用範囲（BR3.3 / BR3.4 / BR3.5）

`formal/orchestration/journal_protocol.qnt` は1集約・writer2・投影1、毎イベント基底更新を表す。
`snapshot_tracks_journal` の `snapSeq == journalLen` はこの設定に限る。
`modules/app/aidlc/tests/journal_protocol_conformance.rs` は `SnapshotStrategy::every(1)` を明示し、
コミット済み ITF を Repository・JournalReaderImpl と結合して再生する。
モデル内の版の算術は抽象化であり、Repository が版を通番から計算する規則ではない。

既定10や任意間隔Nでの保存分岐・差分再生は、このモデルの証明範囲に拡張しない。
別の契約・実装テストで確認する。
「モデルは以前変更せず通った」という履歴だけを現在の合格証拠にはしない。

## 6. 退役した設計

ADR-007 の旧 mkdir ロック、ローカル EventStore trait、独自 SQLite DDL、
`within_write_transaction`、旧ワイヤ型、domain serde-memento は再導入しない。
旧 `WorkflowExecutionRepository` の名前や再水和専用の器も現行公開 API に残さない。
登録簿の I/O と投影公開の詳細は、それぞれの所有 Unit の現行契約で扱う。

## 7. 検証と確認できた範囲

| 対象 | 根拠 | 確認内容 |
|---|---|---|
| Repository 共通契約 | intent_execution_repository_contract.rs | memory / SQLite の保存・検索・競合・参照不変性、および別実行イベントの保存前拒否 |
| 基底と差分 | intent_execution_repository_impl_test.rs | 初回必須、間隔2、古い基底からの差分再生、版維持 |
| 破損境界 | 同実装テスト | 基底欠落、DTO破損、差分通番飛び、foreign manifest、別実行payload、未知ステージでのクラッシュ |
| 基底以前の履歴 | 同実装テスト | 基底以前のforeign manifestを読まない、snapshot単独の復元。この範囲外の監査完全性は保証しない |
| 形式規則 | intent_dir_name.rs のテスト | 連続ハイフンと空区間の拒否 |
| モデル適合 | app/aidlc/tests/journal_protocol_conformance.rs | every(1) を明示したITF再生。既定10のモデル証明とはしない |

2026-09-05 の R-07 是正では、別実行の Started と対象集約の組を渡す拒否期待が
memory / SQLite の両方で失敗し、保存が `Ok(())` になることを親担当が実測した。
書込前 ID 照合を追加した後、共通契約22件と実装固有23件の計45件が成功した。
追加契約は genesis の双方 NotFound 維持と、更新拒否後の対象集約の元状態維持を確認する。
これは ID 不一致の検出根拠であり、任意の同一 ID イベントと集約状態の対応を証明するものではない。
正式な Review・承認操作はこの是正では行っていない。

関連する保存境界も親担当が横断確認した。
`WorkflowDefinitionRepositoryImpl` は別系譜の Defined と対象 definition を両バックエンドで保存していたため、
同じ書込前 ID 照合を加えた。共通契約14件と実装固有12件の計26件が成功し、
追加契約は新規・更新の拒否と状態維持を確認した。
`IntentRepositoryImpl` は既に Created から再構成した集約全体を対象と照合しており、
今回の2実装の検査欠落とは区別する。

## 8. 未解決と申し送り

R-04〜R-09 の現行本文は `../correction-report.md` の対応表で追跡する。
過去 Review の判定とステータスは書き換えず、是正完了と正式レビューの再判定を分ける。
間欠スナップショットを含む一般モデルへの拡張は本是正の対象外であり、every(1)限定を明示している。
この文書から新たなジャーナル削除許可、全履歴再生への変更、保存形式の再設計を導かない。

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-07T05:38:03Z
**Iteration:** 1
**Request Challenge:** review:30b70b9a25be173ec55f85af6cdb7893

### Findings

前回（2026-09-05）の ID `1`・`2`・`3`・`R-04`〜`R-09` を振り直さずに再掲し、現行ワーキングツリー（HEAD `f2b6b6a9`、コード未変更）で各所を再検査して Status を更新した。新規所見は `R-10` から採番する。前回 Critical・Major はすべて現行コードとの照合で解消を確認した。新規は成果物の形式・出典・実測値の鮮度に関するもので、実装を妨げるものではない。

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| 1 | Critical | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/functional-design/rules.md > BR1.2・BR1.3・BR2.3 | 版の算術は現行本文から消えている。BR1.3 は「expected_version は aggregate.version() をそのまま使い、seq_nr から導かない」と定め、実装 `intent_execution_repository_impl.rs:453` も `let expected_version = aggregate.version();` である。ポートの単体テスト（`intent_execution_repository.rs:153-167`）が genesis 後の再書込で `Conflict { expected: 0, actual: 1 }` を固定し、契約テスト 22 件が両バックエンドで成功する。アンダーフロー・常時偽競合の原因は無い。 | 追加対応なし。版を seq_nr から導く旧案は再採用しない。 | Resolved |
| 2 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/functional-design/entities.md > EventStore、および rules.md > BR1.1 | BR1.1 は「版と通番は本家どおり usize」と明記し、`repository_error.rs:36,38,52` の `expected` / `actual` / `seq_nr` も `usize`、ポート署名も `usize` である。ローカル trait と u64 化は本文から失効済みで、型不一致は残っていない。 | 追加対応なし。借り物の契約の原子型を書き換える旧案は再採用しない。 | Resolved |
| 3 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/functional-design/entities.md > IntentExecutionRepositoryImpl、および functional-spec.md > 第 2 節 | `store` は設計・ポート・実装のいずれも `&mut self` で、実装はストアを型引数 `S` として単一所有する。3 成果物に `Mutex` / `RefCell` / `Cell` / interior の語は 0 件で、内部可変性を足す必要が無い。 | 追加対応なし。 | Resolved |
| R-04 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/functional-design/functional-spec.md > 第 1・2 節、および entities.md > Repository・JournalReader・RepositoryError | 第 2 節のコードブロックは現行ポート `intent_execution_repository.rs:71-98` と署名が一字一句一致する。`RepositoryError<Id>` の 4 変種（NotFound / Conflict / Io / Corrupt）は `repository_error.rs:27-56` と一致し、公開 `CorruptCause` は書込側に存在せず RMU（`journal_read_error.rs`）だけが持つ。`JournalReader` は第 1 節の所有表と entities.md で `core-read-model-updater` 所有と明記され、抜粋 3 署名（events_after / checkpoint / advance_checkpoint(&mut self, &ProjectionName, GlobalSeqNr, &ReadTables)）は `journal_reader.rs:77,85,109-114` と一致する。C3 の旧署名との差も第 1 節が明示している。 | 追加対応なし。 | Resolved |
| R-05 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/functional-design/functional-spec.md > 第 2 節・第 3.1 節・第 5 節、および rules.md > BR3.3 | 第 3.1 節手順 4 の「seq_nr == 1 は必ず persist_event_and_snapshot、以後は wants_snapshot が真なら同関数、それ以外は persist_event」は実装 `intent_execution_repository_impl.rs:465-476` の分岐と一致する。`SnapshotStrategy::every(NonZeroUsize)` の既定 10（`snapshot_strategy.rs:30-35`）も一致。BR3.3 と第 5 節は `snapshot_tracks_journal` を every(1) 限定と明示し、ITF 再生先 `journal_protocol_conformance.rs:226-227` が `SnapshotStrategy::every(1)` を明示している。 | 追加対応なし。 | Resolved |
| R-06 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/functional-design/functional-spec.md > 第 3.2 節・第 7 節、および rules.md > BR1.2 | 第 3.2 節の 5 手順は `find_by_id`（`intent_execution_repository_impl.rs:331-438`）と対応する。基底不在時の NotFound / MissingSnapshot 分岐、`base.seq_nr() + 1` を下限とする差分取得、通番連続・manifest・DTO 復号・payload の aggregate_id の 4 検査、`replay(base, events).with_version(version)`、未知ステージのクラッシュ境界がすべて記述と一致する。基底以前を再検査しない範囲も本文が明示している。 | 追加対応なし。 | Resolved |
| R-07 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/functional-design/functional-spec.md > 第 3.1 節手順 1・2 | 型保証の主張は手順 1 で撤回され、手順 2 が書込前の ID 照合を規定する。実装 `intent_execution_repository_impl.rs:447-452` が `event.aggregate_id() != aggregate.id()` を I/O 前に `Corrupt`（source = 私有 `CorruptDetail::WriteContract`）で拒否し、共通契約テスト `an_event_from_another_execution_is_rejected_before_writing` が memory / SQLite の両方で成功する。ID 一致が意味的対応まで証明しないという限界も本文が明記している。 | 追加対応なし。 | Resolved |
| R-08 | Minor | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/functional-design/rules.md > BR1.2・BR1.3、および traceability.json > upstream_ids・coverage | 未定義 BR 参照（BR1.7 / BR5.3）は rules.md から消え、traceability センサーの `orphans` は空になった。親 FR1 が `upstream_ids` と `coverage` に追加され、unit-of-work-story-map は functional-spec.md・entities.md・rules.md・traceability.json の 4 箇所から参照されている。FR1.1 の実装担当は U4 のままである。 | 追加対応なし。 | Resolved |
| R-09 | Minor | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/functional-design/entities.md > IntentDirName、および rules.md > BR4.2 | entities.md（`^[0-9]{6}-[a-z0-9]+(?:-[a-z0-9]+)*$`、空区間・連続ハイフン・末尾ハイフンを拒否）と BR4.2（同式、受理例 260822-a-b / 拒否例 260822-a--b・260822-a-・260822-）が一致し、現行実装のテスト `intent_dir_name.rs:165-168` の `a_segment_may_not_be_empty` が `260822-a--b` の拒否を固定している。 | 追加対応なし。 | Resolved |
| R-10 | Minor | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/functional-design/entities.md・rules.md・functional-spec.md > 出典ブロック | upstream-coverage センサーが `components` を未参照として FAIL する（required な consume）。3 成果物のいずれも domain-design components.md を出典に挙げていない。この Unit で components.md に触れているのは質問票だけで、そこでの呼称は退役した旧名 `PersistenceGateways` である（components.md 240 行・608 行で `CommandGateways` へ改名済み、U9 再走 2026-09-07）。境界そのものは components.md 205 行の `IntentExecutionRepository { find_by_id(&IntentExecutionId), store(&mut self, &IntentExecutionEvent, &IntentExecution) }` と一致しており、実装は妨げられない。 | 3 成果物の出典ブロックに components.md を追加し、この Unit の実装が属するコンポーネントを現行名 `CommandGateways` で示す。旧名 `PersistenceGateways` を新たに書かない。 | New |
| R-11 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/functional-design/functional-spec.md > 全体（第 1〜8 節） | ステージ定義 `.claude/aidlc-common/stages/construction/functional-design.md` の `outputs` は「functional-spec.md is the source of truth for workflows and state machines and carries derived ER-diagram and rules-summary views」と定めるが、functional-spec.md には mermaid の ER 図が無く（3 成果物を mermaid / erDiagram / classDiagram / stateDiagram で検索して 0 件）、規則の要約ビューも無い。規則要約は rules.md 第 2 節に置かれており、情報自体は失われていないが、review_artifact がステージの定める形を満たしていない。required-sections センサーは雛形が未配備のため、この欠落を検出しない。 | functional-spec.md に、entities.md の正本 YAML から導いた ER 図（mermaid、テキスト代替を併記）と、rules.md の 23 規則から導いた規則要約ビューを追加する。正本は entities.md / rules.md のままとし、functional-spec.md 側は派生ビューとして重複を明示する。 | New |
| R-12 | Minor | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/functional-design/functional-spec.md > 第 7 節（WorkflowDefinitionRepositoryImpl の横断確認） | 「共通契約 14 件と実装固有 12 件の計 26 件」という実測値が現行 HEAD で再現しない。`cargo test --locked -p core-command-interface-adapter --test workflow_definition_repository_contract --test workflow_definition_repository_impl_test` の実測は 18 + 12 = 30 件である。原因は記載の誤りではなく鮮度で、同じ 2026-09-05 の後続コミット a1ddb37d（#110）が契約マクロへ `find_for_intent_returns_the_current_definition` と `find_for_intent_reports_the_missing_definition` の 2 名を追加し、2 バックエンド分 4 件増えたためである（記載時点は 7 名 × 2 = 14 件で正しかった）。U3 自身の「22 + 23 = 45 件」は現在も一致する。 | 第 7 節の Definition 側の件数を 18 + 12 = 30 へ更新するか、測定した時点のコミットを併記して、以後の増減で読み手が誤らないようにする。 | New |
| R-13 | Minor | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/functional-design/traceability.json > coverage・reverse | 同一ファイル内で BR5.1 と BR5.2 の扱いが矛盾する。`coverage` は FR1 の target を BR5.1、NFR3 の target の 1 つを BR5.2 としているのに、`reverse` は同じ BR5.1 を「要求 ID を持たない是正」、BR5.2 を「横断の検証規則」として status `N/A` に置いている。要求から引かれている規則を、同時に要求 ID を持たない規則として登録している。センサーは coverage 側に載っているため orphan として検出しない。 | BR5.1 と BR5.2 を `reverse` の N/A 一覧から外すか、`coverage` の target を別の規則へ差し替えて、1 つの規則が同時に「要求に対応する」と「要求を持たない」の両方に登録されない形へ揃える。 | New |
| R-14 | Minor | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/functional-design/functional-spec.md > 第 7 節の検証表 | unit-of-work.md の U3 合格条件は 3 つ（journal_protocol.qnt の ITF 準拠、store → find_by_id ラウンドトリップ、クラッシュ再構成テスト = NFR3 の書く側）だが、第 7 節の検証表は前 2 つに対応する行を持つ一方、クラッシュ再構成に対応する行が無い。当該テストは `modules/app/aidlc/tests/crash_reconstruction_test.rs`（先頭コメントは「クラッシュ再構成 (BR5.2 (a))」、テスト 11 件）として実在し、Repository と JournalReader の双方を合成ルートで駆動している。合格条件の 1 つについて、確認済みかどうかが表から読めない。 | 第 7 節の検証表に crash_reconstruction_test.rs の行を追加し、NFR3 の書く側の確認内容と範囲を記す。なお同ファイルが参照する「BR5.2 (a)」は rules.md の BR5.2 に対応する枝番が無いため、参照先を現行の規則番号へ合わせる。 | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| aidlc-sensor-required-sections.ts（--stage functional-design、entities / rules / functional-spec の各 --output-path） | PASS 3 件（H2 数 3 / 2 / 8、findings 0） | 雛形が未配備のため見出しの有無だけを見る。R-11 の派生ビュー欠落はこの検査の対象外である。 |
| aidlc-sensor-traceability.ts（--output-path traceability.json） | FAIL: `missing_from_upstream_ids` 35 件のみ。gaps / orphans / missing_from_table / invalid_entries / invalid_targets はすべて空 | 35 件は他 Unit 担当の FR で既知のノイズ。前回 2 件あった orphan（BR1.7 / BR5.3）は解消し、親 FR1 が upstream_ids に入って 36 件から 35 件に減った。R-08 の解消根拠。 |
| aidlc-sensor-upstream-coverage.ts（--consumes unit-of-work,unit-of-work-story-map,requirements,components,contract-summary） | FAIL: `unreferenced: ["components"]` | 前回 FAIL の unit-of-work-story-map は解消し、未参照は components だけになった。既定重大度は advisory。R-10 の根拠。 |
| cargo test --locked -p core-command-interface-adapter --test intent_execution_repository_contract --test intent_execution_repository_impl_test | PASS: 22 + 23 = 45 件、失敗・無視 0 | 第 7 節の U3 件数主張と一致。契約は 11 名 × memory / sqlite の 2 バックエンド。`an_event_from_another_execution_is_rejected_before_writing` の実在を確認（R-07）。 |
| cargo test --locked -p core-command-interface-adapter --test workflow_definition_repository_contract --test workflow_definition_repository_impl_test | PASS: 18 + 12 = 30 件、失敗・無視 0 | 第 7 節の「14 + 12 = 26」と一致しない。git show a1ddb37d で契約マクロへ 2 名追加を確認。R-12 の根拠。 |
| 現行ポート・エラー型・集約 API の静的照合（intent_execution_repository.rs:71-98、repository_error.rs:27-56、intent_execution.rs:199・254・352-361） | 一致 | 2 引数 store、`&mut self`、`RepositoryError<Id>` の 4 変種、`UNPERSISTED_VERSION = 0`、`with_version`、`replay(snapshot, (usize, DateTime<Utc>, event))` を確認。所見 1・2・3 と R-04 の解消根拠。 |
| 実装の静的照合（intent_execution_repository_impl.rs:61・147・182・331-438・441-478、snapshot_strategy.rs:19-35） | 一致 | manifest `intent-execution-event/1`、open / in_memory、find_by_id の 4 検査と MissingSnapshot 判定、書込前 ID 照合、`seq_nr == 1 \|\| wants_snapshot` 分岐、既定 10 を確認。R-05・R-06・R-07 の解消根拠。 |
| 依存ピンと DTO 所在（Cargo.toml:119、interface-adapter/Cargo.toml:24、orchestration/dto/） | 一致 | `event-store-adapter-rs = "=3.0.0"`、`features = ["sqlite"]`、intent_execution_dto.rs を含む DTO 群の所在を確認。第 4 節の記述と矛盾しない。 |
| RMU 側契約（journal_reader.rs:77・85・109-114、journal_read_error.rs:28-50、global_seq_nr.rs:14-18） | 一致 | JournalReader の抜粋 3 署名、`JournalReadError { Io, Corrupt, CheckpointRegression }`、`GlobalSeqNr(u64)` と `ZERO` を確認。書込側の RepositoryError と別契約であるという第 2 節の記述も成立する。 |
| 形式検証（formal/orchestration/journal_protocol.qnt:196-223 と 226 以降、ls formal/orchestration/） | 一致 | 不変条件 8 本と witness 4 本（w_conflict / w_crash_then_catchup / w_interleaved_writers / w_idempotent_catchup）を確認。`snapshot_tracks_journal = snapSeq == journalLen` は 203 行。audit_lock.qnt は不在で退役済み。Quint ソルバー自体は今回未実行。 |
| ITF 適合テストの実在（modules/app/aidlc/tests/journal_protocol_conformance.rs:226-227） | 実在を確定 | 前回「所在未確認」とした再生先はここにあり、`SnapshotStrategy::every(NonZeroUsize::new(1))` を明示している。BR3.3 / BR3.5 / 第 5 節の記述と一致。 |
| linter / type-check センサー | 対象外・未実行 | 成果物に TS / JS の出力もスニペットも無い。 |

### Summary

前回の Critical 1・Major 4・Minor 2 はすべて現行コードとの照合で解消を確認し、ポート署名・エラー型・保存分岐・再構成手順・破損分類は設計本文と実装が行単位で一致する。実装は 45 件のテストが緑で、開発者が本書から実装を起こすのに追加の設計判断を要しない。残る所見は形式と記録の鮮度に限られ、最も重いのは review_artifact がステージの定める ER 図と規則要約の派生ビューを持たないこと（R-11）、次いで required な上流 components.md の未参照（R-10）、実測件数の陳腐化（R-12）、traceability の自己矛盾（R-13）、合格条件 1 つ分の検証行の欠落（R-14）である。Critical 0・Major 1 のため判定は READY とし、承認ゲートでは R-11 と R-10 を優先して重み付けされたい。
