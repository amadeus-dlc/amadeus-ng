# review-history-20260905 — functional-spec.md 旧 Review 節の退避

> 2026-09-05T07:08:22Z の独立レビュー（NOT-READY、ID 1〜3 / R-04〜R-09）を、2026-09-07 の再レビュー（iteration 1、READY）追記時に `functional-spec.md` 末尾から切り離して保存したもの。
> 切り離しの理由: レビュー要求は付録前のバイト列（13135 バイト）を束縛し、付録は要求ごとに 1 節だけを許すため（stage-protocol-reviewer 手順 1-2）。
> git 上の原本は `f6726802`（#112）の同ファイル末尾。各 ID の現在の Status は 2026-09-07 の Review 節（全件 Resolved）を参照。
> 本文（functional-spec.md 冒頭・§8）の「末尾の Review は過去の記録として保持する」「過去 Review の判定とステータスは書き換えず」は、本ファイルへの退避を指すものとして読む。

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
