# unit-of-work-story-map — FR → Unit 対応表

> Units Generation（Inception 2.7）成果物。`user-stories` は Skip（`../user-stories/user-stories-assessment.md`
> — developer tooling）のため、ユーザーストーリー（US ID）の代わりに `../requirements-analysis/requirements.md`
> の **FR / NFR ID** を実装 Unit に対応づける。Unit 定義は `unit-of-work.md`、依存は `unit-of-work-dependency.md`、
> コンポーネントの出典は `../domain-design/components.md`、裁定は `../domain-design/decisions.md`。
> 機械可読の対応は `traceability.json`（各 ID の target は本表の Unit ID と一致）。

## 1. FR → Unit

| ID | 要求（要旨） | Unit ID | Directory | 備考 |
|---|---|---|---|---|
| FR1 | 監査台帳（イベントジャーナル）と audit-first 結合 | U3 | `u3-event-store-repository` | 親 ID。子は FR1.1 → U4、FR1.2/1.3 → U3 |
| FR1.1 | 監査シャードの投影・位置付き横断読取 | U4 | `u4-read-model-updater` | 台帳本体（ジャーナル）は U3 |
| FR1.2 | audit-first × SQLite Tx + 楽観 version、改訂 `audit_lock.qnt` | U3 | `u3-event-store-repository` | |
| FR1.3 | `WorkflowExecutionRepository`（store / find_by_id） | U3 | `u3-event-store-repository` | trait はユースケース層、Impl は U3 |
| FR2 | report ユースケース | U5 | `u5-report-use-case` | 親 ID |
| FR2.1 | 遷移コミット（approve / reject / revise / skip / awaiting / resumed） | U5 | `u5-report-use-case` | |
| FR2.2 | report_dispatch + B10 述語 + verification 最小面 | U5 | `u5-report-use-case` | |
| FR3 | next ユースケースと continue | U6 | `u6-next-continue-use-case` | 親 ID |
| FR3.1 | 21 分岐ラダー | U6 | `u6-next-continue-use-case` | 判断本体（`next_decision`）は U2 の集約 |
| FR3.2 | load-steering 分割配信・continue_token・continue | U6 | `u6-next-continue-use-case` | 正準化は U1 |
| FR3.3 | next_decision の層配置（ADR-002 準拠のレビュー確認） | U6 | `u6-next-continue-use-case` | 実装は U2、確認は U6 の Bolt で |
| FR4 | マルチコール CLI と文言カタログ配線 | U7 | `u7-cli-dispatcher-hooks` | 親 ID |
| FR4.1 | ディスパッチャ（ROUTES） | U7 | `u7-cli-dispatcher-hooks` | |
| FR4.2 | 逐語文言の CLI 出力面配線 | U7 | `u7-cli-dispatcher-hooks` | |
| FR5 | 最小フック 4 本 | U7 | `u7-cli-dispatcher-hooks` | 親 ID |
| FR5.1 | Stop forwarding loop フック | U7 | `u7-cli-dispatcher-hooks` | |
| FR5.2 | HUMAN_TURN 記録フック | U7 | `u7-cli-dispatcher-hooks` | |
| FR5.3 | state-transition guard | U7 | `u7-cli-dispatcher-hooks` | |
| FR5.4 | write-audit-log | U7 | `u7-cli-dispatcher-hooks` | 監査行の描画側は U4 |
| FR6 | doctor とドッグフード | U8 | `u8-doctor-dogfood` | 親 ID |
| FR6.1 | `--doctor` サブセット | U8 | `u8-doctor-dogfood` | |
| FR6.2 | 実地スモーク・Issue #7 close | U8 | `u8-doctor-dogfood` | |
| FR7 | canon-json と 0b ゴールデン | U1 | `u1-canon-json-goldens` | 親 ID |
| FR7.1 | hash-canonical 受入表の採取 | U1 | `u1-canon-json-goldens` | |
| FR7.2 | CLI 実行出力ゴールデンの採取 | U1 | `u1-canon-json-goldens` | |
| FR7.3 | `canon-json` クレート実装 | U1 | `u1-canon-json-goldens` | |
| FR8 | 設計監査の土台整備 | U9 | `u9-canon-docs` | 親 ID。子は FR8.1/8.2 → U9、FR8.3/8.4 → U2 |
| FR8.1 | A 束: canon 語彙の自己矛盾修正 + store 注記 + 旧称除去 | U9 | `u9-canon-docs` | |
| FR8.2 | B 束: 仕様の canon 追従 | U9 | `u9-canon-docs` | |
| FR8.3 | PlanAction の完全移動（R1） | U2 | `u2-domain-es-core` | 再輸出なし（ADR-005 改訂） |
| FR8.4 | 有効プラン畳み込みの集約メソッド化（R2） | U2 | `u2-domain-es-core` | |
| FR9 | CI・ガバナンス整備 | U10 | `u10-ci-governance` | 親 ID。子は FR9.1〜9.5 → U10、FR9.6 → U9 |
| FR9.1 | branch protection | U10 | `u10-ci-governance` | |
| FR9.2 | サプライチェーン 4 件 | U10 | `u10-ci-governance` | |
| FR9.3 | tools/lint の CI 3 ステップ | U10 | `u10-ci-governance` | |
| FR9.4 | PBT シード固定・相対ゲート 0.01 | U10 | `u10-ci-governance` | |
| FR9.5 | カバレッジ除外（composition root） | U10 | `u10-ci-governance` | |
| FR9.6 | エラーハンドリング様式規則の起草 | U9 | `u9-canon-docs` | 文書のみ |

## 2. NFR → Unit

| ID | 要求（要旨） | Unit ID | Directory | 備考 |
|---|---|---|---|---|
| NFR1 | upstream 互換（D6 範囲） | U7 | `u7-cli-dispatcher-hooks` | 横断: U1（正準化・ゴールデン）、U4（監査行・状態ファイル）でも検収。主担当は最終の互換面 U7 |
| NFR2 | 品質ゲート維持（CI 3 ジョブ・カバレッジ 90%・TDD） | U10 | `u10-ci-governance` | 全コード Unit が遵守、機械強制は U10 |
| NFR3 | 監査完全性（ジャーナルから再構成・投影で再生成） | U3 | `u3-event-store-repository` | 書く側 U3、描く側 U4 |
| NFR4 | セキュリティ / サプライチェーン | U10 | `u10-ci-governance` | |
| NFR5 | 性能（非目標の明示） | — | — | N/A — Unit 割当なし |

## 3. 複数 Unit にまたがる要求（横断）

- **FR1**: ジャーナル（U3）と投影（U4）に分かれる。親 ID の主担当は U3。
- **FR5.4**: フックの発火・引数処理（U7）と監査行の描画（U4）。主担当は U7。
- **FR8**: 文書（U9）とコード（U2）。主担当は U9。
- **NFR1 / NFR3**: 上表の備考どおり。

## 4. 各 Unit 内の実装順（Unit 内の FR の並び — Unit 間の順序ではない）

| Unit | Unit 内の順 |
|---|---|
| U1 | FR7.1 受入表採取 → FR7.3 canon-json 実装（受入表で red → green）→ FR7.2 CLI ゴールデン採取 |
| U2 | FR8.3 PlanAction 完全移動 → ドメインイベント語彙と decide/apply（ADR-002）→ FR8.4 畳み込み移設 → `engine_loop` ITF 再確認 |
| U3 | InMemory Repository → EventStore trait + SQLite 実装 → `WorkflowExecutionRepositoryImpl`（FR1.3）→ ロック退役 + `audit_lock.qnt` 改訂（FR1.2） |
| U4 | チェックポイント + ジャーナル差分読取 → 状態ファイル投影 → 監査シャード投影（FR1.1）→ 位置付き横断読取 → クラッシュ再構成テスト（NFR3） |
| U5 | FR2.1 遷移コミット（InMemory でテスト）→ FR2.2 B10 述語・verification 最小面 |
| U6 | FR3.1 ラダー（U2 の `next_decision` に委譲）→ FR3.2 load-steering / continue_token / continue → FR3.3 レビュー確認 |
| U7 | FR4.1 ROUTES + composition root → FR4.2 文言配線 → FR5.1〜5.4 フック 4 本（サブコマンド）→ ゴールデン突合 |
| U8 | FR6.1 doctor → FR6.2 実地スモーク → Issue #7 close |
| U9 | FR8.1 → FR8.2 → FR9.6 |
| U10 | FR9.2（toolchain 固定・forbid 昇格・permissions・cargo audit）→ FR9.3 → FR9.4 → FR9.5 → FR9.1 branch protection |

## 5. カバレッジ検証

- 要求側: FR1〜FR9（親 9 + 子 29 = 38 件）と NFR1〜NFR5（5 件）の **43 ID** をすべて列挙し、NFR5（非目標）を
  除く 42 件に Unit を割り当てた。未割当の FR は無い。
- Unit 側: U1〜U10 のすべてが少なくとも 1 つの FR/NFR を持つ。FR を持たない Unit は無い。
- 機械可読の対応は `traceability.json`（`coverage[].target` = 本表の Unit ID）。
