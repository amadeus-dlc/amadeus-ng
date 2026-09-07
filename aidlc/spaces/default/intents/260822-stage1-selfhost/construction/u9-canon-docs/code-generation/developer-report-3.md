AIDLC-UNIT: u9-canon-docs
AIDLC-TESTING-CONTRACT: sha256:303d9bb7b5d777d54a6761be9ed154d85d5bb3f2d6b9cce02f71f4ed1b3a4ff3

# developer-report-3 — 派遣 A: coding-rules 9 ファイル + 仕様 10 号 + 01 号（U9 再走、2026-09-07）

> 出典: `developer-brief-3.md`（作業指示）、`code-generation-plan.md` §5.1（Step 1〜3）、
> `unit-test-instructions.md` §1 (2)(3)(4)(7)(9)、`../nfr-design/security-design.md` §2（作法）/ §4（受入）、
> `../functional-design/gap-measurement-20260907.md` §2.1 / §2.4 / §2.5 / §4 台帳 / §4.8、`../functional-design/rules.md`
> （BR1.6 / BR3.3 / BR3.6 / BR4.2 / BR5.1 / BR5.2 / BR5.3）。
> コードは 1 行も変更していない（`git diff --stat -- modules tools scripts .github Cargo.toml Cargo.lock formal docs/specs/research docs/specs/deviations.md` が空）。

## 1. Red 基線（Step 1、改訂前）

コマンドはブリーフ §手順 Step 1 のとおり、所有ファイルに絞って実行した。

### (1) sentinel 10 語 grep（履歴マーカー除外）— **27 件**

```
grep -rnE 'effective_plan_action|next_in_scope_stage|AuditLedgerRepository|AuditLedgerService|StateFileStore|report_forward|gate_start|WorkflowExecution|RehydratedWorkflowExecution|message-catalog' \
  <所有 coding-rules 9 ファイル> docs/specs/10-orchestration.md docs/specs/01-domain-model.md \
  | grep -vE '~~|旧|失効|是正済み|改名|履歴'
```

内訳はブリーフの見込み（coding-rules 12 + 10 号 9 + 01 号 6 = 27）と**完全一致**した。

| 区分 | 件数 | 行 |
|---|---|---|
| coding-rules | 12 | `error-handling.md:12` / `README.md:50` / `README.md:115` / `factory-naming.md:5` / `:47` / `:84` / `:99` / `gateway-taxonomy.md:20` / `:290` / `command-query-separation.md:5` / `interior-mutability.md:5` / `field-visibility.md:46` |
| `docs/specs/10-orchestration.md` | 9 | `:6` / `:31` / `:43` / `:70` / `:85` / `:101` / `:110` / `:137` / `:236` |
| `docs/specs/01-domain-model.md` | 6 | `:6` / `:96` / `:106` / `:184` / `:231` / `:278` |

### (2)(3)(4) その他の基線

| 検査 | コマンド | 基線（赤） | 期待（緑） |
|---|---|---|---|
| 規則ファイル数 | `ls coding-rules/*.md \| grep -vE 'README\|good-examples\|CONSISTENCY-AUDIT' \| wc -l` | 22 | 22（不変） |
| README 表のリンク行 | `grep -cE '^\| \[' README.md` | 23（good-examples 1 を含む → 規則行 22） | 23（不変） |
| README 索引ずれ | `grep -nE '規則が [0-9]+ 本\|^2026-[0-9-]+追加:' README.md` | **2 行** — `:10`（`2026-09-06追加:` の告知行が「規則が衝突したら」節の中に混入）と `:12`（「規則が 13 本」= 古い件数） | 「規則が 22 本」の 1 行のみ |
| 実装状態の表記 | `grep -cE '予定（未実装' 10 号 01 号` | 10 号 0 / 01 号 0 | 各 1 以上 |
| 用語（RMU 呼出の「同期」） | `grep -nE '同期' 01 号 10 号 \| grep -iE 'rmu\|catch_up\|投影'` | 0 件 | 0 件（維持） |

## 2. 改訂一覧（Step 2 Green / Step 3 Refactor）

改訂 1 件ごとに 1 行。「根拠」列は BR5.3 の要求どおり **コードの所在（path:line または型・関数名）/ テストの有無 / 仕様の該当節** を書く。
コードの path は `modules/` 起点、行番号は 2026-09-07 の作業ツリー（`origin/main` の `modules/` と diff ゼロ）実測。

### 2.1 coding-rules — 改訂 12 行 6 ファイル（BR1.6 / BR4.2）

| # | ファイル:節 | BR | 改訂内容 | 出典注記 | 根拠（コードの所在 / テスト / 仕様節） |
|---|---|---|---|---|---|
| A1 | `README.md` 一覧表 error-handling 行（旧 :50） | BR1.6 / BR4.2 / FR9.6 | 「利用者向け文言はアダプタ層（message-catalog）」→「利用者向け文言は**出す側の `wording` モジュール**（合成ルート `aidlc` と RMU の投影ライタ）」。本文 `error-handling.md` の該当箇条と同じ文言に揃えた | `（2026-08-29 の文言カタログ解体後の形 / 実測 app/aidlc/src/wording.rs・read-model-updater/src/workspace/wording.rs）` | `modules/app/aidlc/src/wording.rs`（実在）/ `modules/core/read-model-updater/src/workspace/wording.rs`（実在）。テスト = 両モジュールにインライン `#[cfg(test)]` あり（本 Unit では実行せず存在のみ確認）。仕様: `error-handling.md` §ルール 4 箇条目、台帳 P7 |
| A2 | `README.md` 「規則が衝突したら」節の冒頭（旧 :10） | BR4.2 | first-class-collections の追加告知 1 行を節から**除去**し、一覧表の first-class-collections 行へ「裁定日 2026-09-06」として畳んだ。表の規則行は 22 のまま | `（裁定日 2026-09-06 — 従来この告知は「規則が衝突したら」節の中に置かれていた）` | コード根拠なし（README 索引の自己整合。ファイル実測 `ls` = 22 本と表の 22 行が一致）。仕様: BR4.2「一覧の行数 = 規則ファイル数」 |
| A3 | `README.md` 同節本文（旧 :12） | BR4.2 | 「規則が 13 本になり」→「規則が **22** 本になり」 | `（実測 ls coding-rules/*.md、README / good-examples / CONSISTENCY-AUDIT を除いて 22）` | 同上（ファイル数実測）。仕様: BR4.2 |
| A4 | `README.md` 機械化ロードマップ §優先順 1 行（旧 :115） | BR1.6 / BR4.2 | 「是正対象は `WorkflowExecutionState` の 1 型」を打消し線 + **是正済み（型ごと消滅）**へ。現行の再構成経路（`replay` / `new`）を添えた | `（B13 2026-08-30、型ごと消滅 / 実測 intent_execution.rs:352）` | `modules/core/command/domain/src/orchestration/intent_execution.rs:352`（`replay`）/ `:290`（`new`）/ `:319`（唯一の構造体リテラル）。`grep -rn 'WorkflowExecutionState' modules/` = 0 件。テスト: `intent_execution.rs:5177` `the_version_the_store_assigned_is_stamped_after_reconstruction` ほか同ファイル内テスト。仕様: gap-measurement §5 の追加実測、台帳 O9 |
| A5 | `error-handling.md` §適用例（旧 :4） | BR1.6 | 「core-domain `CommandError` / `ApplyError` / `StartError` / `SnapshotError`」→ `core-command-domain` の手実装エラー enum **33 本**（代表 12 型を列挙）。`StartError` / `SnapshotError` は不存在として打消し線で残した | `（実測 grep -rn 'pub enum [A-Za-z]*Error' modules/core/command/domain/src/ = 33 / B12・B13 2026-08-30 で失効）` | `grep -rn 'pub enum [A-Za-z]*Error' modules/core/command/domain/src/` = **33**。`grep -rn 'StartError\|SnapshotError\|GraphReadError' modules/` = **0**。列挙した 12 型はいずれも `modules/core/command/domain/src/{orchestration,workflow_definition,workspace}/*_error.rs` に実在。テスト: 各エラー型のファイルにインラインテストあり。仕様: `error-handling.md` 本文（BR4.1 で現行本文が正本）、gap-measurement §2.5 |
| A6 | `factory-naming.md` §反例（旧 :47） | BR1.6 | 「次 Bolt で是正」→ **是正済み**へ履歴化（打消し線 + 現行の再構成経路） | `（B13 2026-08-30、型ごと消滅 / 実測 intent_execution.rs:290,319,352）` | A4 と同じ。加えて構造体リテラル `IntentExecution { .. }` が `new` の中の 1 箇所（:319）だけであることを `grep -n 'IntentExecution {' modules/core/command/domain/src/orchestration/*.rs` で確認。仕様: `factory-naming.md` §「構造体リテラルは型ごとに 1 箇所」 |
| A7 | `factory-naming.md` §`with_*`（旧 :84） | BR1.6 | 例 `WorkflowExecution::with_version` → `IntentExecution::with_version` / `WorkflowDefinition::with_version`（`ScopeMetadata::with_depth` は維持）。3 件とも行番号を添えた | `（B12 2026-08-30 の集約分割・改名に追従 / 実測 intent_execution.rs:254・workflow_definition.rs:273・scope_metadata.rs:45）` | `grep -rn 'fn with_version\|fn with_depth' modules/` の実測 3 件（+ 私有の `state_version_classification.rs:98`、`start_request.rs:39`、`next_turn_input.rs:96` は別文脈）。テスト: `intent_execution.rs:5181` が `with_version(7)` を検査。仕様: `factory-naming.md` §「ビルダーの鎖メソッド」 |
| A8 | `gateway-taxonomy.md` §4 改訂注記（旧 :223） | BR1.6 | 書込モデルの集約名 `WorkflowExecution` → `IntentExecution`（`EventStore` の語は最小変更のため据え置き） | `（集約名は B12 2026-08-30 の分割・改名に追従 / 実測 intent_execution.rs）` | `modules/core/command/domain/src/orchestration/intent_execution.rs`（`pub struct IntentExecution` :132）。テスト: 同ファイル内多数。仕様: 10 号 §2.1、台帳 O1 |
| A9 | `gateway-taxonomy.md` §機械強制の候補 2（旧 :299） | BR1.6 | 照合先クレート `core-domain` → `core-command-domain` | `（クレート名は CQRS のクレート分離に追従 / 実測 modules/core/command/domain/Cargo.toml name = "core-command-domain"）` | `grep -n 'name = ' modules/core/command/{domain,use-case,interface-adapter}/Cargo.toml` の実測 3 件。テストなし（未実装のルール候補）。仕様: 台帳 K1（クレート 10） |
| A10 | `module-visibility.md` §ルール 2 箇条目（旧 :11） | BR1.6 | `core_domain::{workspace, orchestration, workflow_definition}` → `core_command_domain::{orchestration, workflow_definition, workspace}`、`lib.rs` の行番号を添えた | `（実測 modules/core/command/domain/src/lib.rs:29-31 の pub mod）` | `modules/core/command/domain/src/lib.rs:29-31`（`pub mod orchestration / workflow_definition / workspace`）。テスト: 該当なし（可視性はビルドで強制）。仕様: `module-visibility.md` §ルール |
| A11 | `module-visibility.md` 同（旧 :12） | BR1.6 | 例 `message_catalog::{state, lock, bolt}` / `infra_io::{...}` → `core_infrastructure::{canon_json, collections, codec, atomic, append_only, fs_meta, secret_file}`。旧例は「現存しない」として残した | `（実測 modules/core/infrastructure/src/lib.rs:20-26 / 文言カタログは 2026-08-29 に解体）` | `modules/core/infrastructure/src/lib.rs:20-26`（`pub mod` 7 本）。`grep -rn 'infra_io\|message_catalog' modules/` = 0 件。テスト: 各モジュールにインラインテストあり。仕様: 台帳 K4（infrastructure 層は言語拡張のみ） |
| A12 | `module-visibility.md` §昇格の運用（旧 :21） | BR1.6 | `core_domain::workspace::CheckboxState` → `core_command_domain::workspace::CheckboxState` | `（実測 modules/core/command/domain/src/workspace/checkbox_state.rs）` | `modules/core/command/domain/src/workspace/checkbox_state.rs`（実在）。テスト: 同ファイルにインラインテストあり。仕様: 11 号 §2.2（`CheckboxState` の所有元）、台帳 S1 |
| A13 | `use-case-rules.md` §1 DIP（旧 :11） | BR1.6 | クレート名 `core-use-case` / `core-interface-adapter` → `core-command-use-case` / `core-command-interface-adapter`。**dev-dependency にも書けない**ことを追記した（T1 の 3 層構造の前提） | `（実測 modules/core/command/use-case/Cargo.toml: dependencies = core-command-domain / chrono、dev-dependencies = tokio）` | `modules/core/command/use-case/Cargo.toml`（`core-command-interface-adapter` は `[dependencies]` にも `[dev-dependencies]` にも無い）。根拠の一次出典は `modules/core/command/use-case/src/orchestration/test_support.rs:1-17` の doc。テスト: `test_support.rs` を使う全ユースケーステスト（例 `commit_verdict_use_case.rs`）。仕様: `use-case-rules.md` §1、台帳 K2 |
| A14 | `use-case-rules.md` §2 スタティックバインディング（旧 :36） | BR1.6 | 「実装は実質 2 つ（Impl + InMemory）… `XxxUseCase<InMemoryXxxRepository>`」→ **ポート実装の 3 層**（① 公開の `XxxRepositoryImpl<S>::in_memory()` が正 ② use-case クレートの `#[cfg(test)]` フェイク 4 つは DIP 制約下の単体テスト専用 ③ クエリ側の `InMemoryXxxDao` 13）を新設。理由（T1）を明記した | `（オーナー裁定 2026-08-31 / ADR-010 / 実測 intent_execution_repository_impl.rs:182・test_support.rs・query/interface-adapter/src/memory/）` | ① `modules/core/command/interface-adapter/src/orchestration/intent_execution_repository_impl.rs:182` / `intent_repository_impl.rs:156` / `workflow_definition_repository_impl.rs:177`。② `modules/core/command/use-case/src/orchestration/test_support.rs:241,388,622,770`（`pub(crate) struct InMemory*`、`orchestration/mod.rs:38-39` で `#[cfg(test)]`）。③ `grep -rn 'pub struct InMemory.*Dao' modules/core/query/interface-adapter/src/` = **13**。テスト: `port/intent_execution_repository.rs:110-210` の契約テスト 7 本がフェイク経由。仕様: 10 号 §3、台帳 P4（**本報告 §4 の保留 1 で台帳との差を申告**） |

### 2.2 coding-rules — 履歴マーカー付与 8 行 3 ファイル（BR1.6、本文の文意は不変）

| # | ファイル:節 | 付与したマーカーと形 | 根拠 |
|---|---|---|---|
| A15 | `command-query-separation.md` §適用例（旧 :5） | 行末に `（履歴: 旧名 — B12 2026-08-30 で IntentExecutionRepository に改名済み）` | `modules/core/command/use-case/src/orchestration/port/intent_execution_repository.rs`（現行ポート名）。仕様: 台帳 P1 |
| A16 | `interior-mutability.md` §適用例（旧 :5） | 見出しを `**適用例**（履歴 — 旧名を含む是正の記録）` にし、行末に改名注記 | 同上。加えて `port/intent_execution_repository.rs` の doc「レシーバは CQS に従う」が現行の形。仕様: 台帳 P1 / P4 |
| A17 | `field-visibility.md` §射程前の実測段落（旧 :46） | 型名の直後に `（履歴 — この型は B13 2026-08-30 でメメントごと廃止され現存しない）` を挿入。**同一行に載ることを grep で確認**した（初回は改行で 2 行に割れてマーカーが外れたため再修正） | `grep -rn 'WorkflowExecutionState' modules/` = 0 件。仕様: 台帳 O9 |
| A18 | `factory-naming.md` §適用例（旧 :5） | 行末に `（履歴: 旧名 WorkflowExecution は B12 2026-08-30 で Intent + IntentExecution へ分割・改名され、現行は IntentExecution::start — 実測 intent_execution.rs:225）` | `modules/core/command/domain/src/orchestration/intent_execution.rs:225`（`pub fn start`）。仕様: 10 号 §2.1、台帳 O4 |
| A19 | `factory-naming.md` §実測表 2 行目（旧 :99） | セル内に `（履歴 — 型ごと消滅、B13 2026-08-30）` | A4 と同じ。仕様: 台帳 O9 |
| A20 | `error-handling.md` §ルール 4 箇条目（旧 :12） | 括弧内を `旧 message-catalog を 2026-08-29 に解体した後の形。履歴` に整形（**逐語 `message-catalog` は保存**し、マーカー語だけ添えた） | `modules/app/aidlc/src/wording.rs` / `modules/core/read-model-updater/src/workspace/wording.rs`。仕様: 台帳 P7、BR4.1 |
| A21 | `gateway-taxonomy.md` §適用例（旧 :20） | 見出しを `**適用例**（履歴 — 旧名を含む適用の記録）` に | `grep -rn 'StateFileStore\|StageGraphReader' modules/` = 0 件。仕様: BR5.1 (c)（`StageGraphReader` は sentinel 外） |
| A22 | `gateway-taxonomy.md` §適用の帰結 表 1 行目（旧 :290） | セル末尾に `（履歴 — 移行先の名は B12 2026-08-30 で IntentExecutionRepository に改名済み）` | 同上 + `port/intent_execution_repository.rs`。仕様: 台帳 P1 |

### 2.3 `docs/specs/10-orchestration.md`（BR3.3）

| # | 節 | 改訂内容 | 出典注記 | 根拠（コードの所在 / テスト / 仕様節） |
|---|---|---|---|---|
| B1 | 冒頭注記（旧 :3-15、14 行） | B12 読み替え注記と B13 優先順位注記を**本文へ畳み込み**、「追従済み（2026-09-07 / U9 再走）」の 1 行に圧縮（旧注記の要旨は打消し線で 1 行残す）。coding-rules 優先の規範は維持 | `（2026-09-07 / U9 再走で追従を完了）` | 本 Bolt の成果そのもの。仕様: BR3.3 (i) |
| B2 | §1 B1（旧 :31） | 合成の所有者 `WorkflowExecution` → `IntentExecution` | `（集約名は B12 2026-08-30 に追従 / 実測 intent_execution.rs:544 / IntentExecution::effective_plan）` | `intent_execution.rs:544`。テスト: 同ファイル proptest「`effective_plan` 合成」。仕様: 台帳 O10。**注**: この行は gap-measurement §2.1 の表に無いが、受入 (2) が 0 件を要求するため名称のみ改訂した（§4 保留 3） |
| B3 | §2.1 見出しと導入（旧 :43-46） | 「集約: `WorkflowExecution`」→「集約: `Intent` と `IntentExecution`」。2 集約への分割・1 intent : n 実行・ID 参照・取り違えガード（`IntentMismatch`）を導入段落に置き、`(a)` / `(b)` の小節（太字ラベル。新しい見出しレベルは作らない）に分けた | `（B12 2026-08-30 / b51 2026-09-07 / coding-rules/aggregate-references.md）` | `intent.rs:54-62`（`Intent` 7 属性）、`intent_execution.rs:132-180`（12 属性）、`intent_execution.rs:1864` の `matches(intent)` ガード。テスト: `intent_execution.rs` 内の `IntentMismatch` 検査。仕様: 台帳 O1 / O2 / O3 |
| B4 | §2.1 (a) `Intent`（新設段落） | 7 属性・genesis `Intent::create` の完全署名・イベント `IntentEvent::Created` 1 変種・`replay`・クエリ `resolve_review_policy` を記述 | `（実測 intent.rs:54-62 / :114 / intent_event.rs:37-40）` | `intent.rs:54-62` / `:114`（`create` の 5 引数と `Result<(Intent, IntentEvent), IntentError>`）/ `:69`（`resolve_review_policy`）、`intent_event.rs:37-40`（1 変種）。テスト: `intent.rs` インラインテスト。仕様: 台帳 O2 / O7 |
| B5 | §2.1 (b) 状態（旧 :48） | 「16 属性・`version` は集約の外・`RehydratedWorkflowExecution` が持ち回る・`stages: Vec<StageEntry>` / `plan` / `conditional` 展開列」→ **12 属性**（`slots: StageSlots` を含む全列挙）。`version` は**集約の内側**、`Rehydrated*` は失効（0 件）。静的な計画を持たないことを独立の箇条にした | `（実測 intent_execution.rs:132-180 / :254 / port/mod.rs:15-18 / ADR-010・B13 2026-08-30 / オーナー裁定 2026-08-30）` | `intent_execution.rs:132-188`（12 フィールド）、`:254`（`with_version`）、`:197`（`UNPERSISTED_VERSION`）、`stage_slot.rs:22-30`（`StageSlot` 7 欄）、`port/mod.rs:15-18`（「すべて廃止済み」）。`grep -rn 'Rehydrated' modules/` = 型 0 件。テスト: `intent_execution.rs:5177-5187`。仕様: 台帳 O3 / O8 |
| B6 | §2.1 (b) コマンド（旧 :50） | 「コマンド（11）」→ **`&mut self` で 15 ＋ genesis `start`**。追加 5 件（`record_single_stage_run` / `record_skeleton_stance` / `request_review` / `record_review_verdict` / `affirm_practices`）を Bolt 番号つきで列挙し、全 16 の行番号を添えた。`start` が静的関数であることを明記 | `（b47 / b48 / b49、実測 intent_execution.rs:225,794,840,866,890,911,943,981,1011,1033,1140,1214,1252,1317,1388,1434）` | 上記 16 行すべて `grep -nE '    pub fn ...' modules/core/command/domain/src/orchestration/intent_execution.rs` で実測。テスト: 各コマンドに対応するインラインテストあり。仕様: 台帳 O4 |
| B7 | §2.1 (b) ドメインイベント（旧 :51） | 「ドメインイベント（11）」→ **`IntentExecutionEvent` 16 変種**。追加 5 件を Bolt 番号つきで列挙。監査語彙の `EventType::StageCompleted` が別物として残ることと、`Intent` 側が 1 変種であることを注記。旧 manifest 綴りの打消し線は**触っていない**（履歴） | `（実測 intent_execution_event.rs:80-112 / audit_events.rs:83）` | `intent_execution_event.rs:80-112`（16 変種）、`workspace/audit_events.rs:83`（`StageCompleted = "STAGE_COMPLETED"`）、`intent_event.rs:37-40`（1 変種）。テスト: `intent_execution_event.rs` インラインテスト。仕様: 台帳 O5 / O6、§4.8「イベント変種数 = 16」 |
| B8 | §2.1 (b) 再構成（旧 :52 メメント） | 「メメント `state()` / `from_state()` / `WorkflowExecutionState` / serde は memento 経由」→ **メメント型は無い**。差分再生 `replay(snapshot, events)`・復号検査点 `IntentExecution::new`（アダプタの DTO から）・壊れた歴史は panic（`# Panics` 3 + 1 か所）の 3 箇条に書き直した | `（B13 2026-08-30 型ごと廃止 / 2026-09-05 差分再生 / オーナー裁定 2026-08-30 / 実測 intent_execution.rs:352,290,319,347,1506,2364・workflow_definition.rs:213・intent_execution_repository_impl.rs:331-439）` | 左記すべて実測。`grep -n '# Panics'` で 3 + 1 か所を確認。テスト: `intent_execution_repository_impl.rs` の再構成テスト群、`modules/core/command/domain/tests/engine_loop_conformance.rs`（ITF 準拠。同ディレクトリのもう 1 本は `collection_contract_test.rs`）。仕様: 台帳 O9、§4.8「再構成方式 = 差分再生」 |
| B9 | §2.1 (b) `next_decision`（旧 :55） | 「失敗しないクエリ `(&self, &NextRequest) -> NextDecision`」→ `(&self, &Intent, &NextRequest) -> Result<NextDecision, CommandError>`（`IntentMismatch`）。b38 の旧文と、さらに旧い `&WorkflowDefinition` 版の 2 段の失効を打消し線で残した。`jump_resolve` の同形ガードも添えた | `（b47 2026-09-04 で &Intent 追加 / b51 2026-09-07 で Result 化 / 実測 intent_execution.rs:1864・:1788）` | `intent_execution.rs:1864-1872`（署名と `matches` ガード）、`:1788-1795`（`jump_resolve`）。テスト: 同ファイルの `IntentMismatch` テスト。仕様: 台帳 O10、§4.8「`next_decision` の署名」 |
| B10 | §2.1 (b) Tx 境界（旧 :56） | **維持**（改訂なし） | — | `intent_execution_repository_impl.rs` の `persist_event_and_snapshot`。gap-measurement §2.1 の『維持』行 |
| B11 | §2.2 表（旧 :66-76） | 見出しに **「所在（実測 2026-09-07）」列を追加**（3 列 → 4 列）。逐語契約（kind の列挙・JSON 形・28KiB）は 1 バイトも変えていない。`Directive` に「構築可能な変種は 7」を追記、`EffectivePlan` の集約名を `IntentExecution` へ、`ProgressSignature` を **予定（未実装、クリティカルパス 4）**、`DirectiveMaxBytes` を「型ではなく合成ルートの定数」と明記 | `（b26 段階2 2026-08-31 でクエリ側へ移設 / 実測 directive_schema.rs:11・directive.rs:26・continue_token.rs:27・verdict.rs:9・autonomy_mode.rs:12・skeleton_stance.rs:9・jump_direction.rs:6・intent_execution.rs:544・app/src/presenter.rs:62）` | 左記すべて実測。`grep -rn 'ProgressSignature\|progress_signature' modules/` = **0 件**。`Directive` の構築可能変種は `directive.rs:26-55` で 7（`LoadSteering` / `RunStage` / `Ask` / `Print` / `Error` / `Done` / `Parked`）。テスト: `directive_schema.rs` / `presenter.rs:493-514`（28KiB 拒否）。仕様: 台帳 O15。**保留 2 参照** |
| B12 | §2.3 表と位置づけ注記（旧 :82-87） | `next_decision` 行を現行署名（8 変種の列挙つき）へ、`jump_resolve` 行を `(&Intent, StageIndex) -> Result<...>` へ、注記の集約名を `IntentExecution` へ。b38 当時の「他集約の引数は不要」という追記を打消し線で失効させ、現行署名を書いた | `（b47 / b51、実測 intent_execution.rs:1864・:1788・next_decision.rs:16-43）` | `next_decision.rs` の 8 変種（`RunStage` / `Done` / `Parked` / `UnparkThenResume` / `ResumeMenu` / `NewWorkRouting` / `RecoverSkipInconsistency` / `InconsistentSkip`）。テスト: 同ファイル。仕様: 台帳 O10 |
| B13 | §3 ユースケース列（旧 :93） | 実装名へ全面差し替え — **コマンド側 9**（`CommitVerdict` / `CreateIntent` / `DefineWorkflow` / `Park` / `PromotePractices` / `RecordReview` / `RecordSingleStageRun` / `RecordSkeletonStance` / `SwitchAutonomy`）、**クエリ側 13**（`Find*`）、**予定（未実装、クリティカルパス 4）**（`Unpark` / `JumpResolve` / `JumpExecute` / `Recompose`）、**予定（未実装、swarm はスコープ外）**（Bolt 8 動詞 / Swarm 3 動詞）。`Next` / `Continue` の失効を明記 | `（b26 段階2 2026-08-31 / b47 / b48 / b49 / b50、実測 command/use-case/src/orchestration/*_use_case.rs = 9・query/use-case/src/orchestration/find_*_use_case.rs = 13）` | `ls modules/core/command/use-case/src/orchestration/*use_case.rs` = **9 本**（`commit_verdict` / `create_intent` / `define_workflow` / `park` / `promote_practices` / `record_review` / `record_single_stage_run` / `record_skeleton_stance` / `switch_autonomy`）。`grep -rn 'pub struct Find[A-Za-z]*UseCase' modules/core/query/use-case/src/` = **13**。unpark / jump / recompose のユースケースは `ls` で 0 件。テスト: 各ユースケースにインラインテストあり（例 `commit_verdict_use_case.rs:317-344`）。仕様: 台帳 P5 / P6、§4.7 |
| B14 | §3 テストダブルの記述（旧 :99-101） | 「1 trait 1 Impl（`XxxRepositoryImpl` ＋ `InMemoryXxxRepository`）」「`WorkflowDefinitionRepository` は `InMemoryWorkflowDefinitionRepository` を持つ」→ **T1 の 3 層構造**（A14 と同じ内容を仕様の言葉で）。B6 補足ブロックは打消し線で残さず、3 層の箇条へ畳み込んだ | `（ADR-010 / B6 2026-08-27 / オーナー裁定 2026-08-31、実測 *_repository_impl.rs:182,156,177・test_support.rs・query/interface-adapter/src/memory/）` | A14 と同じ。加えて `test_support.rs:1-17` の doc が理由の一次出典。テスト: `port/intent_execution_repository.rs:110-210` の契約テスト。仕様: 台帳 P4。**保留 1 参照** |
| B15 | §3 ポート表 1 行目（旧 :105） | `WorkflowExecutionRepository` → `IntentExecutionRepository`。`store(.., expected_version)` と再水和レコードを打消し線で失効させ、現行 2 動詞の完全署名・CQS のレシーバ規約・「版は集約が運ぶ」を書いた。I8 の型強制も b26 段階2 で失効（クレート分離で保証）と明記 | `（ADR-010 / B13 2026-08-30 / b26 段階2 2026-08-31、実測 port/intent_execution_repository.rs:68-97・intent_execution_repository_impl.rs:90,455）` | `port/intent_execution_repository.rs:68-97`（`find_by_id(&self, &IntentExecutionId)` / `store(&mut self, &IntentExecutionEvent, &IntentExecution)`、`expected_version` 引数なし）。テスト: 同ファイル `mod tests` 7 本。仕様: 台帳 P1 / P8 |
| B16 | §3 ポート表 2 行目（旧 :106） | `WorkflowDefinitionRepository` に `find_for_intent` と `store` を追記し、「`save` は持たない」と `GraphReadError` を打消し線で失効させ `RepositoryError<WorkflowDefinitionId>` 4 変種へ。実装欄のテストダブル記述も 3 層へ | `（b30 / W1 2026-09-02 / 2026-08-31 b26 段階2、実測 port/workflow_definition_repository.rs:77-115・workflow_definition_repository_impl.rs:85,177）` | `port/workflow_definition_repository.rs:77`（`find_by_id`）/ `:91`（`find_for_intent(&Intent)`）/ `:112`（`store`）、`port/repository_error.rs:27-55`（4 変種）。テスト: 同ポートの契約テスト。仕様: 台帳 P1 / P2 / W1 |
| B17 | §3 ポート表（**2 行追加**） | `IntentRepository`（3 動詞）と `CompiledDefinitionRepository`（2 動詞）を新規行として追加。実装型と行番号を添えた | `（B12 の集約分割 / b36 の配布束昇格、実測 port/intent_repository.rs:46-80・port/compiled_definition_repository.rs:50-72・intent_repository_impl.rs:71,156・compiled_definition_repository_impl.rs:408）` | 左記すべて実測。テスト: 各ポートのインライン契約テスト。仕様: 台帳 P1、gap-measurement §2.1 の「（欠落）」行 |
| B18 | §3 表から落とした 2 行（旧 :110） | 監査台帳の集約名 `WorkflowExecution` → `IntentExecution` | `（集約名は B12 2026-08-30 に追従）` | `intent_execution_event.rs`（イベントログ）。仕様: 台帳 O5 |
| B19 | §6 不変条件 I8（旧 :137） | 「`Next` に `WorkflowExecutionRepository` を注入せず型強制」を打消し線で失効させ、`FindNextAnswerUseCase` への移設とクレート分離による物理強制へ | `（b26 段階2 2026-08-31、実測 find_next_answer_use_case.rs:34）` | `modules/core/query/use-case/src/orchestration/find_next_answer_use_case.rs:34`。コマンド側に `Next` ユースケースは 0 件。テスト: 同ファイル。仕様: 台帳 K2 / P6 |
| B20 | §8 実装順序 2（旧 :205） | 集約名を `Intent` / `IntentExecution` へ。proptest の対象から `ProgressSignature` を外し **予定（未実装、クリティカルパス 4）** と明記 | `（B12 2026-08-30 / 実測 grep ProgressSignature = 0 件）` | `grep -rn 'ProgressSignature' modules/` = 0。仕様: §2.2（B11） |
| B21 | §8 実装順序 3（旧 :206） | in-memory Gateway を `XxxRepositoryImpl<S>::in_memory()` に統一し、`InMemoryWorkflowDefinitionRepository` が use-case クレートの `#[cfg(test)]` フェイクである旨を注記。「§3 のポートは 2 本」を失効させ **4 本**へ | `（2026-09-07 / U9 再走、実測 port/ = 4 ファイル）` | `ls modules/core/command/use-case/src/orchestration/port/` = 4 ポート + `mod.rs` + `repository_error.rs`。仕様: 台帳 P1 |
| B22 | §9 S1（旧 :236） | 遷移実装の集約名 `WorkflowExecution` → `IntentExecution` | `（集約名は B12 2026-08-30 に追従）` | `intent_execution.rs`。仕様: 台帳 O4 |
| B23 | §7 導入・§2.1 末尾・§7.1 HOLD-MERGE | **予定（未実装）表記の追加**: §7 全体を「予定（未実装、swarm はスコープ外）」、`Bolt` / `SwarmBatch` を同、`OpaqueFlagStore` を「予定（未実装、クリティカルパス 4）」 | `（2026-09-07 実測 — 該当する集約・サーガ・供給面はコードに無い）` | `grep -rn 'SwarmBatch\|OpaqueFlagStore' modules/` = 0 件。仕様: §4.7、作法「実装状態の表記」 |

### 2.4 `docs/specs/01-domain-model.md`（BR3.3 / BR3.6）

| # | 節 | 改訂内容 | 出典注記 | 根拠（コードの所在 / テスト / 仕様節） |
|---|---|---|---|---|
| C1 | 冒頭注記（旧 :3-15） | 10 号と同形に畳み込み（「追従済み（2026-09-07 / U9 再走）」1 行） | `（2026-09-07 / U9 再走で追従を完了）` | 本 Bolt の成果。仕様: BR3.3 (i) |
| C2 | §3.1 状態機械（旧 :96） | 合成読みの所有者 `WorkflowExecution` → `IntentExecution` | `（集約名は B12 2026-08-30 に追従 / 実測 intent_execution.rs:544）` | `intent_execution.rs:544`。テスト: proptest「`effective_plan` 合成」。仕様: 台帳 O10 |
| C3 | §3.2 集約段落（旧 :102） | 段落を全面的に書き直し — 2 集約への分割、`Intent` 7 属性 + `Created` 1 変種、`IntentExecution` 12 属性 + コマンド 15 + genesis + イベント 16 変種、version は集約の内側、メメント型は無い（4 箇条）。旧 17/16 属性・12 種・`WorkflowExecutionState`・`Rehydrated*` はすべて打消し線で失効 | `（B12・B13 2026-08-30 / b47〜b49 / ADR-008・ADR-010、実測 intent.rs:54-62,114・intent_execution.rs:132-188・intent_execution_event.rs:80-112・intent_event.rs:37-40）` | B3〜B8 と同じ実測。テスト: `modules/core/command/domain/tests/engine_loop_conformance.rs`（ITF 準拠）+ 各集約のインラインテスト。仕様: 10 号 §2.1、台帳 O1〜O9 |
| C4 | §3.2 Domain Primitive 候補（旧 :106） | `WorkflowExecution::start` が `StageDisplay` を焼き込む → **`Intent::create`**（計画確定は `Intent` 側へ移った）。`WorkspaceScan` は `Started` → **`Created` が運び `Intent` が保持**。`Started` が運ぶ 3 つを明記 | `（B12 の分割 / b39 2026-09-02、実測 intent.rs:114・created.rs・started.rs）` | `intent.rs:114`（`create` が `WorkspaceScan` を受ける）、`intent.rs:54-62`（`scan` フィールド）、`intent_execution.rs:225`（`start` は `(id, &Intent, at)` のみ）。テスト: `intent.rs` インラインテスト。仕様: 台帳 O2 / O7 |
| C5 | §3.3 workspace 集約（旧 :116） | 集約 `Intent` / `Space` / `Worktree` を **予定（未実装、クリティカルパス 4）** と明記。`intents.json` の直列化機構も未実装と記載。orchestration の `Intent` は**同名だが別物**と注記 | `（2026-09-07 実測 — workspace モジュールに集約は無く、値オブジェクトと FCC のみ）` | `ls modules/core/command/domain/src/workspace/` に集約なし（値オブジェクト + FCC のみ）。`grep -rn 'intents.json' modules/` に登録簿の書込なし。仕様: 台帳 S4、§4.7 |
| C6 | §3.3 脚注（旧 :121） | 「U2 実装の `IntentId::parse` は kebab を受理、B5 で是正」→ **是正済み**（打消し線 + 現行の所在） | `（B5 で是正済み、実測 orchestration/intent_id.rs・workspace/intent_dir_name.rs）` | `modules/core/command/domain/src/orchestration/intent_id.rs`（UUIDv7 検証）、`workspace/intent_dir_name.rs`（記録ディレクトリ名の別型）。テスト: 両ファイルにインラインテストあり。仕様: 台帳 S1、11 号 §2.2 |
| C7 | §4 B1（旧 :184） | 合成の所有者 `WorkflowExecution` → `IntentExecution` | `（集約名は B12 2026-08-30 に追従）` | C2 と同じ。仕様: 台帳 O10 |
| C8 | §6 第一陣（旧 :231） | 同上 | `（集約名は B12 2026-08-30 に追従）` | 同上 |
| C9 | §7.1 原則 5（旧 :278） | 「`WorkflowExecution` が `WorkflowDefinition` を `definition_id` で参照」→ **2 段**（`Intent` → `WorkflowDefinition` は `definition_id`、`IntentExecution` → `Intent` は `intent_id`）。引数渡し + `id` 照合の規律と `aggregate-references.md` への相互参照を追記 | `（B12 2026-08-30 の集約分割に追従 / coding-rules/aggregate-references.md）` | `intent.rs:56`（`definition_id`）、`intent_execution.rs:138`（`intent_id`）、`intent_execution.rs:1864`（`matches` ガード）。テスト: `IntentMismatch` テスト。仕様: 台帳 O1、BR3.6 |
| C10 | §7.1 原則 7〜12（**追記**） | BR3.3 (h) の求める 6 原則を追記（見出しの「6 原則」→「12 原則」）— 7 ドメインオブジェクト 4 種 + FCC、8 ドメインイベント = エンティティ、9 永続化中立、10 スナップショット + 差分再生と壊れた歴史のクラッシュ、11 CQRS のクレート境界と DAO 1 表 1 引当、12 infrastructure = 言語拡張 / 借り物の契約。**BR3.3 (j) の相互参照**を 14 本の coding-rules へ張った | `（オーナー規律 2026-09-02 / 2026-09-03、ADR-002 / ADR-008 / ADR-010、実測 FCC 16 型・Cargo.toml の依存不在）` | FCC: orchestration 8 / workspace 6 / workflow-definition 2 = **16**（`grep -rln 'FirstClassCollection for'`）+ `core-infrastructure` の `Collection<T>` / `NonEmptyCollection<T>` / `canon_json::ObjectMembers`。永続化中立: `modules/core/command/domain/Cargo.toml` の `[dependencies]` に serde / event-store-adapter-rs なし。DAO: `cargo lint dao-single-table`（`tools/lint/src/check.rs`）。テスト: `modules/core/command/domain/tests/collection_contract_test.rs`（FCC 契約ハーネス）。仕様: 台帳 O6 / O9 / O13 / K2 / K3 / K4 / P6、BR3.6 |

## 3. Green / Refactor 後の検査結果（Step 3、実測をそのまま貼る）

```
=== (2) sentinel 10 語 grep（所有ファイル、履歴マーカー除外）===
$ grep -rnE '...10 語...' <所有 coding-rules 9 ファイル> docs/specs/10-orchestration.md docs/specs/01-domain-model.md \
    | grep -vE '~~|旧|失効|是正済み|改名|履歴' | wc -l
0

（参考: coding-rules 全体（CONSISTENCY-AUDIT 除く）でも 0 件）
$ grep -rnE '...10 語...' aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/*.md \
    | grep -v CONSISTENCY-AUDIT | grep -vE '~~|旧|失効|是正済み|改名|履歴' | wc -l
0

=== (3) README の無矛盾 ===
$ ls aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/*.md | grep -vE 'README|good-examples|CONSISTENCY-AUDIT' | wc -l
22
$ grep -cE '^\| \[' aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/README.md
23
$ grep -nE '規則が [0-9]+ 本|^2026-[0-9-]+追加:' aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/README.md
10:規則が 22 本になり、全文を頭に入れて衝突を裁定する前提は成立しない。**読み替えて進まず、

→ 22 = 23 - 1（good-examples 行）= 22。索引ずれ 2 点は解消（告知行は消え、件数は 22）。

=== (4) 表の列数・見出し重複（unit-test-instructions §1 (4) の python を所有 11 ファイルに絞って実行）===
tables ok

=== (7) 実装状態の表記 ===
$ grep -cE '予定（未実装' docs/specs/10-orchestration.md docs/specs/01-domain-model.md
docs/specs/10-orchestration.md:8
docs/specs/01-domain-model.md:1

=== (9) 用語（RMU 呼出の「同期」）===
$ grep -nE '同期' docs/specs/01-domain-model.md docs/specs/10-orchestration.md | grep -icE 'rmu|catch_up|投影'
0

=== コード変更ゼロ ===
$ git diff --stat -- modules tools scripts .github Cargo.toml Cargo.lock formal docs/specs/research docs/specs/deviations.md
（空）

=== リンク健全性（自主検査）===
$ 10 号 / 01 号 の相対リンク（*.md）を全数解決確認 → BROKEN 0 件
```

### 受入 (7) の第 2 コマンドで残った行と、その理由

`grep -nE 'unpark|recompose|doctor|WorktreeService|OpaqueFlagStore|ScopedStorage|SessionStampStore|intents\.json|SwarmBatch'`
から `予定|~~|旧|失効|履歴` を除いて残るのは、所有 2 ファイルで 17 行。いずれも**実装状態を主張しない行**であり、
unit-test-instructions §1 (7) の「逐語契約の引用ほか、実装状態を主張しない行」に当たる。内訳:

| 種別 | 行 | 理由 |
|---|---|---|
| 責務・所有の記述 | 10 号 :15 / :19 / :59、01 号 :88 / :179 / :189 | 「recompose はこのコンテキストのコマンド」等、**どのコンテキストが所有するか**の記述。実装済み・未実装を主張していない（`recompose` は集約のコマンドとしては実装済み、ユースケース配線が未実装 — その旨は §3 の B13 で予定表記済み） |
| 契約コーパス・語彙表 | 10 号 :7、01 号 :68 / :210 / :217 | upstream 仕様名の引用、横断機構の**所有帰属**表、近縁綴りの用語衝突表、境界を跨ぐ用語 16 件の一覧 |
| Quint 不変条件名 | 10 号 :151 / :239 | `engine_loop::unpark_restores_position` 等のモデル内アクション名（形式モデルは実装済み） |
| 見出し | 10 号 :183（`### 7.2 SwarmBatch サーガ`） | 直上の §7 導入で「本節全体が予定（未実装、swarm はスコープ外）」と明記済み |
| 実装済みの実装ノート | 10 号 :176 / :267 | `--single` / `--skeleton-stance` は b47 で実装済み（`record_single_stage_run` / `record_skeleton_stance`） |
| 01 号 §3.2 責務 | 01 号 :84 / :226 | 状態機械の所有者の記述（C2 / C8 で集約名を現行化済み） |

## 4. 保留（勝手に決めなかったもの）

### 保留 1 — 台帳 P4 / gap-measurement §1 と現行コードの食い違い（**ブリーフ T1 が既に訂正しているが、台帳本文は未修正**）

gap-measurement §1 の「コマンド側ユースケース（9）」欄と §4.2 P4 は「テストダブル型 `InMemoryXxxRepository` は無い」と書くが、
実測では `modules/core/command/use-case/src/orchestration/test_support.rs` に `pub(crate)` のフェイクが **4 つ**ある
（`:241` `InMemoryWorkflowDefinitionRepository` / `:388` `InMemoryCompiledDefinitionRepository` / `:622` `InMemoryIntentExecutionRepository` /
`:770` `InMemoryIntentRepository`、`orchestration/mod.rs:38-39` で `#[cfg(test)]`）。
ブリーフの T1 がこれを 3 層構造として訂正しているので、**T1 に従って書いた**（A14 / B14）。台帳本文（§1 の表と §4.2 P4）は
まだ旧記述のままなので、メインで台帳側を訂正するか、訂正済みである旨を記録に残すか**裁定を仰ぐ**。

なお `test_support.rs:1-17` の doc は「ここに置くのは 1 つだけである」と書いており、実際は 4 つある。**コードの doc 側の記述ずれ**だが、
本 Unit はコードを変更しないので触っていない（別 Bolt の 1 行修正候補）。

### 保留 2 — `StateBinding` は**両側に実在する**（台帳 O15 は「クエリ側に住む」と書く）

台帳 O15 は「`Directive` / `DirectiveKind` / `ContinueToken` / `StateBinding` は**クエリ側 `core-query-use-case`** に住む。
コマンド側ドメインは `state_binding()` の材料だけ」とするが、実測では **両側に同名型がある**:

- `modules/core/command/domain/src/orchestration/state_binding.rs:10` — `pub struct StateBinding(String)`。`IntentExecution::state_binding()`（`intent_execution.rs:2330`）の戻り値。
- `modules/core/query/use-case/src/orchestration/state_binding.rs:11` — `pub struct StateBinding(String)`。継続トークンの封筒側。

10 号 §2.2 の表には `StateBinding` の行が無いため**改訂には影響しなかった**（Directive / DirectiveKind / ContinueToken の 3 型は
クエリ側のみで、台帳どおり）。ただし台帳 O15 の文言はこのままだと誤読を招くので、**台帳側の訂正要否をメインで裁定**してほしい。

### 保留 3 — 台帳の行に無いが受入検査が要求した改訂 3 件

いずれも受入 (2)（sentinel 0 件）または作法「実装状態の表記」を満たすために必要だったが、
gap-measurement §2.1 / §2.4 / §2.5 の表には行が無い。**メインの diff レビューで採否を確認**してほしい。

1. **10 号 §1 B1（旧 :31）** — `WorkflowExecution` の名称のみ改訂。§2.1 の表に無いが受入 (2) が 0 件を要求する（B2）。
2. **`README.md` 機械化ロードマップ §優先順 **2 行目**** — 「是正対象は 1 と同じ型」が、1 行目の是正済み化により宙に浮くため、同じ打消し線 + 是正済み注記を添えた。BR1.6 の 12 行にも sentinel にも含まれないが、放置すると BR4.2「README と実ファイルが不一致」に当たる。**最小変更の原則からは逸脱**しているので、差し戻しの判断はメインに委ねる。
3. **10 号 §7 導入 / §2.1 末尾 / §7.1 HOLD-MERGE** — `Bolt` / `SwarmBatch` / `OpaqueFlagStore` への予定表記（B23）。作法「実装状態の表記」が名指しする項目なので在圏と判断した。

### 保留 4 — BR3.3 (j)（未参照 coding-rules 12 本への相互参照）は**部分実施**

BR3.3 (j) は「一度も仕様から参照されていない coding-rules 12 本へ、該当節から相互参照を付ける」とするが、
ブリーフの作業表（A-3 / A-4）にはこの項目の行が無い。**01 号 §7.1 の原則追記（BR が明示的に許した節の新設）の中で、
原則の内容と直接対応する 15 本**（初稿の「14 本」は数え誤り — PR #119 CodeRabbit 指摘で列挙 15 件に合わせて訂正）に相互参照を張った — abstract-data-type / aggregate-references / command-query-separation /
cqrs-boundaries / domain-object-kinds / domain-persistence-neutrality / domain-services / error-handling /
field-visibility / first-class-collections / infrastructure-layer / interior-mutability / module-visibility /
ubiquitous-language / upstream-contracts。

11 号 / 12 号（派遣 B の所有）側での相互参照と、10 号本文への追加参照は**実施していない**（書込スコープ外・作業表に無い）。
BR3.3 (j) の残りをどこで消化するかは**メインで裁定**してほしい。

### 保留 5 — 根拠を添えられなかった主張は 0 件

改訂 1 件ごとに path:line を実測してから書いた。書けなかった主張（コードで確認できなかったもの）は**採用していない**。
唯一「予定（未実装）」と書いた 4 種（`ProgressSignature` / unpark・jump・recompose のユースケース / Bolt・SwarmBatch /
workspace 集約 3 と `intents.json`）は、**不在を grep / ls で実測**して根拠とした。

## 5. 触っていないもの

| 対象 | 状態 |
|---|---|
| gap-measurement §2.1 の『維持』行（10 号 :56 Tx 境界） | **不変**（B10。`persist_event_and_snapshot` と一致） |
| gap-measurement §2.4 の『維持』相当（01 号 §3.1 集約段落・§3.3 Domain Primitive・§3.3 描画関数） | **不変** |
| 既存の履歴行（打消し線つきの旧 manifest 綴り `workflow-execution-event/1`、10 号 :51 内） | **不変**（gap-measurement §5 の訂正どおり改訂対象外） |
| `coding-rules/` のうち所有外 15 ファイル（`abstract-data-type` / `aggregate-commands` / `aggregate-references` / `cqrs-boundaries` / `domain-equality` / `domain-object-kinds` / `domain-persistence-neutrality` / `domain-services` / `first-class-collections` / `good-examples` / `infrastructure-layer` / `no-backward-compatibility` / `tell-dont-ask` / `ubiquitous-language` / `upstream-contracts` / `CONSISTENCY-AUDIT-2026-08-24.md`） | **不変**（`git status` で確認） |
| `docs/specs/11-workspace.md` / `12-workflow-definition.md` / `deviations.md` / `research/**` | **不変**（11 / 12 号は派遣 B が並行編集中。`git status` に出るのは B の変更） |
| `inception/**`（components / contract-summary / unit-of-work / decisions） | **不変**（派遣 B の所有） |
| コード（`modules/` / `tools/` / `scripts/` / `.github/` / `Cargo.*`）と `formal/` | **不変**（`git diff --stat` が空） |
| `## Review` 節 | 所有 11 ファイルはいずれも `## Review` 節を持たない（`grep -c '^## Review'` = 0）ため、NFR1.5 の対象外 |
| `git add` / `git commit` / push / PR / GitHub 書込 / `.claude/` のツール実行 / `AIDLC_*` の設定 | **未実施**（ブリーフの禁止事項どおり） |
