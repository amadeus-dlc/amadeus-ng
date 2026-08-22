# code-quality-assessment — 品質評価と技術的負債

> リバースエンジニアリング成果物（2026-08-22 実施、`c4d8d95` 時点）。一次情報は開発者スキャンの Technical Debt Signals（実地確認済み）と設計監査 `aidlc/spaces/default/knowledge/aidlc-shared/design-audit-2026-08-22.md`（監査 5 エージェント → 33 主張 → 検証 12 エージェント全数照合、29 CONFIRMED / 4 棄却）。評価軸はコーディング規則の正本 `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/`。

## 品質保証の全体像（実測: 全緑）

スキャン時の実行検証はすべて green:

- `cargo fmt --all --check` PASS / `cargo lint` PASS（所見 0）/ `cargo test --workspace` PASS（**234 テスト全緑**)
- カバレッジ実測 **94.87〜95.29%**（2026-08-22。絶対 90% 床 + PR 相対ゲート 0.5pp を上回る）

品質保証は三層構造で、各層が別種の欠陥を捕らえる:

1. **Quint 形式検証**（不変条件 run 27 本 + 到達性 witness 12 本の反転判定 + 決定的シナリオ、毎 PR）。モデル自体の検査力は mutation テストで証明済み（engine_loop 3/3、audit_lock 10/10 + witness 7/7、stop_hook 7/7）。
2. **ITF 準拠テスト**（Quint トレース 15 本を集約に再生し状態射影を突き合わせ — モデルと実装の乖離を検出）。
3. **PBT（proptest）+ ゴールデンパリティ**（upstream 配布実バイト 33 ノードの全数 load パリティ — upstream 互換の逸脱を検出）。

## リント・CI

- **3 段構え**: ① rustfmt ② clippy（workspace lints 約 50 ルール deny — `unwrap_used` / `expect_used` / `missing_docs` / `unreachable_pub` / `todo` / `print_stdout` / `needless_pass_by_value` 等）③ カスタム `cargo lint`（ルール 3 本、全ルール赤例テスト必須 — 赤例 31 本）。
- **CI**（PR→main + workflow_dispatch）: check（fmt / clippy -D warnings / cargo lint / test）、quint（Node 22 + Quint 0.32.0）、coverage（cargo-llvm-cov）の 3 ジョブ。
- **既知の穴**: ① CI が detached クレート `tools/lint` に届かない（C27 実地確認済み — fmt / clippy / 自己テストの 3 ステップ未追加）。②理由なし `// amadeus-lint: allow(rule)` の抑制が機械的に成立してしまう（C28、`check.rs:93-100` 確認済み）。
- デプロイパイプラインは無し（CLI ツールであり `cargo install` 配布を A1 で計画 — 欠落ではなく計画済み）。

## ドキュメント品質

極めて高水準。`missing_docs = deny` で公開面の doc が全数強制され、upstream `file:line` @ `3c3146cf` へのピン留め逐語引用が doc に常設、仕様・ADR・coding-rules へ相互参照される。仕様は日本語正本（D7、specs 6 本 1,059 行 + ADR 5 本 + research 15 本 + 凍結 upstream 28 本）。

例外は**ルート `README.md` が 12 バイトのスタブ**であること（見出し 1 行のみ。実地確認）。また微細な不整合として、`00-policy.md` §5.1 の逸脱台帳の状態記述が「#1・#2」で止まっており、実台帳の 3 件目（`AIDLC_LOG` 拡張）を反映していない（実地確認 — 台帳側が正）。

## コーディング規則への準拠（正本 7 ファイルを評価軸に）

| 規則 | 準拠状況 | 未解決の所見 |
| --- | --- | --- |
| tell-dont-ask | 準拠。`reap_eligible` のドメイン一元化・`CheckboxState` 分類述語が適用例。lint 2 本で機械強制 | なし |
| domain-equality | 準拠。`OwnerStamp` の 3 フィールド Eq 手実装が適用例 | なし（レビュー基準のまま、未リント化） |
| field-visibility | 準拠。`no-public-fields` で機械強制 | C26: `LockGuard::new` 公開による偽造可能性（型強制の穴） |
| module-visibility | 準拠。私有 mod + ファサード `pub use` が全クレートで一貫 | `cargo lint` ルール化は予定のみ |
| gateway-taxonomy | 概ね準拠（2026-08-22 再設計で Store/Reader 造語・媒体名を一掃済み） | C21/C22: 逐語文言 7 形の Gateway 直書き残存（R4）。C19/C20: doc の load 語彙残滓 5 箇所 |
| use-case-rules | 構造上準拠（クレート分離が DIP を強制）。ユースケース本体が未実装のため本格適用は B-1 以降 | R5: `LoadStageGraph` / `LoadScopeCatalog` の衛生リネーム待ち |
| 規則正本自体 | — | C1/C2: canon 内の自己矛盾 2 箇所（例示の `load()` → `find()` 未修正） |

## 技術的負債（設計監査の A〜E 束を保持）

負債は場当たりでなく、**29 件の CONFIRMED 指摘が確定裁定（R1〜R5）付きで束に分類され、Bolt 化して消化する計画**になっている。束分類は監査文書の区分を保持する。

### A 束 — canon 自己矛盾（docs、数行）

- C1 `coding-rules/use-case-rules.md:38` — 例示 `repository.load()` → `repository.find()`
- C2 `coding-rules/gateway-taxonomy.md:60` — §4 散文「load / save」→「find / save」

### B 束 — 仕様の canon 追従（docs。R1/R2/R3 の履行）

- C3/C4 `11-workspace.md` — ポート表・供給面表を gateway-taxonomy 語彙へ書き直し（`AuditLedgerRepository` 統一）
- C6 `01-domain-model.md:99` — 集約候補表から WorkspaceLock（並行性サービス）・StateFile（媒体）を整理
- C8 `12-workflow-definition.md:68` — `next_in_scope_stage` 行を R2 と整合
- C9/C10 `12-workflow-definition.md` — 個別ビュー名の廃止・集約昇格理由の是正
- C11/C12 — 1 trait 1 Impl の明記、`PlanAction`・`CheckboxState` の所有一意化（R1）

### C 束 — コード修正

- C13 `PlanAction` 移動（R1 — `scope_grid.rs:16` / `workflow_definition.rs:27` のコンテキスト間逆依存解消）
- C14 `effective_plan_action` 移設（R2 — `workflow_definition.rs:133-143`）
- C17 `# Panics` doc 欠落（`workflow_execution.rs:156,163,195` / `lock_protocol.rs:124`。恒久解は StageIndex E1 型 — B-2）
- C18 `gated`（`workflow_execution.rs:189`）の範囲無検査を読取系と統一
- C19/C20 load 語彙残滓 5 箇所（doc・テスト名）
- C21/C22 逐語文言の材料化と `message_catalog` 移設（R4 — `workflow_definition_repository_impl.rs:58-167` 等）
- C23 ADR 0005 依存表と Cargo.toml の整合（R4 完了後）
- C24 `io::Error` の ErrorKind 喪失 3 箇所（エラー変換で復旧情報が落ちる）
- C25 `clock.rs:32-37` 縮退値（0 / u64::MAX）の reap への影響 doc 化
- C26 `LockGuard::new` の捏造可能性 — 封印トークンか関連型で型強制
- C31 `state_writers.rs` の `value: &str` 生値配線 → `StateFieldValue`
- C32 `bolt_refs.rs:66` `append_slug` の無検査 → `BoltSlug::parse` か拒否検査
- C33 `ScopeName` Domain Primitive 新設（scope 名の生 String 流通停止）

### D 束 — リンター / CI

- C27 ci.yml に `tools/lint` 向け fmt / clippy / test の 3 ステップ追加
- C28 理由なし抑制の成立を封じる + 赤例テスト
- C29 未スタンプ猶予 `grace_ms`（`fs_workspace_lock.rs:235`）をドメインへ昇格
- C30 Finding への scope / rationale 追加（新ルールの DoD に組込）

### E 束 — B-1 / B-2 設計への繰延（ステージ内で裁定）

- C16 ドメインイベント不発行 — 集約コマンドの戻り値にイベント列を載せ audit-first で台帳へ（B-1 中核）
- `AuditLedger` を peer 集約でなくイベントログ（真実源は監査）へ再分類 — B-1 冒頭裁定
- StateFile 所有 3 説の一本化（`WorkflowExecution` が集約ルート、StateFile は媒体）— B-2
- StageIndex E1 型（C17/C18 恒久解）・`WorkflowExecution` 保持データのスリム化

### 束外の観察（スキャン実地確認）

- スタブ 3 クレート（canon-json / aidlc / harness-claude）と `state_file_io.rs` の dead_code は**計画済み未着手**であり負債ではない（追跡されている空白）。
- TODO 6 件は全件トラッキングタグ付き、野良 TODO / FIXME / HACK ゼロ。
- audit-events の CLI_RESERVED(8)・MERGE_PROTECTED、directive-schema の Directive 本体は**意図的未定義**（upstream 読解待ち）。
- 微細: Cargo.lock の syn 2 系 / 3 系併存（推定 zerocopy-derive 由来）、`.DS_Store` 散在（gitignore 未収載）、ルート README スタブ、`rust-toolchain.toml` 不在（ツールチェーン固定なし — 実地確認）。

## 棄却済み指摘（再提起しない）

監査で検証の結果 REJECTED となったもの。将来のレビューで再提起しないための記録:

- C5 `find_all_events` は純関数（I/O は供給面）— `11-workspace.md` §2.3 の分類は正しい
- C7 `Load*` はユースケース名であり Repository 語彙規則の適用範囲外（R5 は衛生リネームであって違反是正ではない）
- C15 `plan.len()` は `stage_count` 経由で境界検査に実使用（スリム化論点のみ E 束へ）
- C19 の一部（`workflow_definition_repository_impl.rs:888` と InMemory `:19` の load 残滓は誤認）

## 総合評価

新規行の品質規律（fmt / clippy 50 ルール deny / カスタム lint / 90% カバレッジ床 / 形式検証 / ゴールデンパリティ）は、この規模のコードベースとして例外的に厳格で、実測もすべて緑。負債は「未知の腐敗」ではなく「裁定済み・番号付き・Bolt 化予定の既知残作業」に整理されており、リスクの主体は量ではなく**未履行の裁定（R1〜R4）と未着手の空白（ユースケース本体・CLI・監査 I/O）**にある。stage-1 切替条件（`00-policy.md` §4 の 5 条件）に対しては、条件 5（CI green）の基盤は既にあり、条件 1〜4 を満たすための実装がフェーズ A の残作業として明確に残っている。
