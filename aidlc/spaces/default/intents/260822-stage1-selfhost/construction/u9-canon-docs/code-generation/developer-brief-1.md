# developer-brief-1 — 委任 1: coding-rules / components.md / deviations.md（U9 / Bolt B4）

Conversation language: 日本語（文書本文・注記・報告はすべて日本語。型名 / API 名 / ファイル名 / ID / YAML キー / 逐語文言は英語のまま）。

## 役割と範囲

あなたは aidlc-developer-agent。Unit **u9-canon-docs**（kind: spec、Bolt B4）の委任 1 を担当する。**コードは書かない** — 所有ファイルは次の文書だけ:

- `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/use-case-rules.md`
- `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md`
- `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/error-handling.md`（**新規**）
- `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/README.md`
- `docs/specs/deviations.md`
- `aidlc/spaces/default/intents/260822-stage1-selfhost/inception/domain-design/components.md`
- 報告: `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u9-canon-docs/code-generation/developer-report-1.md`（**新規・あなただけが書く**）

それ以外のファイル（特に `docs/specs/01|10|11|12-*.md` — 委任 2 が並行して編集中、`modules/` / `tools/` / `scripts/` / `.github/` / `Cargo.*`、
`docs/specs/research/**`、計画 `code-generation-plan.md` / `unit-test-instructions.md` / `code-generation-questions.md`）は**読むだけ**。`git commit` / `git add` は
しない（コンダクタが行う）。`.claude/` 配下のツールは実行しない。

## 先に読むもの（順に）

1. `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u9-canon-docs/code-generation/code-generation-plan.md`（§1〜§5、特に §2 写像表と §5.1）
2. `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u9-canon-docs/functional-design/rules.md`（BR1.1〜BR1.4、BR3.4、BR3.5、BR4.1、BR4.2、BR5.1、BR5.2）
   と同ディレクトリの `pending-revision.md`（項目 2 = BR1.5、3 = grep 範囲 / sentinel、4 = diff スコープ）
3. `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u9-canon-docs/nfr-design/security-design.md`（§2 作法、§3 受入、§4 deviations の行）
4. `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u9-canon-docs/functional-design/functional-design-questions.md` の Q1（`error-handling.md` の
   文面ドラフト — **そのまま採用**、Q1 = A）
5. `aidlc/spaces/default/intents/260822-stage1-selfhost/inception/domain-design/decisions.md`（ADR-001 / 003 / 004 / 006 / 007 / 008 — 出典注記に使う）
6. `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/README.md` と既存ルールファイル（書式の手本: 裁定日 / 適用例 / 機械強制 / ルール / 根拠 / 対象外 等）
7. 実装の一次出典（読取のみ）: `modules/core/use-case/src/orchestration/workflow_definition_repository.rs`（`find_by_id`）、`modules/core/domain/src/orchestration/`
   のエラー型（`command_error.rs` / `apply_error.rs` / `start_error.rs` / `snapshot_error.rs` — error-handling.md の「適用例」に挙げる）

## 作業（計画 §5.1 の Step 1〜3）

**Step 1（Red）**: 次を実行し、結果（件数と行）を `developer-report-1.md` の「Red 基線」に記録する。

```bash
grep -n 'repository.load()' aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/use-case-rules.md
grep -n 'load / save' aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md
grep -n 'AuditLedgerRepository\|WorkspaceLock' aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/*.md
ls aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/*.md | grep -v README | wc -l   # 6
grep -c '^| \[' aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/README.md            # 6
grep -n '^| [0-9]' docs/specs/deviations.md                                                   # 最大 # 3
```

**Step 2（Green）**: 次の改訂を行う。各改訂箇所の末尾に出典を括弧書きで残す（例 `（C4 改訂 2026-08-23）` / `（ADR-006）` / `（ADR-007）` /
`（オーナー裁定 2026-08-23）`）。

- BR1.1 `use-case-rules.md` §4: `repository.load()` → `repository.find_by_id()`（`load` は語彙外。`find()` は C4 改訂で廃止済みなので使わない）。
- BR1.2 `gateway-taxonomy.md` §4 の「単一の Repository が集約の load / save を持つ」→「find / save」、§4 の箇条「load 済み集約を `&` 参照で渡す」
  「Controller が Repository で集約を load し」の `load` も `find_by_id` 系の言い方に、§5 末尾「load / save の指揮」→「find / save の指揮」。
  §2b 箇条 3 の例「`WorkflowDefinitionRepository::find` の not-found は…」は `find_by_id` に（C4 改訂 — `NotFound { expected, actual }` / `HarnessIdentity { path, cause }`）。
- BR1.3 `gateway-taxonomy.md` §2b に 1 段落を追加: 「ES Repository（`WorkflowExecutionRepository`）は event-store-adapter-rs 同形の `store(event, aggregate)` /
  `find_by_id` を動詞とする（§2b はステートソーシング Repository の規則であり、ES Repository の動詞は本家ライブラリの語彙に従う — ADR-006）」。
- BR1.4 `gateway-taxonomy.md` §2 の実例リストから `AuditLedger → AuditLedgerRepository` の行を削除し、「`AuditLedger` はイベントログ（`WorkflowExecution` の
  イベント列、ADR-001 / 003）であって集約ではなく、Repository を持たない — 監査シャードは ReadModelUpdater の投影」の 1 行注記に置換。
  §2 冒頭の集約表参照（01 号 §3 / 11 号 §2.1 / 12 号 §2.1）はそのまま。
- BR1.5 `gateway-taxonomy.md` §1b: 見出しを「1b. 非 Repository ポートの一般形」に改め、本文を「Repository に当てはまらない外界協調は、アウトプット契約を
  そのまま trait に表現し、**契約の意味論（予算・再入・二重解放不能 — 非 Clone のガード型など）を型に載せる**。集約ではないものを Repository に無理に
  寄せない」という一般形の記述にする。`WorkspaceLock` の具体シグネチャは削除し、末尾に 1 行「旧模範例の `WorkspaceLock`（2026-08-22 承認）は
  ADR-007 で退役（並行制御は SQLite Tx + 楽観 version）。型に意味論を載せる設計指針だけを引き継ぐ」と注記する。
  「適用例」行（ファイル冒頭）の `StateFileStore ポート削除 / StageGraphReader → WorkflowDefinitionRepository` は履歴（適用例）なので残してよい。
  「適用の帰結」節の旧→新表も履歴注記として残す（旧列の `StateFileStore` / `StageGraphReader` は sentinel 除外対象）。
- BR4.1 `error-handling.md` を新設: 文面は FD 質問票 Q1 の改訂ドラフト**そのまま**（ルール / 根拠 / 機械強制）。書式は既存ファイルに合わせ、冒頭に
  `**裁定日**: 2026-08-23（オーナー、FD Q1 = A）` / `**適用例**: Bolt B1 / B3 のエラー型（core-domain `CommandError` / `ApplyError` / `StartError` /
  `SnapshotError`、core-use-case `GraphReadError`）` / `**機械強制**: `missing_errors_doc` / `missing_panics_doc` / `unwrap_used` / `expect_used` deny
  （`Cargo.toml` workspace lints）。`thiserror` / `anyhow` 禁止は `cargo lint` ルール候補（赤例テスト必須）`。必要なら「対象外」（アダプタ層の
  message-catalog、テストコードの `unwrap`）を短く。
- BR4.2 `README.md`: 表に `[error-handling.md](error-handling.md)` の行（一言: 「失敗はモジュールごとの手実装エラー enum — 材料のみ、文言はアダプタ層、
  thiserror / anyhow 不使用」、機械強制: 上記）を追加。gateway-taxonomy 行の一言に「ES Repository は `store` / `find_by_id`」を補い、本文（§2b）と
  一致させる。行数 = 7。
- BR3.4 `docs/specs/deviations.md`: 表に # 4 の行を追加 — 文面は `security-design.md` §4 の行をそのまま（列: # / 分類 / upstream の挙動 / amadeus-ng の
  挙動 / 理由 / 記録）。「予約（決定済み・記録待ち）」節は ES 化と無関係（インストーラ）なので残す — ただし本行と重複する項目が無いことを確認して報告。
- BR3.5 `components.md`: `WorkspaceModel` エントリを `summary: "workspace 語彙 — 値オブジェクト群（Always Valid newtype）"`、`behaviour` を
  「SpaceName / CloneId / ShardName / StateFieldValue / CheckboxState / StateVersion / IntentId（UUIDv7）/ IntentDirName の値オブジェクト。状態ファイル・
  チェックボックスの描画関数は ReadModelUpdater（U4、Bolt B6）の責務へ移す — 本 Unit ではコードを動かさない（オーナー裁定 2026-08-23）」、
  `responsibilities` を値オブジェクトの提供だけに、`dependents` の ReadModelUpdater の interaction を「値オブジェクトの利用」に。`ReadModelUpdater` の
  `responsibilities` に「状態ファイル・チェックボックスの描画（旧 WorkspaceModel の純関数 — U4 で移管）」を 1 行追加。YAML の整合（インデント・引用符）を保つ。

**Step 3（Refactor）**: 出典注記の形式をそろえ、Markdown 表は見出しと同じ列数（regex 内の `\|` はエスケープ）、同一文言の見出し重複なし。Step 1 の
コマンドを再実行して緑（`repository.load()` 0、`load / save` 0、`AuditLedgerRepository` 0、`WorkspaceLock` は退役注記の行のみ、README 7 = 7、
deviations # 4 あり）。`unit-test-instructions.md` §2 の表検査スクリプトを自分の所有ファイルで走らせ `tables ok` を確認。

## 作法（厳守）

- 最小変更。逐語契約（監査イベント名 / CLI 語彙 / `AIDLC_*` / 逐語文言 / ファイル形式）の記述と `docs/specs/research/**` には触れない。
- 日本語正本、固定トークンは英語。旧記述を残すのは「旧」と明記した比較表だけ。
- 設計（rules.md の BR / 本ブリーフ）に無い判断が要ったら、推測で進めず `developer-report-1.md` の「設計質問」に書いて該当箇所は保留する。

## 報告（`developer-report-1.md`）

見出し: 「Red 基線」「改訂一覧（ファイル:節 → BR → 出典注記）」「Green / Refactor の検査結果（コマンドと出力）」「設計質問」「未了」。最終応答は
この報告の要約（日本語、10 行以内）。
