# security-requirements — U9 正本・仕様の canon 追従（`u9-canon-docs`）

> NFR Requirements（Construction 3.2）成果物（Unit: U9、kind: spec）。**改訂履歴**: 初版 2026-08-23（Bolt B4 向け、レビュー READY Major 1 / Minor 1）→
> **再走 2026-09-07（本版、Modify）**: functional-design 再走版 `../functional-design/rules.md`（BR1.1〜BR5.3、status 付き）と実測記録
> `../functional-design/gap-measurement-20260907.md`（現行コード `main` `02cacea2` 基準）に合わせ、範囲・合格基準・新設要求を同期した。
>
> 出典: `../functional-design/rules.md`（改訂内容と合格条件 BR5.1 / 作法 BR5.2 / 検証規律 BR5.3）、`../functional-design/functional-spec.md`（§2 責任分担、§6 照合結果）、
> `../../../inception/requirements-analysis/requirements.md`（NFR1 upstream 互換（D6 範囲）、NFR2 品質ゲート維持、制約 C4 日本語正本）、
> `../../../inception/contract-design/contract-summary.md`（U9 は契約面を**持たない**が、本再走では contract-summary 自体が改訂対象 — C1 / C3 / C4 / C5 / C6 / §4 の現行化、BR3.7）、
> `.github/workflows/ci.yml` / `scripts/coverage.sh`（CI の実測）、確認事項 `nfr-requirements-questions.md`（前提 P1〜P3 = 2026-08-23、**P4〜P7 = 2026-09-07**、Looks correct）。
>
> spec Unit のため「セキュリティ要求」= 正本文書の**改訂の安全性**（逐語契約を壊さない、出典の追跡可能性、自己整合、**実装状態を偽らない**）の要求であり、
> NFR2（品質ゲート）の文書版もここに置く。各要求は Inception の NFR ID を継承し枝番を付ける（NFR1.x / NFR2.x）。

## 1. 範囲と信頼境界

- 対象は**文書だけ**（P4）: `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/` の 6 ファイル 12 行（README / error-handling / factory-naming /
  gateway-taxonomy / module-visibility / use-case-rules — BR1.6 / BR4.2）、`docs/specs/{01,10,11,12}-*.md`（BR3.2 / BR3.3。`deviations.md` は触らない）、
  共有契約 3 本 `inception/domain-design/components.md`（全面）/ `inception/contract-design/contract-summary.md`（節単位）/
  `inception/units-generation/unit-of-work.md`（注記のみ）（BR3.7）。コードは触らない（`modules/` / `tools/` / `scripts/` / `.github/` / `Cargo.toml` / `Cargo.lock` の diff ゼロ）。
- 信頼境界: (a) 正本の権威 = オーナー裁定（coding-rules は 1 ルール 1 ファイル・裁定日つき）、(b) 仕様の権威 = upstream の観測可能契約（D6）と ADR、
  (c) **現行コードの実測**（`gap-measurement-20260907.md` §1 / §4 — 記録の主張はこれで検証したものだけを採用する、BR5.3）、(d) PR レビュー（アーキテクチャ
  レビュアー + レビューボット）。改訂は (a)(b)(c) に遡れる出典を持つものだけを通す。
- 秘密情報・個人情報を扱わない。外部ネットワーク不要（upstream ピンの参照は既存のローカル写し `tests/golden/upstream-3c3146cf/` と `docs/specs/research/`）。

## 2. 要求

| ID | 要求 | 合格基準 | 出典 |
|---|---|---|---|
| NFR1.1 | **逐語契約の不変** — 仕様改訂は「構造の規範と所有の記述」に限り、upstream 互換の逐語契約（監査イベント 86 語・CLI 語彙・`AIDLC_*`・LLM 分岐条件の文言・ファイル形式）と `docs/specs/research/*.md`（抽出文書）には触れない | `git diff --stat origin/main..HEAD -- docs/specs/research` が空。10 号 §1 の「逐語の完全列挙は抽出文書と upstream を正とする」の一文を維持。改訂行に逐語文言の変更が含まれないことをレビューで確認 | NFR1, BR5.2 |
| NFR1.2 | **逸脱の登録** — ES 化に伴う観測可能な逸脱（SQLite ファイルの追加・ロック dir 非生成・互換ファイルはリードモデル）は `docs/specs/deviations.md` の表に登録し、本文の改訂だけで済ませない | **達成済み**（2026-09-07 実測: deviations.md に SQLite 行 1 件、理由欄が ADR-003 / 007 を指す — BR3.4 done）。本再走では deviations.md を触らないので、達成状態が維持されていることを diff で確認 | NFR1, BR3.4 |
| NFR1.3 | **出典の追跡可能性** — 各改訂箇所に出典（ADR 番号 / 契約 ID / Bolt / オーナー裁定日 / **実測の所在 path:line または型・関数名**）を括弧書きで残し、推測で仕様を変えない | 改訂行の出典注記をレビューで確認（出典の無い改訂は差し戻し）。仕様本文の構造の主張は `gap-measurement-20260907.md` §4 の台帳行（O / P / R / W / S / K）に遡れる | NFR1, BR5.2, BR3.3 / BR3.7 の source |
| NFR1.4 | **実装状態の正確な表示** — 未実装の設計（unpark / jump / recompose のユースケースと CLI 配線、フック 4 本、doctor、workspace 集約 `Intent` / `Space` / `Worktree` と供給面 4 つ、`intents.json` 直列化、Bolt / SwarmBatch、`aidlc-state` / `aidlc-bolt` の未配線動詞）は仕様に『予定（未実装）』と明記し、実装済みのように書かない。記録同士が食い違う主張はコードの現状で裁く | 改訂後の 4 号で、§4.7 の未実装項目がすべて「予定」と明記されていることをレビューで確認。§4.8 の 13 論点（イベント 16 / version 集約内 / 差分再生 / `DefinitionRevision` ドメイン導出 / `StageSlugSet` 辞書順 / skeleton = Construction の最初の EXECUTE / RMU 同期呼出 / `Directive` 構築可能 7・kind 10 / RMU がジャーナルを読む / `CommitOutcome` 2 形 / `ReviewAttempt` は `StageSlot` 内 / `next_decision` の `Result` / `Started` は intent_id + stages）と仕様本文が一致 | NFR1, BR3.3 (g)(h), gap-measurement §4.7 / §4.8, P6 |
| NFR1.5 | **共有契約の履歴保全** — inception 成果物（components / contract-summary / unit-of-work）を現行化するとき、既存の `## Review` 節・打消し線つき履歴・ADR ステータス注記は削除せず、失効は打消し線 + 日付で追記する。`## Review` 節の Verdict / Reviewer / Iteration 行は変更しない | 改訂前後で各ファイルの `## Review` 節のバイトが同一（`diff` で確認）。失効した記述が削除ではなく `~~…~~ — 失効（2026-09-07 / U9 再走）` の形で残る | NFR1, BR5.2, no-backward-compatibility.md 対象外（履歴記述は消さず失効を追記）, P6 |
| NFR2.1 | **コード変更ゼロ** — 再走の Bolt は文書のみ。CI の **7 ジョブ**（`ci.yml` 実測: aidlc-distribution / check / quint / coverage / audit / review-thread-resolution / ci-success）は変更なしで緑のまま | `git diff --stat origin/main..HEAD -- modules tools scripts .github Cargo.toml Cargo.lock` が空。PR の CI 緑（`ci-success` 集約ジョブ） | NFR2, BR5.1 (d), P5 |
| NFR2.2 | **自己整合の機械検査** — 削除済み API 名・退役機構・旧称が現行規範として残らない。sentinel は BR5.1 (c) の 10 語（`effective_plan_action` / `next_in_scope_stage` / `AuditLedgerRepository` / `AuditLedgerService` / `StateFileStore` / `report_forward` / `gate_start` / `WorkflowExecution` / `RehydratedWorkflowExecution` / `message-catalog`）、範囲は `coding-rules/*.md` + `docs/specs/*.md`（`research/` を除く）、判定は**履歴マーカー（同一行に `~~`、または 旧 / 失効 / 是正済み / 改名 / 履歴 の語）の無い行が 0 件** | grep（履歴除外つき）の結果を PR 本文に貼る。`StageGraphReader` は gateway-taxonomy の禁止名テーブルにのみ現存するため sentinel から外す（BR5.1 で確定） | NFR2, BR5.1 (c), P5 |
| NFR2.3 | **索引の無矛盾** — `coding-rules/README.md` の一覧表の行数 = ルールファイル数（README / good-examples / CONSISTENCY-AUDIT を除く）、各行の一言・機械強制が各ファイルと一致（`README.md:50` の message-catalog、`:115` の `WorkflowExecutionState` 是正対象表記を含む） | README と `ls coding-rules/*.md` の突合（レビュー） | NFR2, BR4.2, BR1.6 |
| NFR2.4 | **表・見出しの整形** — 改訂した Markdown 表は見出しと同じ列数（regex 内の `\|` はエスケープ）、同一見出しの重複を作らない（レビューボットの markdownlint 指摘 MD056 / MD024 を予防） | CodeRabbit の該当指摘 0 件（出たら PR 内で直す — PR コメントは無視しない。`review-thread-resolution` ジョブが未解決スレッドを検出する） | NFR2, PR #25 / #27 の教訓 |
| NFR2.5 | **日本語正本** — 人間可読の改訂は日本語、固定トークン（型名・ファイル名・API 名・ID）は英語のまま | レビューで確認 | 制約 C4 |
| NFR2.6 | **検証規律** — 仕様・規則・共有契約に書く主張は、Bolt / Unit の記録・レビュー本文・完了報告を鵜呑みにせず、実装コード・テスト・仕様の三つで検証してから採用する。改訂案 1 件につき「コードの所在（path:line または型 / 関数名）・テストの有無・仕様の該当節」を添え、添えられない改訂案は保留にする | 委譲先の作成報告に各改訂の根拠列（所在 / テスト / 節）があること、メインの diff 全件レビューで `gap-measurement-20260907.md` §2 の『処置』列と一致し『維持』行が変わっていないこと（BR5.1 (e)） | NFR2, BR5.3, Q4 = A（オーナー 2026-09-07「正しさを実装コードもテストも仕様も常に検証」）, P6 |

## 3. 脅威の検討（STRIDE、文書改訂の規模）

| 区分 | 該当 | 扱い |
|---|---|---|
| Spoofing / Elevation of Privilege | 該当なし（文書だけ、権限変更なし） | — |
| Tampering | (a) 改訂のついでに逐語契約（D6）を書き換えてしまう、(b) 出典の無い「勝手な設計変更」を仕様に混ぜる、(c) **記録の転記で誤った規範を混入させる**（例: 旧 `WorkflowExecution` 16 属性・12 イベント・memento をそのまま仕様に写す — R-01 が指摘した再導入の危険）、(d) 未実装を実装済みのように書く | NFR1.1（research/ 不変）、NFR1.3（出典注記）、**NFR2.6（コード・テスト・仕様で検証）**、**NFR1.4（予定と明記）**、レビュー |
| Repudiation | 誰の裁定で変えたかが追えない / 失効した記述を消して経緯が消える | NFR1.3 + PR の記録、coding-rules の裁定日、**NFR1.5（履歴保全）** |
| Information Disclosure | 該当なし（秘密情報を含む文書ではない） | — |
| Denial of Service | 該当なし（CI への影響なし — NFR2.1） | — |

## 4. データ分類

| データ | 分類 | 扱い |
|---|---|---|
| coding-rules / docs/specs / inception の共有契約 3 本 | Public（公開リポジトリ） | 秘密情報なし |

## 5. 適用外

- NFR3（監査完全性）・NFR4（サプライチェーン）・NFR5（性能）: 文書だけの Unit で固有の要求を持たない（依存・コードの変更なし）。判定は 2026-08-23 から変わらない。

## 6. 前版からの変更（2026-09-07 再走）

- §1 範囲を P4 へ差し替え（coding-rules 6 ファイル 12 行 / 仕様 4 号 / 共有契約 3 本）。
- NFR1.2 を達成済みに、NFR2.1 の CI を実測 7 ジョブに、NFR2.2 の sentinel を BR5.1 (c) の 10 語 + 履歴除外判定に同期（旧 pending-revision 2 項目 —
  NFR2.1 / NFR2.2 の出典欄 — は rules.md BR5.1 が同期済みのため本版で閉じた）。
- NFR1.4（実装状態の正確な表示）/ NFR1.5（共有契約の履歴保全）/ NFR2.6（検証規律）を新設。§3 Tampering に (c)(d) を追加。

## Review 履歴（2026-08-23、iteration 1、READY）

> 初版に対する advisory レビュー。所見 #1（コード変更ゼロの diff 範囲が rules / 質問票 / 本表で 3 段階に食い違う）は rules.md BR5.1 (d) を
> `modules tools scripts .github Cargo.toml Cargo.lock` へ広げて解消（2026-09-07）。所見 #2（NFR2.2 の `StageGraphReader` 除外の出典）は BR5.1 (c) が
> sentinel から外すことを確定して解消。当時のセンサー結果は履歴であり、本再走の承認根拠には使わない。

| # | Severity | 要旨 | 本再走での扱い |
|---|---|---|---|
| 1 | Major | diff 範囲が 3 か所で食い違う（rules `modules tools` / 質問票 P2 / 本表 Cargo.* 込み） | 解消 — BR5.1 (d) と NFR2.1 が同じ範囲 |
| 2 | Minor | NFR2.2 が pending-revision に従って `StageGraphReader` を除外しているが出典が不明 | 解消 — BR5.1 (c) で除外を確定 |

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-07T01:24:44Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | `tech-stack-decisions.md` §2「ファイル（coding-rules）」の `gateway-taxonomy.md（:223 / :299）`・`module-visibility.md（:11 / :12 / :21）`・`use-case-rules.md（:11 / :36）` の各セル（`../functional-design/rules.md` BR1.6 由来） | BR1.6・本表が「改訂対象」として名指す行の一部が、実測すると挙げられている語を含んでいない。実測（本レビューで `grep -n` 実施）: `gateway-taxonomy.md:299` は `core-command-domain` ではなく `core-domain`（Repository 名と集約名の照合の説明で、そもそも改訂候補として妥当な行ではない）。`module-visibility.md:11` は `core_command_domain::` ではなく `core_domain::{workspace, orchestration, workflow_definition}`。`module-visibility.md:21` は名前空間の話ではなく「昇格の運用」の一般論で、クレート名への言及自体が無い。`use-case-rules.md:11` は `core-command-use-case` / `core-command-interface-adapter` ではなく `core-use-case` の `Cargo.toml` に `core-interface-adapter` が無いという記述（`-command-` を含まない）。NFR1.3（出典の追跡可能性）と NFR2.6（検証規律 — 「path:line で検証してから採用」）はこの再走の眼目そのものであり、その両方が求める「path:line 根拠」が実測と食い違ったまま `tech-stack-decisions.md` の対象一覧（12 行 6 ファイル）に載っている。次の Bolt がこれらの行を『core-command-domain 等を現行名へ』と機械的に直そうとしても、対象語が実在しないため改訂できない。 | 次の Bolt（コード生成・文書改訂）の作業パッケージに渡す前に、`gateway-taxonomy.md:299` / `module-visibility.md:11` / `module-visibility.md:21` / `use-case-rules.md:11` の 4 セルについて実際の該当行番号と現行文言を再実測し、対象語が実在する行番号へ差し替えるか、対象から外す（すでに現行名になっている、または改訂の必要が無い行は「維持」へ分類）。functional-design 側の `rules.md` BR1.6 の記述も同じ実測結果で同期する必要がある旨を申し送る。 | New |
| R-02 | Major | `security-requirements.md` NFR2.2、`tech-stack-decisions.md` §1「自己整合の検査」 | NFR2.2 の合格基準（sentinel 10 語 + 履歴マーカー 5 種の除外で「無い行が 0 件」）を、本レビューで実際に `grep -nE` 実行して確認したところ、BR1.6 が計画する 12 行の改訂をすべて適用しても 0 件にならない残存が別に存在する: `error-handling.md:12`「2026-08-29 の `message-catalog` 解体後の形」（`解体` は履歴マーカー 5 語 [旧/失効/是正済み/改名/履歴] のいずれでもなく `~~` も無い）、`gateway-taxonomy.md:20`「`StateFileStore` ポート削除」・`:290`「`core_use_case::workspace::StateFileStore`（ポート）| 削除 →」（`削除`/`退去` は履歴マーカーに無い）、`factory-naming.md:5`「`WorkflowExecution::start` ほか」・`:99`「`WorkflowExecutionStateBuilder::plan` ほか12本」（マーカー無し）。これら 5 行は BR1.6 の 12 行リストに含まれておらず、改訂計画の対象外のまま sentinel に residual として残る。NFR2.2 が定義する機械検査は、この再走が主張する改訂範囲では「0 件」という合格状態に到達できない — 前回レビュー所見 #1 と同種（合格基準の達成不能性）の、別の入力（sentinel/履歴マーカーの定義）に起因する新規の同型問題。 | (a) この 5 行を BR1.6 / tech-stack §2 の改訂対象へ追加する、または (b) 履歴マーカー語彙に「解体」「削除」等の同義語を追加して sentinel 判定の対象外にする、のどちらかを次の Bolt 前に裁定し、NFR2.2 の合格基準文言（sentinel 語・除外語の定義）を実際に 0 件へ到達できる形に更新する。 | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| required-sections（security-requirements.md） | PASS（h2_count=7, findings_count=0） | 必須節はすべて揃っている |
| required-sections（tech-stack-decisions.md） | PASS（h2_count=4, findings_count=0） | 同上 |
| traceability（traceability.json） | PASS（gaps=[], orphans=[], findings_count=0） | NFR1〜NFR5 の列挙・target 参照に欠落なし |
| upstream-coverage（security-requirements.md, consumes=functional-spec/rules/requirements/contract-summary） | PASS（unreferenced=[], findings_count=0） | 4 つの上流契約すべてが参照されている |
| linter / type-check | 対象外 | 本 Unit の成果物に TypeScript / JavaScript スニペットなし |

### Summary

自動センサーはすべて green で、上流（rules.md の BR、requirements.md の NFR1/NFR2、gap-measurement §4.7/§4.8）との対応も実測で確認でき、前回所見 #1・#2 の解消主張も裏付けが取れた。ただし本レビューで独自に実測した結果、(1) tech-stack-decisions.md が挙げる改訂対象 12 行のうち 4 セルの引用が実際のファイル内容と一致せず、(2) NFR2.2 の合格基準（sentinel grep 0 件）は現在の改訂範囲では達成できない残存が 5 行ある。いずれも Critical（上流契約と根本的に矛盾し誤った実装を生む）には至らないが、次の Bolt が本ドキュメントの「改訂対象一覧」をそのまま作業指示として使うと空振り・未達が生じるため、着手前の裁定を承認ゲートで検討されたい。
