# U4 リードモデル更新 — 機能設計の補完確認

> 2026-09-05 の補完（初回 + レビュー後修正）の確認記録に、2026-09-07 の再走（unit-major 反復、既存成果物 Modify）の確認を追記した。
> 再走の根拠は実測記録 [gap-measurement-20260907.md](gap-measurement-20260907.md)（基準 = 作業ツリー HEAD `52fce820`、RMU クレートは `origin/main` と同一）。

## 根拠と対象

- [Unit 定義](../../../inception/units-generation/unit-of-work.md): U4 の責務、独立クレート、U7 からの起動。
- [要求割当](../../../inception/units-generation/unit-of-work-story-map.md): FR1.1、NFR1/NFR3 の検収面、FR5.4 の監査描画側。
- [要求](../../../inception/requirements-analysis/requirements.md): 監査出力の逐語互換とクラッシュ後の冪等な再生成。
- [構成](../../../inception/domain-design/components.md)、[契約](../../../inception/contract-design/contract-summary.md): 投影の所有と C3/C5/C6 の境界。
- `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/cqrs-boundaries.md`: 後続裁定による取得ループ・純粋投影核・構造化リードモデルの責務。
- 現物: `modules/core/read-model-updater/src/orchestration/read_model_updater.rs` と同クレートの既存テスト。

この Unit は実装と開発報告を持つが、所定の functional-design ディレクトリに必須設計 4 ファイルがない。既存報告の「完了」は、現在の要求との一致を確認する入口として扱う。

## 再開時に確認した差異

2026-09-05、既存取得ループテスト 29 件は成功した。一方、同じ journal・同じチェックポイント・既に書かれた同じ出力ファイルで再実行すると、監査イベント行は 2 行から 4 行に増え、状態ファイルは同一バイトだった。

検証は既存 FakeReader / Fixture を使い、出力を書いた後にチェックポイントを更新できなかった復旧条件を再現した。実プロセスの強制終了や実 DB のクラッシュを起こした試験ではない。一時検証コードは撤去し、コピーと生ログは `/tmp/verify-this/u4-recovery-probe-test.rs`、`/tmp/verify-this/u4-recovery-probe.log`、既存テストのログは `/tmp/verify-this/u4-existing-tests.log` に保持している。

NFR3 の冪等な再生成とこの観測結果には差がある。古い開発報告にある「欠落より重複を許容」という担当者判断だけで、要求を緩めたとは扱わない。

## 2026-09-05 の確認（履歴 — 初回補完とレビュー後修正）

- 補完する成果物は entities.md、rules.md、functional-spec.md、traceability.json。既存コードの作り直しからは始めず、要求・後続裁定・現物の対応を整理する。
- 対象は取得と投影の責務、監査・状態ファイル・構造化リードモデルの出力、チェックポイント前進、初回起動、参照規則の変更、障害後の再実行。コードの偶然の形だけを設計根拠にしない。
- U3 の集約再構成は「最新スナップショットと、その通番より後の差分イベント」が確定済み。U4 の投影データを再生成するための履歴読取とは区別し、この裁定を問い直さない。
- NFR3 の冪等性を維持し、同じ出力先・同じチェックポイントからの再試行で監査行が重複する現状を実装との相違として明記する。状態ファイルだけでなく監査出力も対象にした、障害前後の受入条件を設計する。
- FR1.1 と NFR1/NFR3 を追跡し、FR5.4 の描画側との接点を示す。フック発火側や CLI 全体の責務を U4 へ移さない。
- 古い共有文書と後続裁定の相違は、出典・適用範囲・未解決事項を残す。保存された旧回答や完了報告を無条件に再適用しない。

**レビュー後の修正範囲（2026-09-05）**

- R-01: 通常の差分処理と再生成を分ける。新しい計画世代と要求IDを使い、履歴末尾と確定位置が同じ100でも、100→100の再生成を実行できるようにする。同じ要求の再送は同じ計画へ戻し、通常の確定位置は後退させない。
- R-02: blocked 計画を superseded として終了し、新しい世代の計画へ置換できるようにする。利用者の変更と、現物で確認できた反映済み監査ブロックを引き継ぐ。旧計画の書込権を失効させ、同じ範囲の二重追記を防ぐ。対応を特定できない出力は勝手に完了扱いしない。
- R-03: 同じspaceの共有構造化面に、個別カーソルとは別の公開世代・位置を持たせる。断面120の後に100を公開して後退させない。同じ変換規約の有効な共有面が既に新しければそれを維持し、ファイル側の計画とカーソルを確定して参照した共有世代を記録する。規約不一致や破損は再生成へ回す。
- 3点の対応をエンティティ・規則・手順・状態表・受入シナリオへ一貫して反映し、再レビューする。アプリケーションの実装変更はこの機能設計修正には含めない。

初回の補完方針は Looks correct で確認済み。2026-09-05 の再確認（レビュー所見3点の修正範囲）も Looks correct で確認済み（当時の記録: `[Answer]: Looks correct`）。

## 再走 2026-09-07 — 前提（確認事項）

再走ではオーナー裁定「現状のコードを基準」に従い、現行コードを正として設計を追従させる（gap-measurement §3 の G-1〜G-8）。人間の裁定を要する新しい基盤選択は無い。

- P1. **基準と範囲**: 基準は HEAD `52fce820`（RMU は `origin/main` と同一）。改訂するのは entities.md / rules.md / functional-spec.md の 3 文書。traceability.json は変更起因なし（upstream 4 ID → BR 17 本）。実装・テスト・共有契約本文は変更しない。
- P2. **b51 由来の追従 2 件**: (G-1) `Recomposed` の監査行 `Stages skipped` / `Stages added` は計画の**文書順**で並べる（`in_document_order`、辞書順にしない）— BR2.1 と `AuditBlock.fields` に明記。上流契約（`audit-format.md` / contract-summary の RECOMPOSED）は順序に沈黙しているため、contract-design の pending-revision へ「列挙順 = 文書順」の追記案を折り戻す。(G-2) 構造化面の生成で集約 `next_decision` が別 intent を拒否した判断は `ReadTablesError::IntentUnavailable` として投影不能に分類し、RMU で判断し直さない — §5 の分類と BR2.3 に明記。
- P3. **排他の言い直し（G-3）**: W2 手順 1 / BR3.4 / entities 制約の「照合開始から確定まで排他を保持・ファイルを正準パス順にロック・対象集合単位で直列化」を、現行の**ストア単位の書込 Tx 2 段（prepare / publish_prepared、`BEGIN IMMEDIATE`）+ 確定 Tx での再検査（pending 行・request_id・確定位置・共有 head）による古い書き手の遮断**に改める。ファイル単位のロックは存在せず、ストア（= space）単位のロックが包含する。中間のファイル適用は Tx 外で行い、Tx 2 の再検査が不一致なら古い計画で書かない。
- P4. **状態と属性の実現（G-4 / G-5）**: §4 状態表に「実現」列を足し、`prepared` / `publishing` は `committed=0` で区別しない、`blocked` は永続状態ではなく `CatchUpError::PublicationConflict { path }` の返却（pending 計画は保持）、`committed` / `superseded` は `amadeus_publication.committed` と `amadeus_publication_history.state` と明記。entities「派生表示と実装境界」を属性単位の対応表（論理属性 ↔ 列 / フィールド / 導出）へ拡張し、列の無い `replacement_id` / `resolution` / `inherited_blocks` / `before_identity` / `after_identity` / `audit_blocks` は「導出」または「全バイト」と書く。`ProjectionCursor` に `anchor_aid` / `anchor_seq_nr`、`SharedProjectionHead` に `verified` を追加する。論理モデルの名前（PublicationBatch / OutputPlan / ProjectionCursor / SharedProjectionHead / AuditBlock）は維持する。
- P5. **入口と配線（G-6 / G-7）**: W6 / W7 に入口（`rebuild_read_model` / `restore_missing_files` / `resolve_publication`）と U7 配線の有無を書く（`resolve_publication` と `rebuild_read_model` は app 未配線、契約テストで検収 — 配線は U7 の裁定事項として申し送り、本再走では裁定を求めない）。W8 表の行 4 / 5 を現行の分類名（`Corrupt(ProjectionSnapshotMismatch)` で停止、`prepare_read_model` が入口で旧規約 head を再生成）で書き直す。
- P6. **検証状況と Review の扱い（G-8）**: §7 に 2026-09-07 の実測行（RMU 9 バイナリ 481 件、workspace 2,354 件）を追加し、旧数値は日付付きで残す。旧 `## Review`（2026-09-05 READY）は `review-history-20260905.md` へ退避済みで、再走の独立レビュー（advisory、iteration 1）を新たに受ける。

## Consolidated Summary Confirmation

- 現行コード HEAD `52fce820` を正として、entities.md / rules.md / functional-spec.md を G-1〜G-8 の範囲で改訂する（Modify）。traceability.json・実装・テスト・共有契約本文は変更しない。
- b51 の追従 2 件（RECOMPOSED の列挙順 = 文書順、`IntentUnavailable` の分類）、排他方式の言い直し（Tx 2 段 + 確定 Tx 再検査、ファイルロック無し）、状態 5 値と論理属性の実現の明記（blocked はエラー返却、列の無い属性は導出）、W6 / W7 の入口と配線状況、W8 表の分類名、§7 の 2026-09-07 実測行を反映する。
- 契約側 1 件（RECOMPOSED の列挙順）は contract-design の pending-revision へ折り戻し、U7 側 1 件（`resolve_publication` / `rebuild_read_model` の配線）は申し送りにとどめ、本再走で裁定は求めない。
- 旧 Review 節は `review-history-20260905.md` に退避済み。改訂後にセンサー（required-sections / upstream-coverage）を再実行し、独立レビュー（advisory、iteration 1）を受ける。

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
