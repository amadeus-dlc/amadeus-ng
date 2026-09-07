# review-history-20260907-iter2 — functional-spec.md の iteration 2 Review 節の退避

> 2026-09-07T08:50:32Z の独立レビュー（advisory、エンジン採番 iteration 2、NOT-READY: Critical 1 / Minor 5）を、所見の是正（R-01〜R-06 を適用）と回復レビュー（iteration 3）の要求に先立って `functional-spec.md` 末尾から切り離して保存したもの。
> 是正内容は reviewer-brief-2.md §1、各所見のコンダクタ再実測は construction/functional-design/memory.md（2026-09-07T08:5x）を参照。

## Review

**Verdict:** NOT-READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-07T08:50:32Z
**Iteration:** 2

advisory（承認判断の参考となる独立レビュー）の 1 回きりのパスであり、修正と再レビューのループは持たない。所見はすべて 2026-09-07 時点の作業ツリーで実測した（`origin/main` = `b9be20f6`、`modules/core/read-model-updater/` は `52fce820` と同一バイト。`git diff --stat 52fce820 HEAD -- modules/core/read-model-updater/` は空）。

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Critical | `functional-spec.md` > §3 W2 手順 1 および §4 状態表の prepared→publishing 行、`rules.md` > BR3.4.logic、`entities.md` > 派生表示と実装境界の末尾段落 | 4 か所が「Tx 1 と Tx 2 の間にファイル適用を Tx 外で行い、そののち Tx 2 が再検査する」と書くが、現行コードは順序も Tx 境界も逆である。`orchestration/publication_store.rs:397` が Tx 2 を `BEGIN IMMEDIATE` で開き、401-417 で再検査（`pending`、`saved != batch`、チェックポイント、`shared_projection::verify`）、418 で `saved.apply()`、419 で `advance_on`、437 で `commit` する。`PublicationBatch::apply` の製品コード呼出は 418 の 1 か所だけで（`grep -rn "\.apply()" modules/core/read-model-updater/src/` の 3 件は 418 と自身の内部 `publication_batch.rs:199`、テスト用 `publication_file.rs:276`）、Tx 1 と Tx 2 の間に適用は存在しない。帰結は 2 つ — ストア（space）単位の書込ロックが全ファイル I/O の間ずっと保持される、および再検査が適用の前であって後ではない。本再走が G-3 で「現行コードへ追従させた」と述べた排他機構そのものが、コードと逆向きに記述されている | W2 手順 1、§4 状態表の当該行、BR3.4.logic、entities 末尾段落を「Tx 2 が再検査してから同じ Tx の中でファイルを適用し、`advance_on` と確定まで同一 Tx で行う」に書き替え、書込ロックがファイル I/O 中も保持されるという帰結を明記する | New |
| R-02 | Minor | `functional-spec.md` > §3 W7 冒頭の入口記述 | 「契約テスト（`publication_recovery_contract` 12 件）で検収する」の 12 がどの数え方でも実測と合わない。同ファイルの `#[tokio::test]` は 33 件（`cargo test --locked -p core-read-model-updater --test publication_recovery_contract -- --list` が `33 tests`）、`resolve_publication` を呼ぶテスト関数は 7 件（呼出箇所は 8）。§7 の「`publication_recovery_contract` 33」とも整合しない。`resolve_publication` の U7 未配線を申し送りにとどめる裁定はこの検収件数を根拠にしているため、数値の裏取りが要る | 12 を実測値へ置換し、何を数えた値か（ファイル全体か `resolve_publication` 経路か）を併記する | New |
| R-03 | Minor | 3 文書に散在する `52fce820`（`functional-spec.md` §1・§4 表見出し・W6・W7・§7、`entities.md` 正本の範囲・属性対応表見出し、`rules.md` 正本と出典） | `git merge-base --is-ancestor 52fce820 origin/main` が偽。`52fce820` は squash 前のブランチ側コミットで、`main` 上の対応コミットは `b9be20f6`（`#120`）である。新規クローンからこの SHA は解決できない。§1 冒頭だけが「RMU クレートは `origin/main` と同一」と補っており実体は追えるが、他の 6 か所は SHA 単独で参照している | `b9be20f6`（`main` の squash コミット）を併記するか置換し、実測の再現手順を fresh clone から辿れるようにする | New |
| R-04 | Minor | `traceability.json` > coverage の FR1.1 エントリ | `rules.md` の BR4.2 は `source: "FR1.1; NFR1"` だが、`traceability.json` の FR1.1 の target 一覧に BR4.2 が無い（NFR1 側には有る）。BR 17 本の `source` と coverage 4 件を全数突合した結果、不一致はこの 1 件のみ。traceability センサーは `gaps` / `orphans` / `missing_from_table` / `invalid_entries` / `invalid_targets` がすべて空で通るため、機械では検知されない | FR1.1 の target へ BR4.2 を追加するか、BR4.2 の `source` から FR1.1 を落として正本間を一致させる | New |
| R-05 | Minor | `rules.md` > BR2.1.logic、`entities.md` > `AuditBlock.fields` の制約 | 両者は「列挙値は計画の文書順で並べ、集合の辞書順にしない」とだけ書く。しかし `in_document_order`（`workspace/projection.rs:1806-1812`）は `plan.stages()` 側を走査して `slugs.contains(...)` で絞るため、計画に含まれない slug は監査行から脱落する（実装のドキュメンテーションコメントも「計画に無い slug は写さない」と明記）。NFR1 の逐語互換に関わる振る舞いが設計に現れていない | BR2.1 と entities の当該制約へ「計画に含まれない slug は監査行へ写さない」を明記する | New |
| R-06 | Minor | `functional-spec.md` > §5 の分類一覧（`PlanUnavailable` の説明） | `PlanUnavailable` を「`Started` / `Created` が無く 1 行も描けない」とのみ説明するが、製品コードの返却は 2 か所ある。`orchestration/read_model_updater.rs:319`（`resolve_plan` の失敗、本文どおり）と `:155`（保存済み計画の `to()` まで履歴が届かないとき）である。後者は §5 にも W3 にも記述が無く、W6 手順 2 の「履歴が不足するなら破損として停止」とも分類が異なる | §5 の分類説明へ、保存済み計画の履歴切り落としも `PlanUnavailable` になることを追記する | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| `aidlc-sensor-required-sections`（`functional-spec.md`） | PASS（`h2_count` 7、`findings_count` 0） | テンプレート未供給のため H2 の構造検査のみ。所見なし |
| `aidlc-sensor-required-sections`（`entities.md`） | PASS（`h2_count` 3、`findings_count` 0） | 同上 |
| `aidlc-sensor-required-sections`（`rules.md`） | PASS（`h2_count` 3、`findings_count` 0） | 同上 |
| `aidlc-sensor-upstream-coverage` | PASS（`unreferenced` 空、`findings_count` 0） | `consumes` 5 本すべてが `functional-spec.md` から参照されている |
| `aidlc-sensor-traceability` | `pass:false`、`findings_count` 36 | 36 件はすべて `missing_from_upstream_ids`（共有 story-map 上の他 Unit の要求 ID）で既知のノイズ。`gaps` / `orphans` / `missing_from_table` / `invalid_entries` / `invalid_targets` はすべて空。R-04 はこの検査が拾わない層の不一致である |
| `linter` センサー | 対象外 | 生成物が Markdown / JSON のみで TS/JS の生成コードが無い |
| `cargo test --locked -p core-read-model-updater` | 9 バイナリ 481 件 passed、0 failed | 内訳も §7 の記載と一致（lib 295 / audit_block_golden 1 / cross_shard_read 5 / journal_reader_impl 46 / projection_golden 18 / publication_file_contract 13 / publication_recovery_contract 33 / read_model_updater 31 / read_tables 39）。workspace 全体 2,354 件は本レビューでは再実行していない |

### Summary

主要な懸念は R-01 で、本再走が「現行コードを正として追従させた」と宣言した排他機構そのもの（ファイル適用の Tx 内外と、再検査と適用の前後関係）がコードと逆向きに書かれている。他の 5 件は数値・コミット参照・分類の精度に関する所見で、設計の骨格は健全である。G-1（`in_document_order` による `Recomposed` の文書順）、G-2（`IntentUnavailable` の `ReadTables::project` からの伝播）、G-4（§4 の実現列、blocked = `PublicationConflict` の返却）、G-5（`ProjectionCursor` の anchor 2 属性、`SharedProjectionHead.verified`、`read_*` 15 表 + steering 2 表）、G-6（W6 / W7 の入口と U7 配線 — `restore_missing_files` は `runtime.rs:1822` で `ReadModelUpdater::catch_up` の前に毎回呼ばれ、`rebuild_read_model` / `resolve_publication` は runtime から未配線）、G-7（W8 の `advance_on` 三分岐と BR5.3 の分類名）、G-8（§7 の実測行）、および `CatchUpError` 14 変種の過不足なしはいずれも実測で裏が取れた。RECOMPOSED の列挙順を上流契約が定めていないという主張も正しく（`audit-format.md:93` は必須フィールドのみ、`contract-summary` :433-436 は payload の形のみ）、その折り戻しは `inception/contract-design/pending-revision.md` の項目 2 に文面案付きで既に着地しているため、扱いは妥当である。

