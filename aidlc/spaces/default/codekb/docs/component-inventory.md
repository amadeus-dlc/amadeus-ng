# component-inventory — 全コンポーネント一覧

> リバースエンジニアリング成果物（2026-08-22 実施、`c4d8d95` 時点）。Cargo クレート 11（workspace メンバー 10 + detached 1）+ 非 Cargo コンポーネント。依存はすべて内向き（逆依存はビルドエラー）。

## 一覧表

| コンポーネント | 種別 | 状態 | 一言責務 |
| --- | --- | --- | --- |
| core-domain | library | 実装済み | 3 コンテキストのドメイン層（集約・Domain Primitive・純関数） |
| core-use-case | library | ポートのみ | ユースケース層のポート trait 2 本（本体未着手） |
| core-interface-adapter | library | 実装済み | Gateway 実装 + 機構（Clock / ProcessProbe） |
| audit-events | library | 実装済み | 監査イベント語彙の閉集合（Published Language） |
| directive-schema | library | 部分実装 | ディレクティブ種別の閉集合 |
| message-catalog | library | 実装済み | upstream 逐語文言カタログ（7 形 Captured） |
| canon-json | library | スタブ | 正準 JSON シリアライザ予定地（ADR 0001） |
| infra-io | library | 実装済み | 低水準 I/O プリミティブ（ポリシーなし） |
| aidlc | binary | スタブ | composition root マルチコールバイナリ |
| harness-claude | library | スタブ | Claude Code ハーネス配線予定地 |
| amadeus-lint | binary（detached） | 実装済み | コーディング規則の機械強制（`cargo lint`） |
| formal | Quint モデル | 実装済み | 決定論コアの状態機械契約（3 本、mutation 検証済み) |
| scripts | bash | 実装済み | カバレッジゲート / Quint ゲートの CI 実行体 |

## Cargo クレート

### core-domain

`modules/core/domain`。ドメイン層。I/O ゼロ・serde 非依存。境界づけられたコンテキスト 3 つ:

- `orchestration` — 集約 `WorkflowExecution`（engine_loop.qnt の純粋ステップ関数）、`AutonomyMode` / `JumpDirection` / `PlanAction` / `SkeletonStance` / `Verdict`
- `workflow_definition` — 読取モデル集約 `WorkflowDefinition`、`StageGraph` / `ScopeGrid` / `StageNode` ほか Domain Primitive 10 種
- `workspace` — `LockProtocol`（audit_lock.qnt の純粋ステップ関数）、`reap_eligible` 述語、状態ファイル純関数群、Always Valid newtype 群

依存: `audit-events`, `directive-schema`, `message-catalog`。

### core-use-case

`modules/core/use-case`。ポート trait のみ: `WorkflowDefinitionRepository`（読取専用 `find`）、`WorkspaceLock`（`acquire` / `release`、非 Clone `LockGuard`）。ユースケース本体は未着手。依存: `core-domain`, `audit-events`, `directive-schema`。アダプタ層への依存が無いこと自体が DIP の機械強制（E0432）。

### core-interface-adapter

`modules/core/interface-adapter`。Gateway 実装層: `WorkflowDefinitionRepositoryImpl`（PL 3 入力読取・serde ワイヤ構造体・逐語文言）、`InMemoryWorkflowDefinitionRepository`（テストダブル）、`FsWorkspaceLock`（mkdir-EEXIST + reap CAS）、`state_file_io`（B-2 向け内部部品、dead_code 許可中）、機構 `Clock` / `ProcessProbe`（Fake 付き）。依存: `core-use-case`, `core-domain`, `audit-events`, `directive-schema`, `canon-json`, `message-catalog`, `infra-io` + 外部 serde / serde_json / md5。

### audit-events

`modules/shared/audit-events`。監査イベントスキーマ（Published Language）: `EventType` 86 語 / 22 カテゴリの閉集合、MANDATORY 8、CLI_PROTECTED 18。CLI_RESERVED(8)・MERGE_PROTECTED は意図的未定義。依存ゼロ。

### directive-schema

`modules/shared/directive-schema`。`DirectiveKind` 10 種の閉集合（placeholder 2 種にマーク）。Directive 本体・28KiB 上限・continue_token は後続スライス。依存ゼロ。

### message-catalog

`modules/shared/message-catalog`。upstream 逐語文言カタログ（ADR 0002）。7 形、全数 `Captured`（バイト一致確認済み、JS `toFixed(1)` 丸め再現含む）。依存ゼロ。

### canon-json

`modules/shared/canon-json`。スタブ（3 行）。正準 JSON シリアライザ（ADR 0001 / A2）の予定地 — ハッシュ・ドリフトガード・冪等判定の土台になる計画。依存ゼロ。

### infra-io

`modules/infra-io`。低水準 I/O プリミティブ: `atomic`（tmp + rename + fsync）、`append_only`（O_APPEND \| O_NOFOLLOW 追記 open）、`fs_meta`（lstat / dev-ino / W_OK）、`process_probe`（kill(pid,0) ESRCH 判定）。ポリシーなし。`#![forbid(unsafe_code)]` を safe wrapper（nix / libc）で維持。依存: 外部のみ（libc, nix）。

### aidlc

`modules/app/aidlc`。composition root マルチコールバイナリ。現状 `const fn main()` のスタブ（サブコマンド 0）。実装はフェーズ A。依存: `core-interface-adapter`, `infra-io`。

### harness-claude

`modules/harness/claude`。スタブ（3 行）。Claude Code ハーネス配線の予定地。依存: `core-interface-adapter`。

### amadeus-lint

`tools/lint`。**workspace 非メンバーの detached クレート**（意図的 — coverage / test 対象から外すため。副作用として CI が届かない = C27）。`cargo lint` エイリアスで起動。syn ベースでルール 3 本（`checkbox-vocabulary` / `reap-decision-locality` / `no-public-fields`）、赤例テスト 31 本。抑制は理由コメント付き `// amadeus-lint: allow(<rule>)`。依存: syn, proc-macro2（独自 `Cargo.lock` で独立解決）。

## 非 Cargo コンポーネント

### formal

Quint モデル 3 本、計 1,102 行: `orchestration/engine_loop.qnt`・`orchestration/stop_hook.qnt`・`workspace/audit_lock.qnt`。mutation テストで検査力証明済み（engine_loop 3/3、audit_lock 10/10 + witness 7/7、stop_hook 7/7）。CI の quint ジョブが typecheck / 不変条件 run 27 本 / 到達性 witness 12 本の反転判定を毎 PR 実行。

### scripts

bash 2 本: `coverage.sh`（cargo-llvm-cov、絶対 90% 床 + PR 相対ゲート許容 0.5pp）、`quint-gate.sh`（Quint ゲートの実行体）。

### テスト資産（tests/）

`tests/golden/upstream-3c3146cf/`（ゴールデン入力。stage-graph.json 81,850 bytes、バイト変更禁止）、`tests/conformance/fixtures/`（ITF トレース 15 本）。コードではなく契約データだが、互換性検証の要のため一覧に含める。

### 開発プロセスホスト（.claude/ + aidlc/）

stage-0 の AI-DLC フレームワーク資産（upstream 2.6.54 の既製ワークスペースシェル）。プロダクトコードではない。ただし `aidlc/spaces/default/knowledge/aidlc-shared/` の coding-rules と design-audit は実装と双方向参照される裁定 record であり、事実上の設計正本の一部。
