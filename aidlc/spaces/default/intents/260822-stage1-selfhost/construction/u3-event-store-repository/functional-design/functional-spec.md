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

**Verdict:** NOT-READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-05T07:08:22Z
**Iteration:** 1

### Findings

entities.md の旧番号 1〜3 を保持し、旧レビュー本体は変更していない。自前 SQL・ローカル EventStore・within_write_transaction・旧ワイヤ形式は、既存の失効注記どおり履歴として評価した。以下の新規所見は、失効していないとされる記述、後継として提示された記述、または現行契約への参照不足に対するものである。

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| 1 | Critical | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/functional-design/rules.md > BR1.2・BR1.3・BR2.3 | 旧所見の version−1 算出は撤回済み。現行 store は aggregate.version() を期待値として渡し、genesis の version 0、更新成功、古い版の競合拒否は両バックエンドの契約テストで成功する。旧アンダーフロー・常時偽競合の原因は解消している。 | 追加対応なし。版を seq_nr から導く旧改善案は再採用しない。 | Resolved |
| 2 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/functional-design/entities.md > EventStore、および rules.md > BR1.1 | ローカル trait と u64 化は明示的に失効し、本家の usize に復帰した。現行 Repository ポートと実装の版・通番も usize。旧所見の型不一致は解消している。 | 追加対応なし。借用した契約の型を変更する旧案は再採用しない。 | Resolved |
| 3 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/functional-design/entities.md > WorkflowExecutionRepositoryImpl、および functional-spec.md > 第 2 節 | store は設計・現行ポート・実装とも &mut self であり、ストアを直接保持する。&self から &mut self を呼べないという旧問題はなく、内部可変性を追加する必要もない。 | 追加対応なし。 | Resolved |
| R-04 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/functional-design/functional-spec.md > 第 1・2 節、および entities.md > Repository・JournalReader・RepositoryError | 後継として示す第 2 節は RehydratedWorkflowExecution と裸の expected_version 引数を現行形とするが、現行 IntentExecutionRepository は IntentExecutionId で集約そのものを返し、store(event, aggregate) が集約の版を使う。RepositoryError は ID 型を引数に取る共通型で、CorruptCause の公開分類はなく source 連鎖。JournalReader 一式の所有も、共有 C3 の B8 注記では RMU へ移動済みなのに本書の use-case / adapter 配置表は更新されていない。消費側が使う署名・依存先を一意に決められない。 | 後続裁定に沿って現行ポート・ID・版の持ち回り・エラー形・所有クレートを同期する。古い C3 の署名との差も明示し、読み手に実装からの推測を要求しない。 | New |
| R-05 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/functional-design/functional-spec.md > 第 2 節・第 3.1 節・第 5 節、および rules.md > BR3.3 | 後継 store 手順は常に persist_event_and_snapshot を呼ぶとし、JournalProtocolModel は snapSeq == journalLen を約束する。一方、現行 SnapshotStrategy の既定は 10 イベントごとで、初回以外は persist_event 経路がある。既存テストでは N=2 の seq_nr=3 で snapshot.seq_nr=2、version=3、既定設定でも 3 イベント後の基底は seq_nr=1。差分再生は成功するが、「毎 store 更新」のモデルを既定実装全体の保証として扱えない。 | 初回必須・設定間隔・イベントのみ保存の分岐と再構成への影響を設計へ反映する。モデルの保証を毎回更新設定に限定するのか、間欠更新へ拡張するのかを明示し、対応する検証根拠を付ける。差分再生という確定方針自体は問い直さない。 | New |
| R-06 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/functional-design/functional-spec.md > 第 3.2 節・第 7 節、および rules.md > BR1.2 | 第 3.2 節の後継説明は snapshot 不在を無条件 NotFound とし、MissingSnapshot を判別できないとするが、BR1.2.logic と現行実装は journal が残れば Corrupt とする。さらに現行は基底の DTO 復元後に base.seq_nr+1 からだけ読み、差分の通番・manifest・aggregate_id を検査する。旧 from_state / serde-memento を前提とした説明からは、基底以前を検査しない範囲、Corrupt とドメイン再生時のクラッシュの境界が読めない。 | 最新スナップショット＋後続差分を軸に、第 3.2 節と BR1.2 の欠落・復号・差分検査・失敗分類を統一する。journal 全削除時に古い基底が読めるという実測を、欠落許容の新たな承認とは扱わない。 | New |
| R-07 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/functional-design/functional-spec.md > B7 追加失効注記・第 3.1 節 check_preconditions | 「イベントが識別子を持たないため別集約のイベントを渡せず、identity 検査は型で不要になった」という根拠が失効している。現行 IntentExecutionEvent は aggregate_id を持ち、store の event と aggregate は独立した参照引数なので、別実行から得た同型イベントを組み合わせることを型は防がない。現行 envelope は集約から外側 ID、イベントから payload を別々に組み、書込前に ID 照合を行わない。これは設計の型保証主張が成立しないという静的所見であり、不整合な対の保存実験は今回行っていない。 | 型保証という説明を撤回し、同じコマンドから得たイベントと適用後集約を渡す責任と保証箇所を明記する。書込境界の検査を要するかは、不整合な対の受入テストで現挙動を確認してから判断する。 | New |
| R-08 | Minor | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/functional-design/rules.md > BR1.2・BR1.3、および traceability.json > upstream_ids・coverage | BR1.7 と BR5.3 を根拠として参照するが、この Unit の規則定義には存在しない。センサーはこの 2 件を orphan と報告したが、実態は未解決の規則参照である。また story-map では親 FR1 の主担当が U3 なのに対応表にはなく、成果物には unit-of-work-story-map への参照もない。 | 他 Unit の規則を意図するなら共有契約等の正確な出典へ置き換える。親 FR1 の文書上の集約対応と、要求割当表への参照を追加する。FR1.1 の実装担当を U3 に移さない。 | New |
| R-09 | Minor | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/functional-design/entities.md > IntentDirName、および rules.md > BR4.2 | pending-revision 項目 2 の正規表現訂正が未反映。設計の式は 260822-a--b を受理するが、現行 IntentDirName::parse は空区間を拒否し、doc も連続ハイフンを認めない式を示す。 | 既存の改訂候補に沿って、entities と BR4.2 の正規表現を空区間拒否の形へ揃える。 | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| aidlc-sensor-required-sections.ts（--stage functional-design、各 --output-path） | PASS: entities / rules / functional-spec、所見 0 | 追記前の H2 数は 3 / 2 / 8。内容の世代差は検査しない。 |
| aidlc-sensor-upstream-coverage.ts（consumes 5 件、deliverables 3 件を明示） | FAIL: unit-of-work-story-map が未参照 | 質問票の参照は成果物側の参照を代替しない。R-08。 |
| aidlc-sensor-traceability.ts | FAIL: missing_from_upstream_ids 36 件、orphans 2 件。他の gaps / missing_from_table / invalid_entries / invalid_targets は空 | 36 件中、親 FR1 は U3 主担当、残り 35 件は担当外。orphan 表示の BR1.7 / BR5.3 は定義ではなく本文中の未解決参照。R-08。 |
| linter / type-check の適用判定 | 対象外・未実行 | 成果物に TS/JS/TSX コード出力や該当スニペットはない。 |
| cargo test --locked -p core-command-interface-adapter --test intent_execution_repository_contract --test intent_execution_repository_impl_test | PASS: 20 + 23 = 43 件、失敗・無視 0 | memory / SQLite の正常保存・競合・再読取、間欠スナップショット、差分欠落・foreign manifest・別実行 ID の拒否等を再実行。試験が成功している実装を旧設計へ戻す理由にはならない。 |
| 現行ポート・実装・SnapshotStrategy・journal_protocol.qnt の静的照合 | 差異を確認 | R-04〜R-07 の根拠。モデルは毎回更新を仮定し、実装は間欠更新を持つ。不整合な event/aggregate 対の保存後の挙動は未実測。 |
| ITF 準拠・Quint ソルバー | 今回未実行 | 旧設計が挙げる再生先は移動しており、確認した command-interface-adapter と read-model-updater の直接候補パスには journal_protocol_conformance.rs がなかった。現行の所在は未確認で、テスト不存在や不合格とは断定しない。過去の「モデルは変更せず通った」を現在の証明に流用しない。 |
| 旧所見と失効注記の照合 | 旧 1〜3 は Resolved | 自前 SQL の算出式・u64 化・内部可変性の追加案は再適用しない。within_write_transaction の代替未決という履歴も、実装済み／未実装を今回新たに断定する根拠には用いていない。 |

### Summary

旧 Critical は解消済みだが、未解消の Major 4・Minor 2 のため ADVISORY 判定は NOT-READY。Repository 関連の 43 テストは成功している。主な問題は、現行の署名・所有・更新頻度・破損判定へ追従していない設計記録と、現在は成立しない型保証の説明であり、未実測の保存経路を実装不具合と断定してはいない。
