# 設計監査 2026-08-22 — 検証済み指摘と確定裁定（intent 260822-stage1-selfhost のインプット）

オーナー全29スキルによる設計監査（監査5エージェント → 33主張 → 検証12エージェントが
`main@c4d8d95` に全数照合。29 CONFIRMED / 4 棄却）。本ファイルは INCEPTION の
reverse-engineering / domain-design / delivery-planning の入力であり、A〜D 束は
Bolt 化して **Bolt 単位 PR** で消化する。詳細版はオーナーの Artifact に保存済み。

## 確定裁定（オーナー承認 2026-08-22）

- **R1 DECIDED**: `PlanAction` の所有は workflow_definition コンテキストへ一本化する
  （orchestration は再輸出せず、呼出側パスを同一 Bolt で一斉修正する完全移動 — 2026-08-22 の
  再エクスポート禁止裁定（coding-rules/module-visibility.md 追補）により「re-export で参照」から改訂。
  ADR-005 参照）。01/12号の宣言どおり。コード移動 = C13、仕様表記 = C12。
- **R2 DECIDED**: 有効プランの畳み込み（オーバレイ ∨ グリッド）は orchestration 側の
  ドメインサービス（`(&WorkflowExecution, &WorkflowDefinition) → …`）へ移設。
  `WorkflowDefinition` にはグリッド照会 `plan_action_in_grid` のみ残す（C14/C8、B1 の明文どおり）。
- **R3 DECIDED**: 11号 §3 ポート表・供給面表を gateway-taxonomy へ準拠書き直し。
  機構（Clock/ProcessProbe/Tmpdir）はアダプタ層機構、`FileStore` は Repository 実装内部、
  Git は外部システムクライアント。`AuditLedgerService` → `AuditLedgerRepository` に統一（C3/C4/C6/C11）。
- **R4 DECIDED**: Gateway 直書きの逐語文言7形は `message-catalog` へ移設（ADR 0002 決定6 の履行。
  C21/C22/C23。ドメイン層エラーは材料のみ保持し文言化はアダプタ層）。
- **R5 DECIDED**: ユースケース名 `LoadStageGraph` / `LoadScopeCatalog` は 12号改訂時に
  ドメイン意図名へリネーム（規則違反ではないが語彙衛生）。

## 修正束（Bolt 候補）

### A. canon 自己矛盾（docs、数行）
- C1 `coding-rules/use-case-rules.md:38` — 例示 `repository.load()` → `repository.find()`
- C2 `coding-rules/gateway-taxonomy.md:60` — §4 散文「load / save」→「find / save」

### B. 仕様の canon 追従（docs。R1/R2/R3 の履行）
- C3 `docs/specs/11-workspace.md:85` — ポート表（FileStore/GitPort/ProcessProbe/Tmpdir/Clock）書き直し
- C4 `docs/specs/11-workspace.md:77-83` — 供給面表の造語・媒体名を taxonomy 語彙へ、AuditLedgerRepository 統一
- C6 `docs/specs/01-domain-model.md:99` — 集約候補表から WorkspaceLock（並行性サービス）・StateFile（媒体）を整理
- C11 `docs/specs/10-orchestration.md:78` — 実装欄「同上」を廃し 1 trait 1 Impl を明記
- C12 `10:47 / 01:77,101` — PlanAction・CheckboxState の所有一意化（R1）
- C8 `docs/specs/12-workflow-definition.md:68` — next_in_scope_stage 行を R2 と整合させる
- C9 `12:170-174` — StageGraphQuery/StageNodeView/SensorBindingView の個別名を廃止（集約の述語面と記述）
- C10 `12:39` — 集約昇格の第一理由を「3入力は compile が lockstep で出す（一貫性単位）」へ

### C. コード修正
- C13 PlanAction 移動（R1）: `scope_grid.rs:16` / `workflow_definition.rs:27` の逆依存解消
- C14 effective_plan_action 移設（R2）: `workflow_definition.rs:133-143`
- C17 `# Panics` 欠落: `workflow_execution.rs:156,163,195` / `lock_protocol.rs:124`（恒久解は StageIndex E1 — B-2）
- C18 `gated`（`workflow_execution.rs:189`）の範囲無検査を読取系と統一
- C19/C20 load 語彙残滓5箇所（InMemory doc :4・テスト名 :70、use-case port doc :2,:22,:74）
- C21 ドメイン層エラーの文言保持を材料保持へ: `autonomy_mode.rs:71` / `state_writers.rs:53`（R4）
- C22 逐語文言7形を message_catalog へ移設: `workflow_definition_repository_impl.rs:58-167`（R4）
- C23 ADR 0005 依存表と Cargo.toml の整合（R4 完了後に再確認）
- C24 io::Error の ErrorKind 喪失: `workspace_lock.rs:80-85` / `fs_workspace_lock.rs:343` / `state_file_io.rs:90-105`
- C25 `clock.rs:32-37` 縮退値（0 / u64::MAX）の reap への影響を doc 化
- C26 `LockGuard::new`（`workspace_lock.rs:56`）の捏造可能性 — 封印トークンか関連型で型強制
- C31 `state_writers.rs:20,51,79` の `value: &str` → `StateFieldValue` 配線
- C32 `bolt_refs.rs:66` append_slug の無検査 → BoltSlug::parse か拒否検査
- C33 `ScopeName` Domain Primitive 新設（SpaceName 同型）— scope 名の生 String 流通を止める

### D. リンター / CI
- C27 ci.yml に tools/lint 向け fmt/clippy/test 3ステップ追加（detached workspace のため現状どれも届かない）
- C28 理由なし `// amadeus-lint: allow(rule)` の抑制成立を封じる＋赤例テスト
- C29 未スタンプ猶予 `grace_ms`（`fs_workspace_lock.rs:235`）の R2 検出漏れ — ドメインへ上げて識別子追加
- C30 Finding に scope/rationale 追加・help 文言の assert（新ルール R4〜R7 の DoD に組込）

## E. B-1 / B-2 設計への繰延（ステージ内で裁定）
- C16 ドメインイベント不発行 — 集約コマンドの戻り値にイベント列を載せ audit-first で台帳へ（B-1 中核）
- AuditLedger を peer 集約でなくイベントログ（B9: 真実源は監査）へ再分類 — B-1 冒頭裁定
- StateFile 所有 3説の一本化（WorkflowExecution が集約ルート、StateFile は媒体）— B-2
- StageIndex E1 型（C17/C18 恒久解）・WorkflowExecution 保持データのスリム化（PlanAction 要素は構築後未読）

## 棄却済み（再提起しない）
- C5 `find_all_events` は純関数（I/O は供給面）— 11号 §2.3 の分類は正しい
- C7 `Load*` はユースケース名であり Repository 語彙規則の適用範囲外（R5 は衛生リネーム）
- C15 `plan.len()` は stage_count 経由で境界検査に実使用（スリム化論点のみ E へ）
- C19 の一部（impl :888 と InMemory :19 の load 残滓は誤認）
