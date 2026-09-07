# security-design — U9 正本・仕様の canon 追従（`u9-canon-docs`）

> NFR Design（Construction 3.3）成果物（Unit: U9、kind: spec）。**改訂履歴**: 初版 2026-08-23（Bolt B4 向け、レビュー READY Minor 1）→
> **再走 2026-09-07（本版、Modify）**: 再走版 `../nfr-requirements/security-requirements.md`（NFR1.1〜NFR1.5 / NFR2.1〜NFR2.6）と実測記録
> `../functional-design/gap-measurement-20260907.md`（現行コード `main` `02cacea2` 基準、§1〜§5 と追加実測 1〜4）に合わせ、改訂の作法・委譲の設計・
> 受入検査を更新した。
>
> 出典: `../nfr-requirements/security-requirements.md`（要求 11 件と §3 STRIDE）、`../nfr-requirements/tech-stack-decisions.md`（Markdown / 日本語正本 / grep 受入 /
> 実測の手段）、`../functional-design/rules.md`（BR1.6 / BR3.3 / BR3.7 / BR4.2 / BR5.1 / BR5.2 / BR5.3）、`../functional-design/functional-spec.md`（§2 責任分担、
> §3 ワークフロー）、`../../../inception/contract-design/contract-summary.md`（U9 は契約面を持たないが、本再走では改訂対象 — C1 / C3 / C4 / C5 / C6 / §4）、
> nfr-requirements レビュー（2026-09-07、R-01 却下 / R-02 採用 — gap-measurement 追加実測 3・4）、オーナー裁定 2026-09-07（選択肢 A: R-02 は推奨裁定で進む）、
> 確認事項 `nfr-design-questions.md`（P1〜P3 = 2026-08-23、**P4〜P8 = 2026-09-07**、Looks correct）。
> spec kind のため成果物は本ファイルと `traceability.json`（logical-components は作らない）。
>
> 文書だけの Unit の「セキュリティ設計」= 正本を壊さずに改訂するための**作法**、改訂を委譲するときの**境界**、壊れていないことを示す**受入検査**の設計。

## 1. 設計方針

(a) 改訂は最小変更で出典を残す（追跡可能性 — NFR1.3）、(b) 逐語契約には触れない（upstream 互換 — NFR1.1）、(c) 合否は機械的に示す（diff / grep / 行数 /
バイト比較 — NFR2.x）、(d) **記録ではなく現行コードを基準にする**（記録の主張は実装コード・テスト・仕様で検証してから採用 — NFR2.6 / BR5.3）、
(e) **未実装を実装済みのように書かない**（『予定（未実装）』の明記 — NFR1.4）、(f) **履歴は消さず失効を追記する**（共有契約の `## Review` 節・打消し線 —
NFR1.5）、(g) レビュー指摘はすべて処理してからマージ（review-thread gate — NFR2.4）。

## 2. 改訂の作法（NFR1.1 / NFR1.3 / NFR1.4 / NFR1.5 / NFR2.5）

| 作法 | 内容 |
|---|---|
| 最小変更 | 対象節だけを書き換え、周辺の逐語・体裁は保つ。改訂箇所は gap-measurement §2.1〜§2.6 の表の行に限り、『維持』行は触らない（BR5.1 (e)）。節の新設は BR が求める場合のみ（01 号 §7.1 の原則追記、10 号 §3 のポート 2 行追加、12 号 §2.3 のクエリ 3 件追記） |
| 出典注記 | 改訂した文・表の行・箇条の末尾に括弧書きで出典を残す — 形式 `（ADR-010）` / `（B13 2026-08-30）` / `（オーナー裁定 2026-09-02）` / **`（実測 intent_execution.rs:40 / IntentExecution::replay）`**。複数は `/` 区切り。実測の所在は path:line または型・関数名（NFR1.3、tech-stack §1） |
| 逐語契約の保護 | `docs/specs/research/**` は読むだけ（変更ゼロ）。10 号 §1 の「逐語の完全列挙は抽出文書と upstream を正とする」を維持。監査イベント名 / CLI 語彙 / `AIDLC_*` / 逐語文言 / ファイル形式の記述は引用のみで改変しない。`Directive` / `DirectiveKind` / `ContinueToken` は「所在 = クエリ側」を書き足すだけで、kind の列挙や JSON 形は変えない（BR3.3 (d)） |
| 履歴の残し方 | 失効した記述は削除せず `~~旧文~~ — 失効（日付 / 出典）` で残す。履歴として残す行には履歴マーカー（同一行に `~~`、または 旧 / 失効 / 是正済み / 改名 / 履歴 のいずれか）を必ず含める — これが受入 (2) の grep 除外条件になる。coding-rules の履歴的言及 8 行（P4）は本文を変えずマーカー語だけを添える。既存の `## Review` 節（components / contract-summary / unit-of-work）は 1 バイトも変えない（NFR1.5） |
| 実装状態の表記 | gap-measurement §4.7 の未実装項目（unpark / jump / recompose のユースケースと CLI 配線、フック 4 本、doctor、`aidlc-state` 他 24 動詞・`aidlc-bolt` 他 7 動詞、workspace 集約 3 と供給面 4、`intents.json` 直列化、Bolt / SwarmBatch）は `予定（未実装、クリティカルパス n）` の形で書く。記録間で食い違う主張は §4.8 の裁き（コードの現状）に従い、記録 A / B の文面を転記しない（NFR1.4） |
| 用語 | RMU の呼出は「ポート・ユースケース・RMU は `async fn`、駆動ループ・`tokio::spawn` を持たず、合成ルートが await で直列に呼ぶ」と書き、**「同期呼出」とは書かない**（ADR-006 は有効 — gap-measurement 追加実測 1）。「同期」を使えるのはドメイン（集約）が I/O を持たず純粋であることを述べる文だけ |
| 言語と体裁 | 日本語正本、固定トークン（型名 / API 名 / ファイル名 / ID / YAML キー / 逐語文言）は英語のまま。Markdown 表は見出しと同じ列数（regex 内の `\|` はエスケープ）、同一見出しの重複を作らない（NFR2.4 / NFR2.5） |
| 範囲の規律 | 改訂対象は P4 の確定版 — coding-rules **20 行 9 ファイル**（改訂 12 行 6 ファイル + 履歴マーカー付与 8 行 3 ファイル）、仕様 4 号（01 / 10 / 11 / 12。deviations は触らない）、共有契約 3 本（components 全面 / contract-summary 節単位 / unit-of-work 注記のみ）、`decisions.md` ADR-010 :476-478 への失効注記 1 段落。コード（modules / tools / scripts / .github / Cargo.*）と `formal/` は触らない |

## 3. 委譲の設計（NFR2.6 — 2 派遣の書込スコープとブリーフ）

project.md Mandated「実装は委譲し、メインは設計・監査・レビュー・最終統合判断に温存」に従い、文書改訂の Bolt はサブエージェント 2 派遣で作成し、
メインセッションが差分の全件レビューと受入検査（§4）を行う。書込スコープは重複させない。

| 派遣 | 書込スコープ（所有ファイル） | 主な作業 | 規模 |
|---|---|---|---|
| A | `coding-rules/`（README / error-handling / factory-naming / gateway-taxonomy / module-visibility / use-case-rules / command-query-separation / interior-mutability / field-visibility）、`docs/specs/10-orchestration.md`、`docs/specs/01-domain-model.md` | BR1.6 の 12 行置換 + 履歴マーカー 8 行 + BR4.2 の README 同期（表の一言・機械強制、:10 の first-class-collections 行の置き場、:12「13 本」の件数）、10 号 §2.1 の 2 集約分割・§3 ポート表・§2.2 所在列、01 号 §3.2 / §3.3 / §7.1 | S + M |
| B | `docs/specs/11-workspace.md`、`docs/specs/12-workflow-definition.md`、`inception/domain-design/components.md`、`inception/contract-design/contract-summary.md`、`inception/units-generation/unit-of-work.md`（注記のみ）、`inception/domain-design/decisions.md`（ADR-010 :476-478 の失効注記のみ） | 11 号 §2.1 予定明記・§3 ポート・§2.3 / §4 RMU、12 号 §2.3 クエリ 3 件・§5 ユースケース表、components.md 全面（クレート 10 / 集約 4 / イベント 16・1・2・4 / `read_*` 17 表）、contract-summary C1 / C3 / C4 / C5 / C6 / §4、unit-of-work U3 注記 4 行、ADR-010 注記 1 段落 | M + M |

**ブリーフの必須事項**（どちらの派遣にも書く）:

- (a) **実測表は 3 列丸ごと渡す** — gap-measurement §2.1〜§2.6 の『所在（行）/ 現行文言 / コード（置換後）』を省略せずに貼る。行番号だけを渡すと「その行に置換後の語が無い」と読み違える（nfr-requirements レビュー R-01 の実例）。行番号は探索の目安であり、同じ節の現在の内容を読む。
- (b) **「正しい姿」の正本** = gap-measurement §4 の台帳（O1〜O15 / P1〜P9 / R1〜R7 / W1〜W5 / S1〜S4 / K1〜K6）と §4.8 の裁き。台帳に無い主張は書かない。記録（Bolt / Unit の設計記録・完了報告）を転記しない。
- (c) **訂正 2 件を明示** — ① RMU 呼出は §2「用語」のとおり書き「同期」と書かない、② `decisions.md:476-478` に `~~…~~ — 失効（2026-08-30 / B13、version は集約内 version() / with_version()、Rehydrated* 撤去、expected_version 引数なし）` を追記する（削除しない）。
- (d) **作成報告の形** — 改訂 1 件ごとに根拠列「コードの所在（path:line または型 / 関数名）/ テストの有無 / 仕様の該当節」を付ける。添えられない改訂は保留として報告し、勝手に決めない（NFR2.6 / BR5.3）。
- (e) **禁止事項** — push / PR / GitHub 書込をしない。`AIDLC_*` 環境変数でフックを回避しない。書込スコープ外のファイルに触らない。コードと `formal/` を触らない。成果物の保存は Write / Edit 経由。
- (f) **受入検査の自己実行** — §4 の (2) sentinel grep と (4) 表整形を派遣側でも実行し、結果を報告に貼る（メインの再実測で突合する）。
- (g) **モデルの推奨** — 派遣 A・B とも Opus。A の置換は定型だが、10 号 §2.1 の 2 集約分割と 01 号 §3.2 の書き直しは §4 台帳の読解を要し、レビュアー（Opus 級）でも表の列を読み違えた実例があるため、Sonnet の節約より読解の確度を優先する。この判断は code-generation の計画承認でオーナーが変えてよい。

**メインセッションの責任**: ブリーフの作成、派遣結果の diff 全件レビュー、§4 の受入検査 (1)〜(10) の実行、gap-measurement §2 との突合、統合結果の受入判断、
PR 本文への実測結果の転記。新しい方針や未解決の規範衝突はオーナーが決める（保留として上げる）。

## 4. 受入検査の設計（NFR1.2 / NFR1.4 / NFR1.5 / NFR2.1〜NFR2.4 — PR の受入チェックリスト）

code-generation の計画に次の 10 項目を置き、Bolt の PR 本文に**実測結果を貼る**。基線はいずれも 2026-09-07 の実測。

1. **コード変更ゼロ**（NFR2.1）: `git diff --stat origin/main..HEAD -- modules tools scripts .github Cargo.toml Cargo.lock` が空。CI 7 ジョブ（aidlc-distribution / check /
   quint / coverage / audit / review-thread-resolution / ci-success）は変更なしで緑。
2. **sentinel grep**（NFR2.2）: 次の結果が **0 件**（基線 44 件 = coding-rules 12 + CONSISTENCY-AUDIT 4 + 仕様 4 号 28。CONSISTENCY-AUDIT 4 件は範囲外化、
   残り 40 件は BR1.6 の 12 行・履歴マーカー 8 行・BR3.3 の 4 号改訂で消える）。

   ```sh
   grep -rnE 'effective_plan_action|next_in_scope_stage|AuditLedgerRepository|AuditLedgerService|StateFileStore|report_forward|gate_start|WorkflowExecution|RehydratedWorkflowExecution|message-catalog' \
     $(ls aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/*.md | grep -v CONSISTENCY-AUDIT) docs/specs/*.md \
     | grep -vE '~~|旧|失効|是正済み|改名|履歴'
   ```

   範囲は `coding-rules/*.md`（日付つき監査記録 `CONSISTENCY-AUDIT-*.md` を除く — オーナー裁定 2026-09-07 選択肢 A）+ `docs/specs/*.md`（`research/` を除く）。
   `StageGraphReader` は BR5.1 (c) で sentinel から外れている（gateway-taxonomy「適用の帰結」節の旧→新移行表と禁止名テーブルに履歴として残るため）。
3. **README の無矛盾**（NFR2.3）: `ls coding-rules/*.md | grep -vE 'README|good-examples|CONSISTENCY-AUDIT' | wc -l`（= 22）と、README 表のリンク行 23 のうち
   good-examples 行を除いた 22 が一致（基線で一致済み）。各行の一言・機械強制が本文と一致（目視）。加えて :10 の first-class-collections 追加 1 行が
   「規則が衝突したら」節の中に混入している点と、:12「規則が 13 本」の古い件数を BR4.2 で直す。
4. **表・見出しの整形**（NFR2.4）: 改訂した表の全行でセル数 = 見出し数（`\|` エスケープを含めて数える）、同一文言の見出し重複なし。
5. **逸脱登録の維持**（NFR1.2）: `git diff --stat origin/main..HEAD -- docs/specs/deviations.md` が空（SQLite 行 1 件は 2026-08-23 の Bolt で登録済み）。
6. **履歴保全**（NFR1.5）: components / contract-summary / unit-of-work の 3 本で、`## Review` 見出し（基線 :430 / :486 / :207）から末尾までのバイトが改訂前後で同一
   （`diff <(git show origin/main:<file> | sed -n '/^## Review$/,$p') <(sed -n '/^## Review$/,$p' <file>)` が空）。`decisions.md` は `## Review` 節を持たないので、
   diff が ADR-010 の追記 1 段落だけであることを確認する。失効は削除ではなく打消し線 + 日付で残っている。
7. **実装状態の表記**（NFR1.4）: 4 号の `予定（未実装` の grep（基線 0 件）で、§4.7 の未実装項目 — unpark / jump / recompose、フック 4 本、doctor、workspace 集約
   `Intent` / `Space` / `Worktree` と供給面 4 つ、`intents.json` 直列化、Bolt / SwarmBatch — がすべて予定表記になっている。§4.8 の 13 論点と本文が一致。
8. **実測表との突合**（BR5.1 (e)）: gap-measurement §2.1〜§2.6 の各行の『処置』が反映され、『維持』行が変わっていない（メインの diff 全件レビュー）。
9. **用語**（追加実測 1）: `grep -nE '同期' docs/specs/{01,10,11,12}-*.md inception/domain-design/components.md inception/contract-design/contract-summary.md | grep -iE 'rmu|catch_up|投影'`
   が 0 件（基線 0 件。components.md:30「集約は … 純粋・同期」はドメインの記述であり対象外）。RMU 呼出は `async fn` / await 直列と書かれている。
10. **レビュー**（NFR2.4 / ステージ）: CodeRabbit のスレッドは返信 + resolve を全件（review-thread gate が `ci-success` を赤にする）。code-generation の advisory
    レビューで `## Review` READY（所見は PR 本文に転記）。収束条件（必須 CI green ∧ unresolved = 0 ∧ 全コメント返信済み）を最新 head で再実測してから merge queue へ。

## 5. 逸脱登録の維持（NFR1.2 / BR3.4）

達成済み（2026-09-07 実測: `docs/specs/deviations.md` に SQLite 行 1 件、理由欄が ADR-003 / 007 を指す）。本再走の Bolt は deviations.md を触らないので、
設計上の手当ては受入 (5)「変えていないことの確認」に縮める。初版 §4 に置いていた行の草案は適用済みのため本版から外した（内容は deviations.md が正本）。

## 6. 失敗の扱い

- 受入 (1)〜(9) のいずれかが落ちたら PR を merge queue に入れない（直して再実測）。
- 出典の無い改訂・逐語契約に触れた改訂・§4 台帳に無い主張はレビューで差し戻し（NFR1.3 / NFR1.1 / NFR2.6）。
- 派遣が「根拠列を添えられない」と報告した改訂は保留とし、メインが実測して裁くか、規範の衝突ならオーナー裁定へ上げる（functional-spec §3.3「保留」）。
- 派遣が書込スコープ外に触った場合はその変更を取り消し、ブリーフを直して再派遣する。
- 改訂が設計（rules.md の BR）に無い判断を要したら、推測で進めず code-summary の「設計質問」に書いてオーナー裁定へ。

## 7. 要求への対応

| 要求 | 設計上の手当て |
|---|---|
| NFR1.1 | 逐語契約の保護・research/ 不変（§2）、`Directive` 系は所在列のみ（§2）、受入 (1) |
| NFR1.2 | 達成済みの維持（§5）、受入 (5) |
| NFR1.3 | 出典注記の形式に実測の所在を含める（§2）、ブリーフ (d) の根拠列（§3） |
| NFR1.4 | 実装状態の表記（§2）、受入 (7)、§4.8 の裁きに従う（§3 (b)） |
| NFR1.5 | 履歴の残し方（§2）、ADR-010 注記は追記のみ（§3 (c)）、受入 (6) |
| NFR2.1 | コード変更ゼロの diff と CI 7 ジョブ（受入 (1)）、範囲の規律（§2） |
| NFR2.2 | sentinel 10 語 grep — CONSISTENCY-AUDIT 除外・履歴マーカー付与 8 行で 0 件へ到達可能に（受入 (2)） |
| NFR2.3 | README の行数 22 = 22 と索引のずれ 2 点（受入 (3)） |
| NFR2.4 | 表・見出しの整形（§2 / 受入 (4)）、レビューボット全件と収束条件（受入 (10)） |
| NFR2.5 | 日本語正本・固定トークンは英語（§2） |
| NFR2.6 | 現行コード基準・記録の非転記（§1 (d) / §3 (a)(b)）、根拠列つきの作成報告（§3 (d)）、実測表との突合（受入 (8)）、用語検査（受入 (9)） |

## 8. 前版からの変更（2026-09-07 再走）

- §1 に方針 (d) 実測基準 / (e) 実装状態 / (f) 履歴保全を追加。
- §2 の作法を再走版へ: 出典注記に実測の所在、履歴の残し方を打消し線 + マーカー語に、「実装状態の表記」「用語」を新設、範囲の規律を P4 の確定版（20 行 9 ファイル /
  4 号 / 共有契約 3 本 / ADR-010 注記）へ。初版の「履歴は『旧』明記の比較表にだけ」は、打消し線 + 日付の追記も認める形に広げた（NFR1.5 と整合）。
- §3「委譲の設計」を新設（派遣 A / B の書込スコープ、ブリーフ必須事項 (a)〜(g)、メインの責任）。
- §4 の受入検査を 7 → 10 項目へ（履歴保全・実装状態・実測表突合・用語を追加。sentinel は 7 語 → 10 語、範囲から CONSISTENCY-AUDIT を除外、基線 44 件を記録）。
- §5 逸脱登録を「達成済みの維持」に縮小（行の草案は適用済み）。
- §7 の要求対応表を NFR1.1〜NFR1.5 / NFR2.1〜NFR2.6 の 11 行へ。
- 初版のレビュー所見（Minor 1 = `StageGraphReader` 除外の出典）は BR5.1 (c) が `StageGraphReader` を sentinel から外したことで前提ごと閉じた（`pending-revision.md` に注記）。

## Review 履歴（2026-08-23、iteration 1、READY）

> 初版に対する advisory レビュー。所見 #1（Minor — §3 受入 2 の `StageGraphReader` 除外根拠が gateway-taxonomy の禁止名テーブルではなく旧→新移行表であり、
> pending-revision への出典も欠けていた）は、再走版 BR5.1 (c) が `StageGraphReader` を sentinel から外す裁定を確定したため、除外根拠の記述自体が不要になり
> 前提ごと解消（2026-09-07）。当時のセンサー結果は履歴であり、本再走の承認根拠には使わない。

| # | Severity | 要旨 | 本再走での扱い |
|---|---|---|---|
| 1 | Minor | `StageGraphReader` 除外の根拠となる表の名指しが実ファイルと不一致、pending-revision への出典欠落 | 解消 — BR5.1 (c) で sentinel から除外を確定（受入 (2) に出典を明記） |

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-07T01:48:28Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Minor | `security-design.md` 冒頭注記（:11）「オーナー裁定 2026-09-07（選択肢 A: R-02 は推奨裁定で進む）」 | `gap-measurement-20260907.md` §5「追加実測4」（:254-261）は R-02 の採用案を (a) CONSISTENCY-AUDIT 除外・(b) 履歴的言及8行へのマーカー付与の2点として「推奨する裁定（オーナー確認待ち）」と記すのみで、A/B/C/X のような選択肢文字を提示していない（Q4 のような明示的な選択肢構造ではない）。監査ログ（`audit/j5ik2o-mac-studio-lan-c4a9057ffc1c.md`）を通読したが、「選択肢 A」という文言に対応する `DECISION_RECORDED` イベントは見当たらない。ただし、この (a)(b) の内容そのものは nfr-requirements の `nfr-requirements-questions.md` P4〜P7 と本ステージの `nfr-design-questions.md` P4（確認事項）に転記されており、両ステージとも `SUMMARY_CONFIRMATION_RECORDED`（nfr-design は 2026-09-07T01:43:37Z、Details: "Looks correct"）で人間が確認済みである。内容の正しさ（範囲 20 行9ファイル・sentinel 0件到達性）は本レビューで独立に実測し裏付けが取れており（下記 Validation Tool Results）、裁定の実体が無いわけではない——ラベルの精度の問題である。 | 「選択肢 A」という表現を、実際に監査ログへ残っている確認チェックポイント（nfr-requirements P4-P7 の Looks correct、nfr-design P4-P8 の Looks correct、2026-09-07T01:43:37Z）への参照に置き換えるか、Q4 のような明示的な A/B/C/X 選択肢がこの決定にも存在したことを示す監査エントリを追記する。実害はなく、次回改訂時の軽微な修正で足りる。 | New |

### Validation Tool Results

| Tool / 実測 | 結果 | 解釈 |
|---|---|---|
| required-sections（security-design.md） | PASS（h2_count=9, findings_count=0） | 必須節はすべて揃っている |
| traceability（traceability.json） | PASS（gaps=[], orphans=[], findings_count=0） | NFR1.1〜NFR2.6 の11 IDが security-requirements.md のNFR集合と過不足なく一致し、target は本文の実在節（§1〜§5）に解決する |
| upstream-coverage（security-design.md, consumes=security-requirements/tech-stack-decisions/functional-spec/contract-summary） | PASS（unreferenced=[], findings_count=0） | 4上流すべて参照済み |
| sentinel grep 実測（§4 受入(2)、CONSISTENCY-AUDIT除外） | 40件（内訳: coding-rules 12 / 01号 6 / 10号 9 / 11号 5 / 12号 8） | 本文の主張どおり。CONSISTENCY-AUDIT込みは44件（追加4件はCONSISTENCY-AUDIT-2026-08-24.md）で、基線44の内訳（coding-rules 12 + CONSISTENCY-AUDIT 4 + 仕様28）と完全一致 |
| sentinel grep 行単位の到達可能性検証 | coding-rules 12件 = BR1.6改訂対象のうち実際にsentinel語を含む4行（README:50/115, factory-naming:47/84）+ マーカー付与対象8行（command-query-separation:5, interior-mutability:5, field-visibility:46, factory-naming:5/99, error-handling:12, gateway-taxonomy:20/290）の合計と完全一致。仕様28件はすべて`WorkflowExecution`語で、gap-measurement §2.1〜§2.4の行単位ledgerが「要改訂/名称のみ」で個別に処置を割り当てている | R-02（nfr-requirements iteration1）が指摘した「現行計画では0件に到達できない」という欠陥は、P4の範囲拡張（12行6ファイル→20行9ファイル）で解消されている。残存漏れは検出されなかった |
| README無矛盾実測（§4受入(3)） | 規則ファイル数=22、索引テーブルのリンク行=23（うちgood-examples 1）→ 22=22で一致 | 本文の主張と完全一致 |
| README索引ずれ2点の実在確認 | README.md:10 のfirst-class-collections追加行は見出し「規則が衝突したら（優先順）」（:8）の直後、:12「規則が13本になり」の直前に置かれている（「規則が衝突したら」節の中に混入）。:12は「13本」だが実際は22本 | 本文が指摘する2点のずれは実在する |
| 訂正(c)①の実測（RMU呼出の用語） | `modules/app/aidlc/src/main.rs:12`に`#[tokio::main(flavor = "current_thread")]` `async fn main`、`runtime.rs:194,200,238`はいずれも`.await`呼出、`tokio::spawn`/`std::thread::spawn`は両ファイルに0件 | 「async fn・駆動ループ/spawnなし・合成ルートがawaitで直列に呼ぶ」という訂正後の記述と一致。「同期呼出」と書かないという方針は正しい |
| 訂正(c)②の実測（ADR-010失効注記） | `inception/domain-design/decisions.md:476-478`は現在形で「楽観versionは集約の外」「`RehydratedWorkflowExecution`」「`expected_version`引数」と記述し、打消し線や失効注記が無いことを確認 | B13（2026-08-30、version集約内化）以降の状態と食い違ったまま現在形で残っており、追記の必要性は実在する |
| ## Review見出し行番号の実測（§4受入(6)） | `components.md:430`、`contract-summary.md:486`、`unit-of-work.md:207`にいずれも`## Review`見出しが存在。`decisions.md`に`## Review`見出しは0件 | 本文の行番号主張と完全一致。バイト同一検査の基準行として妥当 |
| 委譲スコープの重複確認（§3） | 派遣A（coding-rules 9ファイル、10号、01号）と派遣B（11号、12号、components.md、contract-summary.md、unit-of-work.md、decisions.md）の所有ファイル集合に重複なし | project.md Mandated「書込スコープは重複させない」と整合 |
| linter / type-check | 対象外 | 本Unitの成果物にTypeScript / JavaScriptスニペットなし |

### Summary

自動センサー3種はすべてPASSで、本レビューで独自に実施した実測（sentinel grepの行単位到達可能性、README索引数値、ADR-010欠落注記、非同期呼出の実装事実、共有契約3本の`## Review`見出し行番号、委譲スコープの非重複性）はすべて本文の主張と一致した。前回のnfr-requirementsレビューが指摘したR-02（NFR2.2の「0件」が現行計画では達成不能）は、本ステージのP4範囲拡張（coding-rules 12行6ファイル→20行9ファイル）で構造的に解消されており、残存する未処置行は検出されなかった。唯一の所見（R-01、Minor）は、冒頭注記の「選択肢A」というラベルが監査ログ上の明示的なA/B/C/X選択肢に対応していないという記述精度の問題であり、決定の実体（範囲・根拠）自体は監査ログ上の確認チェックポイントで裏付けが取れている。Critical 0・Major 0のためREADY。
