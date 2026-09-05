# U3 設計不整合の是正報告

2026-09-05。対象は `functional-design/entities.md`、`rules.md`、`functional-spec.md`、`traceability.json` の現行本文。
過去の Review は当時の所見・検証・判定として保持した。正式なステージ、レビュー、質問、承認、レシート、audit、state、memory の操作は行っていない。

## 是正の根拠と対応

正本の `coding-rules/README.md` と gateway-taxonomy / upstream-contracts / domain-persistence-neutrality / error-handling / cqrs-boundaries、
C3/C6 の後続裁定、現行ポート・実装・DTO・試験を照合した。
「最新スナップショットとそれ以降の差分イベント集合でリプレイする」という確定方針を設計の中心に据えた。

| 所見 | 本文の是正 | 根拠・限界 |
|---|---|---|
| R-04 | IntentExecutionRepository と IntentExecutionId、集約が版を運ぶ2引数store、RepositoryError<Id>とsource連鎖へ同期。JournalReader一式をRMU所有として記載 | command/use-case のポートに記録された2026-08-30の版所有裁定と command/interface-adapter の実装、C3 B8/B13、RMUの現行trait。版所有の根拠をB13とはしない。旧署名からの読み替えを要求しない |
| R-05 | 初回必須、既定10、間隔N、イベントのみ保存を明記。QuintのsnapSeq==journalLenをevery(1)限定とした | SnapshotStrategy、実装の分岐、app/aidlc/tests/journal_protocol_conformance.rs の every(1)。間欠更新の全般的なモデル証明を主張しない |
| R-06 | 基底なしの場合のNotFound/Corrupt、DTO復元、base.seq_nr()+1包含下限、差分検査、domain replayクラッシュ境界を統一 | intent_execution_repository_impl_test.rs の基底欠落・破損・差分・基底以前未読試験。journal全削除後の基底読取を削除許可に読み替えない |
| R-07 | 同型の別実行イベントは構成可能と訂正。store入口のID照合でCorrupt(source=WriteContract)を返す保証へ同期 | 親担当の追加実測でmemory/SQLiteの双方が不整合Startedを保存した。最小修正後は新規・更新時の拒否を共通契約で固定。同一IDで状態とイベント内容が意味的に一致することまで型やID検査が保証するとはしない |
| R-08 | 未定義のBR参照を明示的な出典に置換。親FR1をupstream_ids/coverageへ追加しstory-mapを成果物に参照 | unit-of-work-story-mapの割当。FR1.1はU4実装担当のまま。過去Review本文の旧BR参照は履歴として変更しない |
| R-09 | IntentDirNameを `^[0-9]{6}-[a-z0-9]+(?:-[a-z0-9]+)*$`、64字以下に統一 | 現行parseは空区間を拒否する。entitiesのYAML、rulesのYAMLと要約を同期 |

自前 EventStore / SQL、serde-memento、再水和用の器、全履歴による集約リプレイを復活させていない。
Repository のエラー原因と、RMU の公開 `CorruptCause` を区別した。
旧毎回更新モデルの説明を一般実装の保証から外し、モデルそのものを無断変更していない。

## 検証

| 種別 | 結果 | 証拠と範囲 |
|---|---|---|
| 作業ツリー信頼 | 完了 | `mise trust` は未信頼設定なしと報告 |
| 不整合イベント対の拒否期待（修正前） | 2件失敗 | 親担当実測 `/tmp/verify-this/u3-event-pair-red.log`。別実行Started+対象集約がmemory/SQLite双方でOkとなった |
| Repository試験（修正後） | 45件成功 | 親担当実測 `/tmp/verify-this/u3-event-pair-green.log`。22共通契約 + 23実装固有。追加共通契約はgenesisで双方NotFound、更新で対象の元状態維持を確認 |
| 別系譜Definitionの拒否期待（修正前） | 2件失敗 | 親担当実測 `/tmp/verify-this/u3-definition-pair-red.log`。別系譜Defined+対象definitionがmemory/SQLite双方でOkとなった |
| Definition試験（修正後） | 26件成功 | 親担当実測 `/tmp/verify-this/u3-definition-pair-green.log`。14共通契約 + 12実装固有。同じWriteContractでI/O前に拒否し、新規・更新の状態維持を確認 |
| 型・所有・版・再生経路 | 静的照合 | 現行ポート、DTO、実装、SnapshotStrategy、RMU、Quintモデルとapp側ITFの設定 |
| YAML / JSON / Review保存 | 成功 | PyYAMLでentities/rulesを解析、全23規則の定義・要約一致、未定義BR参照なしを現行rulesで確認。traceabilityの親FR1と子担当、正規表現の受理・拒否例、文字化け・コードフェンスも検査。編集前後のReview全文は同一 |
| 全体検証 | 親担当が実施 | 設計担当はworkspaceテスト、Quint、coverageを重複実行していない。修正前の2135件・coverage98.65%を修正後の証拠として流用しない |

## 残る範囲と判断

- 過去 Review の NOT-READY と各所見の New は変更していない。本文是正は正式な再レビュー・承認を代行しない。
- every(1) の形式モデルと、既定10・任意間隔Nの実装試験は別の検証範囲。一般モデルへ拡張したとは主張しない。
- ID一致の書込検査は別集約イベントの混入を拒否するが、同一IDの任意イベントと集約状態の対応を証明しない。
- ジャーナルの基底以前と欠落した末尾はRepository再構成の独立検査対象ではない。監査要件を緩和する裁定はしていない。
- 横断確認ではWorkflowDefinitionRepositoryImplにも同じID照合欠落が実測され、書込前拒否へ是正された。IntentRepositoryImplは既にCreatedから再構成した対象全体と照合しており、今回の2実装の欠落と混同しない。
