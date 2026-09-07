# nfr-design-questions — U9 正本・仕様の canon 追従（`u9-canon-docs`）

> NFR Design（Construction 3.3）の質問票（Unit: U9、kind: spec、Bolt: B4）。出典: `../nfr-requirements/security-requirements.md`（NFR1.1〜1.3 /
> NFR2.1〜2.5）、`../nfr-requirements/tech-stack-decisions.md`、`../functional-design/rules.md`（BR1.1〜BR5.2）と `../functional-design/pending-revision.md`
> （回復レビュー所見 — BR2.5 の範囲、§1b の WorkspaceLock 模範例、BR5.1 の grep 範囲、BR5.1 (d) の diff スコープ）、`../../../inception/contract-design/
> contract-summary.md`（U9 は契約面を持たない）。spec kind のため成果物は `security-design.md` / `traceability.json` の 2 つ（logical-components は作らない）。
>
> **質問なし。** 文書だけの Unit の NFR 設計は「改訂の作法と受入検査の具体化」に尽き、要求（NFR1.x / NFR2.x）から一意に決まる。次の前提を確認して
> 成果物へ進む。

## 前提（確認事項）

- P1. 改訂の作法（NFR1.1 / 1.3 / 2.5 の設計）: 各改訂は (a) 対象節を最小限に書き換え、(b) 行末または段落末に出典を括弧書き `（ADR-008 / C4 改訂 /
  Bolt B3 / オーナー裁定 2026-08-23）` で残し、(c) 逐語契約（D6）の文言・`docs/specs/research/` には触れず、(d) 日本語正本・固定トークンは英語。
  旧記述を残す場合は「旧」と明記した比較表（履歴注記）にだけ置く。
- P2. 受入検査の設計（NFR2.1 / 2.2 / 2.3 / 2.4）: PR の受入チェックリストを code-generation の計画に置き、(a) `git diff --stat origin/main..HEAD -- modules tools
  scripts .github Cargo.toml Cargo.lock` が空、(b) sentinel grep `grep -rnE 'effective_plan_action|next_in_scope_stage|AuditLedgerRepository|AuditLedgerService|StateFileStore|report_forward|gate_start' aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/*.md docs/specs/*.md`
  の結果が履歴注記（「旧」明記の比較表・禁止名テーブル）のみ、(c) README の行数 = ルールファイル数、(d) 改訂した表の列数一致（`\|` エスケープ）と見出し重複なし、
  を PR 本文に実測で貼る。(e) CodeRabbit の指摘はすべて返信・解消してから merge queue。
- P3. 逸脱登録の設計（NFR1.2）: deviations.md に 1 行 — 分類『設計変更』、upstream『状態ファイル・監査シャードのテキストファイル群を真実源とし、mkdir ロックで
  read-modify-write を直列化』、amadeus-ng『SQLite ジャーナル（journal / snapshot / checkpoint）を真実源、楽観 version で直列化、ロック dir は生成しない、
  `aidlc-state.md` / 監査シャードはリードモデルとしてバイト互換で再生成』、理由『ADR-001 / 003 / 004 / 007（NFR1 の逸脱登録）』、記録『2026-08-23 / ADR-003, ADR-007』。
  予約行（決定済み・記録待ち）の該当項目は本行へ統合。

## 再走時の前提（2026-09-07 — functional-design 差し戻し後の U9 再走、Modify）

新規の質問は無い。設計の中身は再走版 `../nfr-requirements/security-requirements.md`（NFR1.1〜NFR1.5 / NFR2.1〜NFR2.6）と実測記録
`../functional-design/gap-measurement-20260907.md`（§1〜§5、追加実測 1〜4）から一意に決まる。オーナー裁定（2026-09-07、選択肢 A）で
nfr-requirements レビュー R-02 の推奨裁定（CONSISTENCY-AUDIT を grep 範囲外 + 履歴的言及 8 行に履歴マーカー付与）が確定した。次の前提を確認して成果物へ進む。

- P4. 範囲（改訂対象の確定版）: coding-rules **20 行 9 ファイル** = 改訂 12 行 6 ファイル（BR1.6: README :50 / :115、error-handling :4、factory-naming :47 / :84、
  gateway-taxonomy :223 / :299、module-visibility :11 / :12 / :21、use-case-rules :11 / :36）+ 履歴マーカー付与 8 行 3 ファイル追加（command-query-separation :5、
  interior-mutability :5、field-visibility :46、factory-naming :5 / :99、error-handling :12、gateway-taxonomy :20 / :290 — 本文は変えず「旧」等の既定 5 語のいずれかを
  添える）。仕様 4 号（01 / 10 / 11 / 12 — BR3.3。deviations は触らない）。共有契約 3 本（components 全面 / contract-summary 節単位 / unit-of-work 注記のみ — BR3.7）
  + **`inception/domain-design/decisions.md` ADR-010 :476-478 への B13 失効注記 1 段落**（追加実測 2 — BR3.7 (d)「変更しない」の訂正）。コードは触らない。
- P5. 委譲の設計（project.md Mandated「実装は委譲」）: サブエージェント 2 派遣、書込スコープは非重複。派遣 A = coding-rules 20 行 + 10 号 + 01 号、
  派遣 B = 11 号 + 12 号 + components.md + contract-summary.md + unit-of-work.md 注記 + decisions.md ADR-010 注記。ブリーフの必須事項:
  (a) gap-measurement §2 の表を**3 列（所在 / 現行文言 / コード）丸ごと**渡す — 行番号だけを渡すと読み違える（nfr-requirements レビュー R-01 の教訓）、
  (b) 「正しい姿」の正本 = §4 台帳と §4.8 の裁き、(c) 訂正 2 件 — RMU 呼出は「`async fn`・駆動ループなし・合成ルートが await で直列に呼ぶ」と書き「同期」と書かない、
  ADR-010 の当該段落に `~~…~~ — 失効（2026-08-30 / B13、version は集約内 version() / with_version()）` を追記、(d) 作成報告は改訂 1 件ごとに根拠列
  「コードの所在 / テストの有無 / 仕様の該当節」を付ける（NFR2.6）、(e) push / PR / GitHub 書込をしない、`AIDLC_*` 環境変数でフックを回避しない、
  成果物の保存は Write / Edit 経由。メインは差分の全件レビューと受入検査（P6）の実行、統合結果の受入判断を行う。
- P6. 受入検査（PR チェックリスト、実測を PR 本文に貼る）: (1) `git diff --stat origin/main..HEAD -- modules tools scripts .github Cargo.toml Cargo.lock` が空、
  (2) sentinel 10 語の grep — 範囲は `coding-rules/*.md`（**`CONSISTENCY-AUDIT-*.md` を除く**）+ `docs/specs/*.md`（`research/` を除く）、履歴マーカー
  （同一行に `~~`、または 旧 / 失効 / 是正済み / 改名 / 履歴）の無い行が **0 件**（基線 2026-09-07 = 44 件、うち CONSISTENCY-AUDIT 4 件は範囲外化、40 件は改訂で消える）、
  (3) README 無矛盾 — 規則ファイル 22 = 表の規則行 22（表 23 行のうち good-examples 行を除く。2026-09-07 実測で一致）、各行の一言・機械強制が本文と一致、
  加えて実測で見つけた索引のずれ 2 点（README :10 の first-class-collections 追加 1 行が「規則が衝突したら」節の中に置かれている、:12「規則が 13 本」は古い件数）を
  BR4.2 で直す、(4) 改訂した表の列数一致・見出し重複なし、(5) `docs/specs/deviations.md` の diff が空（NFR1.2 は達成済みの維持）、
  (6) 共有契約 3 本 + decisions.md の既存 `## Review` 節のバイトが改訂前後で同一（NFR1.5）、(7) 4 号で gap-measurement §4.7 の未実装項目がすべて
  「予定（未実装）」表記（NFR1.4）、(8) gap-measurement §2 の『処置』列が反映され『維持』行が不変（BR5.1 (e)、メインの diff 全件レビュー）、
  (9) 用語 — 4 号・共有契約の RMU 呼出の記述に「同期」が無く `async fn` / await 直列と書かれている、(10) レビューボットの指摘全件返信・解消 + ステージレビュアー READY。
- P7. 逸脱登録（旧 P3）は 2026-08-23 の Bolt で達成済み（deviations.md に SQLite 行 1 件、実測）。本再走では「変えていないことの確認」（P6 (5)）に縮める。
- P8. 成果物は `security-design.md` / `traceability.json` の 2 点（spec kind のため logical-components は作らない）。`pending-revision.md`（2026-08-23 レビュー
  Minor 1 = `StageGraphReader` 除外の出典）は、BR5.1 (c) が `StageGraphReader` を sentinel から外したことで前提ごと閉じる。

## Consolidated Summary Confirmation

- U9 に固有の NFR 設計質問はなし。設計 = 改訂の作法（最小変更・出典注記・逐語契約不変・日本語正本・履歴注記は「旧」明記）、受入検査（diff 空・sentinel grep・
  README 行数・表整形・レビューボット全件対応）、逸脱登録の行（ADR-001 / 003 / 004 / 007）
- 成果物は security-design.md / traceability.json（logical-components は spec kind のため作らない）

**再走時の確認範囲（2026-09-07、Q4 = A の再走、Modify）**

- 2026-08-23 の要約（上）は当時の確認として保持し、やり直さない。今回は P4〜P8 の再走範囲だけを確認する。
- 成果物 2 点を Modify: `security-design.md`（§1 方針に実測基準・実装状態の正確表示・履歴保全を追加、§2 作法を再走版へ、§3 に委譲 2 派遣の書込スコープと
  ブリーフ必須事項を新設、§4 受入検査を 10 項目へ、逸脱登録の節は維持確認へ縮小、§6 要求対応表を NFR1.1〜1.5 / NFR2.1〜2.6 の 11 行へ、旧 `## Review` は
  履歴節へ）、`traceability.json`（upstream_ids を 11 ID へ、target を再走版の節へ）。`pending-revision.md` は末尾に「閉じた」注記のみ。
- 改訂対象は P4 の確定版（coding-rules 20 行 9 ファイル / 仕様 4 号 / 共有契約 3 本 / decisions.md ADR-010 注記）。コードは触らない。
- 次ステージは U9 の Code Generation（文書改訂 Bolt。委譲 2 派遣、書込スコープは P5 のとおり）。

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
