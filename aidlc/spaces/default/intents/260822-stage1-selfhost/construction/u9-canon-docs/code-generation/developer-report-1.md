# developer-report-1 — 委任 1: coding-rules / components.md / deviations.md（U9 / Bolt B4）

> `developer-brief-1.md` に基づく作業報告。計画 `code-generation-plan.md` §5.1 の Step 1（Red）→ Step 2（Green）→ Step 3（Refactor）。
> コードは書いていない（`modules` / `tools` / `scripts` / `.github` / `Cargo.*` の diff ゼロ、`docs/specs/research/**` 無変更）。
> `git add` / `git commit` は行っていない（コンダクタの責務）。

## Red 基線

改訂前（`origin/main` 相当の作業ツリー）に Step 1 のコマンドを実行した結果。

| # | コマンド | 結果（赤） |
|---|---|---|
| 1 | `grep -n 'repository.load()' coding-rules/use-case-rules.md` | **1 件** — 38 行目「Controller が `repository.load()` した集約を `&` 参照で渡す」 |
| 2 | `grep -n 'load / save' coding-rules/gateway-taxonomy.md` | **2 件** — 60 行目「単一の Repository が集約の load / save を持つ」、79 行目「load / save の指揮はユースケースが執る」 |
| 3 | `grep -n 'AuditLedgerRepository\|WorkspaceLock' coding-rules/*.md` | **4 件** — gateway-taxonomy.md:26（§1b 見出し `WorkspaceLock`）、:28（§1b 本文の具体シグネチャ）、:42（§2 実例 `AuditLedger → AuditLedgerRepository`）、:93（「適用の帰結」表の `FsWorkspaceLock` — 旧→新の履歴注記） |
| 4 | `ls coding-rules/*.md \| grep -v README \| wc -l` | **6**（`error-handling.md` 未作成） |
| 5 | `grep -c '^\| \[' coding-rules/README.md` | **6** |
| 6 | `grep -n '^\| [0-9]' docs/specs/deviations.md` | **3 行**（# 1 / # 2 / # 3。最大 # は 3、# 4 は不在） |

補足の赤（Step 2 で同時に扱った箇所）: `gateway-taxonomy.md` §2b 34 行目の例が `WorkflowDefinitionRepository::find`（C4 改訂で廃止済みの引数なし `find()`）、
§4 65・67 行目が `load` 語彙、`components.md` の `WorkspaceModel` が「語彙と純関数」のままで描画関数の移管方針が未記載。

## 改訂一覧（ファイル:節 → BR → 出典注記）

| # | ファイル:節 | BR | 改訂内容 | 出典注記（本文に残した括弧書き） |
|---|---|---|---|---|
| 1 | `coding-rules/use-case-rules.md` §4 | BR1.1 | `repository.load()` → `repository.find_by_id()` | （動詞は gateway-taxonomy §2b の許容語彙に合わせた。`find()` は廃止 — C4 改訂 2026-08-23） |
| 2 | `coding-rules/gateway-taxonomy.md` §1b | BR1.5 | 見出しを「1b. 非 Repository ポートの一般形」に変更。本文を一般形（アウトプット契約を trait に表現し、**契約の意味論を型に載せる** — 上限・締切は専用の引数型、使い切り資源は非 `Clone` のガード型）へ再構成。`WorkspaceLock` の具体シグネチャ（`acquire` / `release`）は削除し、退役の 1 行注記を末尾に追加 | （ADR-007 / 2026-08-23） |
| 3 | 同 §2b（箇条 3） | BR1.2 | 例を `WorkflowDefinitionRepository::find` → `find_by_id` に。失敗態度を実装どおり `NotFound { expected, actual }` / `HarnessIdentity { path, cause }` で明示し、引数なし `find()` の廃止を注記 | （C4 改訂 2026-08-23 / ADR-008） |
| 4 | 同 §2b（末尾に 1 段落追加） | BR1.3 | 「**ES Repository の拡張語彙**: `WorkflowExecutionRepository` は `store(event, aggregate)` / `find_by_id` を動詞とする。§2b の許容動詞一覧はステートソーシング Repository の規則であり、ES Repository の動詞は本家ライブラリ（event-store-adapter-rs）の語彙に従う」 | （ADR-006） |
| 5 | 同 §2（実例リスト） | BR1.4 | `- AuditLedger → AuditLedgerRepository` の行を削除し、「`AuditLedger` はイベントログ（`WorkflowExecution` のイベント列）であって集約ではないため Repository を持たない — 監査シャードは ReadModelUpdater の投影」の 1 行注記に置換。§2 冒頭の集約表参照（01 号 §3 / 11 号 §2.1 / 12 号 §2.1）は据え置き | （ADR-001 / 003） |
| 6 | 同 §4（散文・箇条・例） | BR1.2 | 「集約の load / save を持つ」→「find / save を持つ」。箇条「**load 済み集約を `&` 参照で渡す**」→「**`find_by_id` 済み集約を `&` 参照で渡す**」（本文の `load` も `find_by_id` へ）。直後の I8 例「Controller が load 済みの `WorkflowExecution` を…」も `find_by_id` 済みへ同期 | （設計監査 C2 / 2026-08-23） |
| 7 | 同 §5（末尾） | BR1.2 | 「load / save の指揮はユースケースが執る」→「find / save の指揮はユースケースが執る」 | （設計監査 C2 / 2026-08-23） |
| 8 | `coding-rules/error-handling.md`（**新規**） | BR4.1 | FD 質問票 Q1 = A の改訂ドラフトをそのまま「ルール」節に展開。既存書式に合わせ冒頭に `**裁定日**` / `**適用例**` / `**機械強制**`、末尾に「根拠」「対象外」（アダプタ層 message-catalog、テストコードの `unwrap` / `expect`）を置いた | 裁定日 2026-08-23（オーナー、FD Q1 = A）。適用例は core-domain `CommandError` / `ApplyError` / `StartError` / `SnapshotError`、core-use-case `GraphReadError` |
| 9 | `coding-rules/README.md` | BR4.2 | 一覧表に `[error-handling.md](error-handling.md)` の行を追加（use-case-rules 行の直後）。gateway-taxonomy 行の「一言」に「ES Repository は `store` / `find_by_id`」を補って §2b と一致させた。表の行数 **7** = ルールファイル数 **7** | （ADR-006） |
| 10 | `docs/specs/deviations.md` | BR3.4 | 表に **# 4** を追加。文面は `security-design.md` §4 の行（列: # / 分類 / upstream の挙動 / amadeus-ng の挙動 / 理由 / 記録）。「予約（決定済み・記録待ち）」節は残置 — 唯一の項目がインストーラ追加（A1 / `cargo install`）で ES・SQLite・ロックとは無関係、**# 4 と重複する項目は無い**ことを確認済み | 記録欄 `2026-08-23 / ADR-003, ADR-007（NFR1 の逸脱登録。SQLite ファイルの最終パスは U3 の設計で確定するため「相当」と記す — 確定時に本行を更新する）` |
| 11 | `inception/domain-design/components.md`（`WorkspaceModel`） | BR3.5 | `summary` を `"workspace 語彙 — 値オブジェクト群（Always Valid newtype）"` に。`behaviour` を値オブジェクト列挙（SpaceName / CloneId / ShardName / StateFieldValue / CheckboxState / StateVersion / IntentId（UUIDv7）/ IntentDirName）+ 描画関数の U4 移管方針に。`responsibilities` を値オブジェクト提供の 1 項目に縮退。`dependents` の ReadModelUpdater の `interaction` を「値オブジェクトの利用」に | （オーナー裁定 2026-08-23）を behaviour 内に明記。ADR-007 の既存注記は保持 |
| 12 | 同（`ReadModelUpdater`） | BR3.5 | `responsibilities` に「状態ファイル・チェックボックスの描画（旧 WorkspaceModel の純関数 — U4 で移管）」を 1 行追加。整合のため `depends_on` の WorkspaceModel 項の `interaction` を「値オブジェクトの利用」へ同期 | （オーナー裁定 2026-08-23） |
| 13 | 同（`## Component Summary` 表） | BR3.5 | `WorkspaceModel` 行の Purpose「語彙と純関数（ロック退役後）」→「語彙（値オブジェクト群。描画関数は U4 へ移管）」— YAML 本体との自己整合のため | — |

出典注記の形式は `security-design.md` §2 に従い `（ADR-xxx）` / `（C4 改訂 2026-08-23）` / `（設計監査 C2 / 2026-08-23）` / `（オーナー裁定 2026-08-23）` に統一した。

## Green / Refactor の検査結果（コマンドと出力）

Step 1 のコマンドを再実行（すべてリポジトリルート、`CR=aidlc/spaces/default/knowledge/aidlc-shared/coding-rules`）。

```
$ grep -c 'repository.load()' $CR/use-case-rules.md
0
$ grep -c 'load / save' $CR/gateway-taxonomy.md
0
$ grep -n 'AuditLedgerRepository\|WorkspaceLock' $CR/*.md
gateway-taxonomy.md:30:旧模範例の `WorkspaceLock`（2026-08-22 承認）は ADR-007 で退役した（並行制御は SQLite Tx + 楽観 version）。…
gateway-taxonomy.md:98:| `core_use_case::workspace::Clock` / `ProcessProbe` | … | どのユースケースも消費しない。`FsWorkspaceLock` の注入シームにすぎず、機構は Infrastructure 責務 |
$ ls $CR/*.md | grep -v README | wc -l
7
$ grep -c '^| \[' $CR/README.md
7
$ grep -c '^| 4 |' docs/specs/deviations.md
1
```

- `AuditLedgerRepository` = **0 件**（合格）。
- `WorkspaceLock` の残存 2 件はいずれも**履歴注記**: 30 行目 = BR1.5 が指示した退役注記そのもの、98 行目 = 「適用の帰結（2026-08-22 の再設計）」の旧→新移行表（ブリーフが履歴として残すよう指示した表。`FsWorkspaceLock` は理由欄の言及）。規範として `WorkspaceLock` を推す記述は残っていない。

sentinel 7 語の grep（本委任の所有範囲 = coding-rules のみ。仕様 4 号は委任 2 の範囲）:

```
$ grep -rnE 'effective_plan_action|next_in_scope_stage|AuditLedgerRepository|AuditLedgerService|StateFileStore|report_forward|gate_start' $CR/*.md
gateway-taxonomy.md:4:**適用例**: Gateway 責務再設計 PR（`StateFileStore` ポート削除 / `StageGraphReader` → `WorkflowDefinitionRepository` / …）
gateway-taxonomy.md:96:| `core_use_case::workspace::StateFileStore`（ポート） | 削除 → B-2 の `WorkflowExecutionRepository` | ポート造語 + 媒体名。… |
```

残る 2 件はどちらも履歴（4 行目 = ファイル冒頭の「適用例」、96 行目 = 旧→新移行表の**旧**列）で、ブリーフが明示的に残置を指示した箇所。`effective_plan_action` / `next_in_scope_stage` / `AuditLedgerRepository` / `AuditLedgerService` / `report_forward` / `gate_start` は coding-rules に **0 件**。

表の列数検査（`unit-test-instructions.md` §2 のスクリプトを本委任の所有ファイル 6 本で実行）:

```
$ python3 <<'EOF' … files = use-case-rules.md, gateway-taxonomy.md, error-handling.md, README.md,
                          docs/specs/deviations.md, inception/domain-design/components.md … EOF
tables ok
```

見出し重複（受入 4b、同 6 ファイル）:

```
$ for f in …; do grep -n '^#' "$f" | sed 's/^[0-9]*://' | sort | uniq -d; done
（出力なし）
```

コード変更ゼロ（受入 1 / 1b）:

```
$ git diff --stat origin/main..HEAD -- modules tools scripts .github Cargo.toml Cargo.lock
（出力なし）
$ git diff --stat -- modules tools scripts .github Cargo.toml Cargo.lock
（出力なし）
$ git status --porcelain docs/specs/research
（出力なし）
```

`components.md` の YAML 整合（インデント・引用符）:

```
$ python3 -c "yaml.safe_load(<components.md の唯一の ```yaml ブロック>)"
yaml blocks: 1
block 0 parsed ok ['OrchestrationEngine', 'WorkflowDefinitionModel', 'WorkspaceModel', 'EngineUseCases',
 'PersistenceGateways', 'ReadModelUpdater', 'CliDispatcher', 'CanonJson', 'PublishedLanguage', 'InfraIo', 'HarnessClaude']
```

作業ツリーの状態（未コミット、`git add` していない）:

```
 M aidlc/spaces/default/intents/260822-stage1-selfhost/inception/domain-design/components.md
 M aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/README.md
 M aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md
 M aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/use-case-rules.md
 M docs/specs/deviations.md
?? aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/error-handling.md
```

## 設計質問

いずれも作業を止めるものではなく、**保留した箇所は無い**（所有ファイルの BR はすべて適用済み）。コンダクタの確認を求める判断が 4 点。

1. **deviations # 4 の記録欄に U3 注記を足した**（ブリーフは「§4 の文面をそのまま」、`security-design.md` §4 の直後の散文は「行には『相当』を付けて **U3 が確定時に更新する旨を注記**」）。両立させるため、行の実質は §4 のまま、記録欄の括弧内にだけ「SQLite ファイルの最終パスは U3 の設計で確定するため『相当』と記す — 確定時に本行を更新する」を追記した。逐語一致を優先するなら削る。
2. **`FsWorkspaceLock`（gateway-taxonomy.md:98）を残した**。ブリーフの緑条件は「`WorkspaceLock` は退役注記の行のみ」だが、同じブリーフが「『適用の帰結』節の旧→新表も履歴注記として残す」と指示している。`security-design.md` §2「履歴の残し方」の定義（旧→新の比較表 = 履歴注記）で除外対象と判断して残置した。除去するなら BR の追加が要る。
3. **BR1.2 の明示 4 箇所に加えて §4 の I8 例（旧 67 行目「Controller が load 済みの `WorkflowExecution` を…」）も直した**。同一節内で `load` 語彙が残ると BR1.2 の趣旨と自己矛盾するため。同様に **components.md の `ReadModelUpdater.depends_on` の interaction と `## Component Summary` 表の `WorkspaceModel` 行**も、YAML 本体との自己整合のため BR3.5 の明示範囲を越えて同期した。いずれも語彙合わせの最小変更で、新しい規範は導入していない。
4. **委任 2 との相互参照リスク**: `gateway-taxonomy.md` §2 冒頭は集約表を 01 号 §3 / 11 号 §2.1 / 12 号 §2.1 として参照している（ブリーフ指示どおり据え置き）。委任 2 が BR2.1 / BR2.2 / BR3.1 でこれらの節番号や集約表の位置を動かす場合、この参照の追随が要る。統合時（Step 7）に節番号の一致を確認されたい。

## 未了

- 本委任の所有ファイルに対する BR1.1 / BR1.2 / BR1.3 / BR1.4 / BR1.5 / BR4.1 / BR4.2 / BR3.4 / BR3.5 は**すべて適用済み**。未着手の項目は無い。
- 範囲外（コンダクタ / 委任 2）: 仕様 4 号（`docs/specs/01|10|11|12-*.md`）の改訂と、そこを含めた sentinel grep 全体の実測（受入 2）、受入 6（CodeRabbit スレッドの全件対応）、`code-summary.md` / `traceability.json` の作成、コミット・PR。
