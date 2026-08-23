# security-design — U9 正本・仕様の canon 追従（`u9-canon-docs`）

> NFR Design（Construction 3.3）成果物（Unit: U9、kind: spec、Bolt: B4）。出典: `../nfr-requirements/security-requirements.md`（NFR1.1〜1.3 / NFR2.1〜2.5、
> レビュー所見 1・2 — diff スコープの統一と出典明示）、`../nfr-requirements/tech-stack-decisions.md`、`../functional-design/rules.md`（BR1.1〜BR5.2）と
> `../functional-design/pending-revision.md`（回復レビュー所見 — BR2.5 の範囲 / §1b の WorkspaceLock / BR5.1 の grep 範囲と diff スコープ）、
> `../../../inception/contract-design/contract-summary.md`（U9 は契約面を持たない）、確認事項 `nfr-design-questions.md`（前提 P1〜P3、Looks correct）。
> spec kind のため成果物は本ファイルと `traceability.json`（logical-components は作らない）。
>
> 文書だけの Unit の「セキュリティ設計」= 正本を壊さずに改訂するための**作法**と、壊れていないことを示す**受入検査**の設計。

## 1. 設計方針

(a) 改訂は最小変更で出典を残す（追跡可能性）、(b) 逐語契約には触れない（upstream 互換）、(c) 合否は機械的に示す（diff / grep / 行数）、
(d) レビュー指摘はすべて処理してからマージ（review-thread gate）。

## 2. 改訂の作法（NFR1.1 / NFR1.3 / NFR2.5）

| 作法 | 内容 |
|---|---|
| 最小変更 | 対象節だけを書き換え、周辺の逐語・体裁は保つ。節の新設は BR が求める場合のみ（01 号 §7 のドメインモデル原則、§3.3 の集約表、12 号 §2.1 の識別子、deviations の 1 行） |
| 出典注記 | 改訂した文・表の行・箇条の末尾に括弧書きで出典を残す — 形式 `（ADR-008）` / `（C4 改訂 2026-08-23）` / `（Bolt B3 実装）` / `（オーナー裁定 2026-08-23）` / `（設計監査 R3 / C4）`。複数は `/` 区切り |
| 逐語契約の保護 | `docs/specs/research/**` は読むだけ（変更ゼロ）。10 号 §1 の「逐語の完全列挙は抽出文書と upstream を正とする」を維持。監査イベント名 / CLI 語彙 / `AIDLC_*` / 逐語文言 / ファイル形式の記述は引用のみで改変しない |
| 履歴の残し方 | 旧記述を残すときは「旧 → 新」の比較表に限り、見出しか 1 列目に「旧」と明記（BR5.1 の grep 除外対象 = 履歴注記）。本文の規範には旧 API 名・退役機構・旧称を残さない |
| 言語と体裁 | 日本語正本、固定トークン（型名 / API 名 / ファイル名 / ID / YAML キー / 逐語文言）は英語のまま。Markdown 表は見出しと同じ列数（regex 内の `\|` はエスケープ）、同一見出しの重複を作らない |
| 範囲の規律 | 改訂対象は entities.md の一覧（coding-rules 4 + 仕様 5 + components.md 1）に限る。コード（modules / tools / scripts / .github / Cargo.*）は触らない |

## 3. 受入検査の設計（NFR2.1〜2.4 — PR の受入チェックリスト）

code-generation の計画に次のチェックリストを置き、Bolt B4 の PR 本文に**実測結果を貼る**:

1. コード変更ゼロ: `git diff --stat origin/main..HEAD -- modules tools scripts .github Cargo.toml Cargo.lock` が空（BR5.1 (d) を安全側に統一 — NFR 要求レビュー所見 1）。
2. sentinel grep（NFR2.2）: `grep -rnE 'effective_plan_action|next_in_scope_stage|AuditLedgerRepository|AuditLedgerService|StateFileStore|report_forward|gate_start' aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/*.md docs/specs/*.md`
   の結果が履歴注記（「旧」明記の比較表、gateway-taxonomy §2 の禁止名テーブル）のみ。`StageGraphReader` は禁止名テーブルの意図的な記録として対象外。
   `next_in_scope_stage` は 12 号の全 5 出現（§2.3 ×2 / §4 / §8 / §9）を改訂対象にする（FD pending-revision 項目 1）。
3. README の無矛盾（NFR2.3）: `ls aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/*.md | grep -v README | wc -l` = README の表の行数。各行の一言・機械強制・裁定日が
   ファイル本文と一致（目視）。
4. 表・見出しの整形（NFR2.4）: 改訂した表の全行でセル数 = 見出し数（`\|` エスケープを含めて数える）、同一文言の見出し重複なし。
5. deviations の登録（NFR1.2）: 表に 1 行追加され、理由欄が ADR-001 / 003 / 004 / 007 を指す（§4）。
6. レビューボット（NFR2.4）: CodeRabbit のスレッドは返信 + resolve を全件（review-thread gate が `CI Success` を赤にする）。
7. ステージのレビュアー: code-generation の advisory レビューで `## Review` READY（または所見を PR 本文に転記）。

## 4. 逸脱登録の行（NFR1.2 / BR3.4）

`docs/specs/deviations.md` の表に追加する行（列: # / 分類 / upstream の挙動 / amadeus-ng の挙動 / 理由 / 記録）:

| # | 分類 | upstream の挙動 | amadeus-ng の挙動 | 理由 | 記録 |
|---|---|---|---|---|---|
| 4 | 設計変更 | 状態ファイル `aidlc-state.md`・監査シャードのテキストファイル群が真実源。read-modify-write は mkdir ロック（`<record>/.aidlc-lock/`、owner.json スタンプ、reap）で直列化 | SQLite ジャーナル（`journal` / `snapshot` / `checkpoint` — C6）が真実源。遷移は楽観 version で直列化し、ロック dir は生成しない。`aidlc-state.md` / 監査シャードは ReadModelUpdater の投影として**バイト互換**で再生成（リードモデル） | ES 化（ADR-001 / 003 / 004）とロック退役（ADR-007）。観測可能な差は (a) `.aidlc-store.sqlite` 相当のファイル追加（git 管理外）、(b) ロック dir の非生成。互換ファイルの内容は不変 | 2026-08-23 / ADR-003, ADR-007（NFR1 の逸脱登録） |

「予約（決定済み・記録待ち）」節に同趣旨の項目があれば本行へ統合して予約節から除く（重複登録を避ける）。SQLite ファイルの最終パスは U3 の設計で確定するため、
行には「相当」を付けて U3 が確定時に更新する旨を注記。

## 5. 失敗の扱い

- 受入チェック 1〜6 のいずれかが落ちたら PR を merge queue に入れない（直して再実測）。
- 出典の無い改訂・逐語契約に触れた改訂はレビューで差し戻し（NFR1.3 / NFR1.1）。
- 改訂が設計（rules.md の BR）に無い判断を要したら、推測で進めず code-summary の「設計質問」に書いてコンダクタ裁定へ（B3 の運用と同じ）。

## 6. 要求への対応

| 要求 | 設計上の手当て |
|---|---|
| NFR1.1 | 逐語契約の保護・research/ 不変（§2）、受入 2 |
| NFR1.2 | 逸脱登録の行（§4）、受入 5 |
| NFR1.3 | 出典注記の形式（§2） |
| NFR2.1 | コード変更ゼロの diff（§3 受入 1 — スコープは Cargo.* まで） |
| NFR2.2 | sentinel grep（§3 受入 2 — 除外規定と 12 号 5 箇所） |
| NFR2.3 | README の無矛盾（§3 受入 3） |
| NFR2.4 | 表・見出しの整形（§2 / §3 受入 4）、レビューボット全件（受入 6） |
| NFR2.5 | 日本語正本・固定トークン（§2） |

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-08-23T05:25:55Z
**Iteration:** 1（advisory, unit: u9-canon-docs）

### Findings

| # | Severity | Location | Finding | Recommendation |
|---|---|---|---|---|
| 1 | Minor | security-design.md §3 受入 2 vs `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md` | `StageGraphReader` の除外根拠が実ファイルと一致しない。§3 受入 2 は「`StageGraphReader` は禁止名テーブルの意図的な記録として対象外」と書くが、gateway-taxonomy.md の§2「Repository 名 = 集約名 + Repository」の禁止名テーブル（47〜51行目）に載っているのは `StateFileRepository` / `StageGraphRepository`（末尾 Repository）であって `StageGraphReader`（末尾 Reader）ではない。`StageGraphReader` が実際に登場するのは別セクション「## 適用の帰結（2026-08-22 の再設計）」内の「旧 → 新」移行表（89〜93行目、`旧` 列に明記）であり、これは security-design.md §2「履歴の残し方」が定義する履歴注記（「旧」明記の比較表）の方の基準で除外されるべきもの。除外という結論自体（sentinel grep 7 語からも `StageGraphReader` は既に外れている）は実質的に妥当だが、根拠として名指す表が違う。加えて、この一文には `pending-revision.md` への出典が無く、直前の nfr-requirements レビュー（`../nfr-requirements/security-requirements.md` 末尾 `## Review` Minor #2）が同じ NFR2.2 の `StageGraphReader` 記述に対して指摘した「出典を `../functional-design/pending-revision.md` 所見 3 と明示せよ」という是正が、`next_in_scope_stage` の一文（`（FD pending-revision 項目 1）`）には反映された一方、`StageGraphReader` の一文には反映されないまま残っている。 | 「`StageGraphReader` は履歴注記（gateway-taxonomy.md『適用の帰結』節の旧→新移行表、旧列）として対象外（`../functional-design/pending-revision.md` 所見 3）」のように、表の場所と出典ファイルを明示する一文に差し替える。 |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| `bun .claude/tools/aidlc-sensor-traceability.ts --stage nfr-design --output-path .../nfr-design/traceability.json` | `{"pass":true,"gaps":[],"orphans":[],"missing_from_table":[],"missing_from_upstream_ids":[],"invalid_entries":[],"invalid_targets":[],"findings_count":0}` | `traceability.json` の8 ID（NFR1.1〜NFR2.5）は `security-requirements.md` の upstream ID集合と過不足なく一致し、target欄の節番号もsecurity-design.md本文の実在節（§2/§3/§4）に解決する。機械検証 green。 |
| `bun .claude/tools/aidlc-sensor-required-sections.ts --stage nfr-design --output-path .../nfr-design/security-design.md` | `{"pass":true,"h2_count":6,...}` | H2見出し6本、閾値（≥2）を満たす。 |

### Summary

U9（spec Unit、Bolt B4）のNFR設計は、上流3点（nfr-requirements、functional-design の rules.md / pending-revision.md、実ファイルの docs/specs/deviations.md・docs/specs/12-workflow-definition.md・coding-rules/*.md）と広く突き合わせたが、いずれも実測と一致した — (a) §3受入1のdiffスコープはsecurity-requirements.md NFR2.1と一致し、rules.md BR5.1(d)より広い理由も「NFR要求レビュー所見1」の引用で明示済み、(b) §3受入2の `next_in_scope_stage` 5出現（§2.3×2/§4/§8/§9）は12号workflow-definition.mdの実測grep（5件、該当節も完全一致）と一致、(c) §2「範囲の規律」の「coding-rules 4 + 仕様 5 + components.md 1」はentities.mdのinstances一覧と一致、(d) §4のdeviations行はdeviations.md実ファイルの列形式・次番号（現行1〜3の次は4）・予約節（重複なし）と整合し、rules.md BR3.4の文言とも一致。traceability.json・required-sectionsは共にgreen。唯一の懸念はMinor#1（`StageGraphReader`除外の出典表が実ファイルと不一致、かつpending-revision.mdへの出典欠落）で、PRの合否判定には影響しない文書精度の指摘。Critical 0件・Major 0件のためadvisory目安（Critical 0かつMajor≤2）を満たし、READYと判定する。
