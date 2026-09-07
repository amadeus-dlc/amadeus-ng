# reviewer-brief-2 — U4 Functional Design 再走の回復レビュー（2026-09-07、advisory、エンジン採番 iteration 3）

Conversation language: 日本語

Why now: Re-check after the artifact changed.

> レビュアー `aidlc-architecture-reviewer-agent` への依頼書（2 回目）。**依頼の条件・読まないもの・検証ツール・`## Review` 節の書き方は
> [reviewer-brief-1.md](reviewer-brief-1.md) §A（および §B の規則束）をそのまま適用する。** 本書はその差分だけを書く。
> 前回（iteration 2、2026-09-07T08:50:32Z、NOT-READY: Critical 1 / Minor 5）の `## Review` 節は `functional-spec.md` から切り離して
> [review-history-20260907-iter2.md](review-history-20260907-iter2.md) に保存した。今回の付録はその前置バイト列の末尾に追記する。

## 1. 前回所見への対応（コンダクタが 1 件ずつ再実測して有効と確認し、適用済み）

| ID | 前回の所見 | 適用した是正 |
|---|---|---|
| R-01（Critical） | 「Tx 1 と Tx 2 の間にファイル適用を Tx 外で行う」が逆。`publication_store.rs:397` の Tx 2 が再検査 :401-417 → `saved.apply()` :418 → `advance_on` :419 → commit :437 | W2 手順 1、§4 状態表の prepared→publishing 行、BR3.4.logic、entities の排他制約と末尾段落の 5 か所を「Tx 2 は再検査のあと同じ Tx の中でファイルを適用し、確定まで書込ロックを保持する」に書き替えた。実測記録 gap-measurement §1 排他行 / §3 G-3 も訂正済み |
| R-02（Minor） | W7 の「`publication_recovery_contract` 12 件」が実測と合わない | 「33 件のうち `resolve_publication` を呼ぶ 7 件」に置換（`#[tokio::test]` 33、呼出 8 か所 = 7 関数） |
| R-03（Minor） | `52fce820` は `origin/main` の祖先でない | 3 文書の 11 か所を `b9be20f6`（`main` の #120 squash）に置換し、§1 / rules 冒頭 / entities 冒頭に「実測時の作業ツリー `52fce820` と RMU クレートは同一バイト」の等価注記を残した |
| R-04（Minor） | BR4.2 の source に FR1.1 があるが traceability の FR1.1 target に無い | FR1.1 の target に BR4.2 を追加 |
| R-05（Minor） | `in_document_order` は計画に無い slug を写さない | BR2.1.logic と entities `AuditBlock.fields` に「計画に含まれない slug は監査行へ写さない」を追記 |
| R-06（Minor） | `PlanUnavailable` は `read_model_updater.rs:155`（保存済み計画の終点まで履歴が届かない）でも返る | §5 の分類説明に 2 か所（:319 / :155）を明記し、W6 手順 2 の「履歴不足」を `PlanUnavailable` / `Corrupt` で書き分けた |

是正は凍結フック（review-freeze）がネイティブ編集を拒むためスクリプトで適用した（U9 再走と同じ運び）。前回の終端受領はこの書込で失効し、
エンジンが許す**回復レビュー 1 回**が本依頼である。本依頼の受領後は成果物を編集しない（残る所見はすべて pending-revision 経由でゲートへ）。

## 2. 今回とくに確認してほしいこと

1. R-01 の是正 5 か所が互いに矛盾せず、`publication_store.rs:388-440` / `journal_reader_impl.rs:471-541` の実体と一致するか。「Tx 1 と Tx 2 の間」に
   在るのは pending 行だけである、という記述が正しいか（`publish` :373-386 に他の処理が無いか）。
2. R-02〜R-06 の適用文面が実測値と一致するか（件数・行番号・変種名を再実測）。
3. 前回 READY 相当と評価した点（G-1 / G-2 / G-4〜G-8、`CatchUpError` 14 変種、RECOMPOSED 順序の上流沈黙と contract-design への折り戻し）に
   今回の是正で退行が無いか。
4. 新規の所見があれば新 ID（R-07〜）で報告する。前回 ID は「対応済み / 未対応」の判定を Status 列に書く（Resolved / Open）。

## 3. `## Review` 節の追加条件

- `**Iteration:** 3`。`**Request Challenge:**` 行は、依頼プロンプトで値が渡された場合のみその値を書く（渡されなければ書かない）。
- 所見表には前回 ID（R-01〜R-06）の再判定行を含め、Status を `Resolved` または `Open` にする。新規所見は `New`。
- その他は reviewer-brief-1.md §A.6 と同じ。
