# U4 リードモデル更新 — 機能設計の補完確認

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

## Consolidated Summary Confirmation

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

初回の補完方針は Looks correct で確認済み。今回の再確認はレビュー所見3点の修正範囲を対象とする。

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
