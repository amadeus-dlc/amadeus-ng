# U4 functional-design — pending-revision（ステージゲートで処理）

## 2026-09-07 再走（Modify）— 独立レビュー iteration 2（NOT-READY）の所見と適用文面

> 2026-09-07 の再走で成果物 4 本を現行コード `b9be20f6`（`main` #120。実測時の作業ツリー `52fce820` と RMU クレートは同一バイト）へ追従させ（G-1〜G-8、`gap-measurement-20260907.md`）、
> 独立レビュー（advisory、エンジン採番 iteration 2、`Recovery: stale-receipt`、2026-09-07T08:50:32Z）は **NOT-READY**（Critical 1 / Minor 5）。旧 Review 節は
> `review-history-20260905.md`、今回の Review 節は `functional-spec.md` 末尾（凍結）と `review-history-20260907-iter2.md`（複製）にある。
> 所見 6 件はコンダクタが 1 件ずつ現行コードで再実測して**すべて有効**と確認した。是正を適用しようとしたところ、review-freeze フックがネイティブ編集を拒み、
> エンジンは「回復レビュー枠は 2026-09-05 受領の失効分（今回の iteration 2 自体）で消費済み」と 2 度目の要求を拒否した（redo jump は unit-major で全 Unit の
> functional-design 受領を失効させるため採らない）。よって成果物 4 本は終端受領時のバイト列へ**復元済み**（sha256 一致を確認）で、是正は本ファイルと
> `revised-20260907/`（適用済みの改訂候補 4 本）に確定文面として置き、ゲートの Request Changes 経路で適用する。

| ID | 重大度 | 場所 | 所見（再実測の根拠） | 適用文面（`revised-20260907/` に適用済み） |
|---|---|---|---|---|
| R-01 | Critical | functional-spec W2 手順 1 / §4 状態表 prepared→publishing 行、rules BR3.4.logic、entities 排他制約 / 末尾段落（計 5 か所） | 本再走の G-3 が「Tx 1 と Tx 2 の間にファイル適用を Tx 外で行う」と書いたが逆。`publication_store.rs:397` の Tx 2（`BEGIN IMMEDIATE`）が再検査 :401-417 → `saved.apply()` :418 → `advance_on` :419 → commit :437。`PublicationBatch::apply` の製品コード呼出は :418 の 1 か所 | 「Tx 2 は pending 行の request_id・確定位置・共有 head・target_binding を**先に再検査**し、そのあと同じ Tx の中でファイルを適用（`PublicationBatch::apply`）し、`advance_on` と committed の確定まで書込ロックを保持して commit する。Tx 1 と Tx 2 の間に別の書き手が完了・置換・前進させていれば再検査で古い計画では書かない」。§4 の実現列は「`publish_prepared` の Tx 2 が再検査のあと同じ Tx 内で `PublicationBatch::apply` を実行する」。gap-measurement §1 / G-3 は訂正済み |
| R-02 | Minor | functional-spec W7 冒頭 | 「`publication_recovery_contract` 12 件」は grep の出現数。`#[tokio::test]` 33、`resolve_publication` 呼出 8 か所 = 7 関数 | 「契約テスト（`publication_recovery_contract` の 33 件のうち `resolve_publication` を呼ぶ 7 件）で検収する」 |
| R-03 | Minor | 3 文書の `HEAD \`52fce820\`` 11 か所 | `git merge-base --is-ancestor 52fce820 origin/main` は偽（squash 前のブランチ側 SHA） | `\`b9be20f6\`` に置換し、§1 / rules 冒頭 / entities 冒頭に「`main` の #120 squash コミット。実測時の作業ツリー `52fce820` と RMU クレートは同一バイト」の等価注記 |
| R-04 | Minor | traceability.json FR1.1 | BR4.2 の `source` に FR1.1 があるが FR1.1 の target に無い（BR 17 本の source と coverage 4 件の全数突合で唯一の不一致） | FR1.1 の target に `BR4.2` を追加 |
| R-05 | Minor | rules BR2.1.logic、entities `AuditBlock.fields` | `in_document_order`（`projection.rs:1806-1812`）は `plan.stages()` を走査して絞るため、計画に無い slug は監査行から脱落する（doc コメントにも明記） | 両所に「計画に含まれない slug は監査行へ写さない」を追記 |
| R-06 | Minor | functional-spec §5 の `PlanUnavailable` 説明、W6 手順 2 | 返却は `read_model_updater.rs:319`（`resolve_plan` 失敗）と `:155`（保存済み計画の終点まで履歴が届かない）の 2 か所 | §5 に 2 か所を明記、W6 手順 2 を「保存済み計画の終点まで届かなければ `PlanUnavailable`、アンカー不一致なら `Corrupt`」に書き分け |

付随する訂正（成果物ではないため適用済み）: `gap-measurement-20260907.md` §0 基準（squash 後 SHA）・§1 排他行・§1 呼出元行・§3 G-3。`functional-design-questions.md` の再走前提 P3 は
「中間のファイル適用は Tx 外で行い」と書いており R-01 と同じ誤り — 質問票は summary-confirmation の受領（SHA-256）が束縛するため書き替えず、本行で訂正を記録する
（正: Tx 2 が再検査のあと同じ Tx 内でファイルを適用する）。

**折り戻し済み**: RECOMPOSED の列挙順（文書順）の契約明記は `../../../inception/contract-design/pending-revision.md` 項目 2 に文面案付きで登録。
**申し送り（U7）**: `JournalReaderImpl::resolve_publication` / `rebuild_read_model` は app 未配線（契約テストのみ）。配線の要否は U7 の再走で裁定する（本再走では裁定を求めない）。
**下流への注記**: U4 の nfr-requirements / nfr-design / code-generation 再走は、凍結版の W2.1 / BR3.4 / entities 排他制約ではなく上表 R-01 の文面（`revised-20260907/`）を前提に読む。

**教訓（§13 候補）**: 再走 Modify で既存受領が失効すると、最初の公式レビューが「回復レビュー」として枠を消費し、その後の produces 是正には 2 度目のレビューが取れない。
再走 Modify では公式要求（`aidlc-log.ts review`）の**前に**受領を伴わないドライレビュー（同じ brief、同じ検証手順）を 1 回通し、所見を反映してから公式要求を出す。
また終端受領後の是正は review-freeze が止める前提で、sed を先に流さない（本件では sed 11 か所が先に通過して受領が失効し、復元作業が必要になった）。

## 2026-09-05〜06 の記録（履歴）


R-01〜R-03に加え、以下のR-04/R-05も2026-09-05の修正・再レビューで解消済みとなった。最終判定はREADY、新規所見なし。以下には修正内容と検収条件を履歴として残す。復旧機構の実装や障害試験が完了したことを意味しない。

## R-04 — 再生成計画の保存順序

対象: functional-spec.md の W6 手順3・4。

反映済み: 手順3で完全な出力の計算、before/afterの同一性、利用者部分の保持を確定し、手順4で新世代を受理して計画を保存する順にした。完全な計画、request_id、active_generationの更新をW1手順8と同じ不可分な操作で受理すると明記した。

検収: 出力計算や同一性の確定に失敗した時点では、新しいprepared計画もactive_generationの変更も観測されない。計画受理後に停止した場合は、保存済みの確定バイトだけでW3から再開できる。

## R-05 — 同一位置の内容比較

対象: rules.md の BR5.3.logic。

反映済み: 有効な共有面について、as_ofがtarget未満なら候補を公開、同値なら候補の行集合との一致を検証して維持、超過なら新しい共有面を維持、と分けた。同値で内容が違う場合は破損として停止する。欠落・破損・規約不一致の既存条件は維持した。

検収: as_of=targetでも候補の内容が既存面と異なるケースは成功しない。functional-spec.mdのW8とBR5.3の判断が一致する。


## 2026-09-06 JST — 実装同期の追記

上記R-04/R-05とREADY判定は2026-09-05の記録として変更していない。以下はその後の実装との同期であり、新しいReview判定ではない。

- 保存済み終点までの復旧を先に確定し、同じ`catch_up`呼出しで追加分を別計画・別Txとして処理する。最大2計画とし、runtime側の駆動ループは追加しない。
- 後続計画が失敗しても旧計画のcommitを保持し、呼出元には失敗を返す。復旧を確認できるまでは通常の指示や変異へ進めない。
- U7の`catch_up_before_reading`から失敗を伝播する。`next` / `resume`はerror directive・exit 0、`report` / `practices_promote` / `set_autonomy`はrefused・exit 1で停止する。
- エラー変換をSQLiteの`at_store`とファイルI/Oの`at_output`へ集約した。比較用SAVEPOINTの失敗、共有行の型破損、古いpredecessor、確定中のhead喪失の契約試験を追加した。

統合版 `9b4a6d55` の全workspaceは2,200件成功、相対カバレッジは未達。その後の最終コード `e1691a53` は相対カバレッジとCIが成功し、mainへ統合済み。対象コミット別の結果は[implementation-report.md](../implementation-report.md)へ統合後に記録する。
