# B11 委任ブリーフ 1 — U5 report ユースケース（FR2.1 のみ）

Conversation language: 日本語
委任先モデル: Opus（層をまたぐ設計判断を含む — 集約の内部導出化 + ユースケース層の定型確立）
最終責任: Fable 5 メインセッション（全 diff レビュー・検証の独立再実行・受入判定）

## 目的

ユースケース層の**最初の 1 本**を書き、以降の U6・U7 が乗る定型を確立する。
定型 = 「`find_by_id` で集約を**再構成** → 集約コマンドで判断 → `store` で保存 → 型付きの結果を返す」。

`report` の遷移コミット 6 経路（awaiting-approval / approved・completed / rejected / revised /
skipped / resumed）を実装する。

## 固定裁定（オーナー裁定 2026-08-29。**変更禁止・相談なしの逸脱禁止**）

1. **U5 は RMU を呼ばない。** 読み取り用ファイル（`aidlc-state.md` / 監査シャード）の最新化を
   起動するのは **合成ルート（U7 = CLI の配線箇所）** である。`core-command-use-case/Cargo.toml`
   に `core-read-model-updater` を**足さない**（`coding-rules/cqrs-boundaries.md`。クレート分離で
   機械強制されており、足した瞬間に規則違反）。
2. **フェーズ境界は集約が自分で導出する。** `WorkflowExecution::approve_gate` の
   `phase_boundary: Option<PhaseBoundary>` 引数を**廃止**し、集約が `stages()`（各 `StageEntry` が
   `phase` を持つ）と次の実効 EXECUTE ステージから内部導出する。オーナー統一ルール
   「集約は FSM。判断は集約に閉じ込め、ユースケースはフロー制御のみ」に合わせる措置である。
   **これは本家仕様からの逸脱ではなく内部の実装方法の選択**なので、**外形は 1 バイトも
   変えない**: `GateApproved` の payload 形（`phase_boundary` を持つ）、監査シャードのバイト列、
   `aidlc-state.md` の差分はすべて現状どおり。
3. **`StoreVersion` newtype 化は却下。** 楽観 version は本家 `event-store-adapter-rs` v3.0.0 が
   `expected_version: usize` で定めた語彙であり、こちら側で衣を着せるのは Conformist 方針
   （`=3.0.0` ピン・腐敗防止層なし、`coding-rules/upstream-contracts.md`）に反する。`usize` の
   ままポートを往復させる。ポート doc の「newtype 化は U5/U6 の候補」という記述は**却下済みとして
   削除**し、却下理由（Conformist 維持）を 1 行残すこと。
4. **用語**: 新しく書く日本語散文で「再水和」を使わない。「**再構成**」と書く。既存 48 ファイルと
   Rust 型名 `RehydratedWorkflowExecution` の一括置換は**本 Bolt の対象外**（別 PR）。
5. **FR2.2 は本 Bolt の対象外**（レシート述語・verification 面は次の Bolt）。

## 必読

- `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/` の**全 18 規則**。特に:
  - `use-case-rules.md`（DIP・静的束縛・ユースケース間呼出禁止・I8 型の参照渡し）— **本 Bolt の正本**
  - `cqrs-boundaries.md`（依存境界。裁定 1 の根拠）
  - `upstream-contracts.md`（Conformist。裁定 3 の根拠）
  - `command-query-separation.md` / `tell-dont-ask.md` / `field-visibility.md` / `module-visibility.md`
  - `error-handling.md`（thiserror/anyhow 不使用。手実装 enum + `fmt::Display`）
- `modules/core/command/use-case/src/orchestration/workflow_execution_repository.rs`（ポート全文。
  楽観 version の往復規約 3 か条と、既存の `FakeRepository` テストダブルの型）
- `modules/core/command/domain/src/orchestration/workflow_execution.rs`（集約。特に
  `approve_gate` / `reject_gate` / `revise_stage` / `skip_stage` / `complete_stage` / `open_gate` /
  `gated` / `checkbox` / `stale_report` / `accepts_commands`）
- `aidlc/spaces/default/intents/260822-stage1-selfhost/inception/contract-design/contract-summary.md`
  §C5（`report --result X` → ドメインイベントの対応表。**契約の正本**）
- `tests/golden/upstream-3c3146cf/cli/report/` の 7 ケース全部（`argv` / `stdout.json` /
  `state.diff` / `audit.md`）— **仕様の出典として読む**。CLI 実行の検収は U7 で接続するので、
  本 Bolt で実行ゲートには繋げない
- `.claude/tools/aidlc-orchestrate.ts` の `handleReport` / `parseReportFlags` /
  `FORWARD_RESULTS` / `GATE_RESULTS` / `RESUME_RESULTS` / `SKIP_RESULT`（upstream 実挙動）

## 実装スコープ

### (A) domain — フェーズ境界の内部導出（裁定 2）

- `approve_gate` から `phase_boundary` 引数を廃止し、集約内部で導出する。
- 導出規則: 承認するステージの `phase` と、次の実効 EXECUTE ステージ（既存
  `next_in_scope_slug` と同じ走査）の `phase` が異なるなら `PhaseBoundary::new(from, to)`、
  同じ、または次が無い（= 最終）なら `None`。**upstream の実挙動と突き合わせて確認すること**
  （`tests/golden/upstream-3c3146cf/cli/report/approved-across-phases` は classic スコープの
  `delivery-planning` 承認で inception → construction。`approved` は同一フェーズ内で境界なし）。
- 既存の domain テストを追随させる。`a_gate_approval_carries_the_caller_supplied_phase_boundary`
  は「集約が自分で導出する」テストへ**置き換える**（名前も実態に合わせる）。
- 境界を**またぐ場合とまたがない場合と最終ステージ**の 3 つを最低限テストで固定する。

### (B) use-case — `ReportUseCase`（本体）

- 型: `pub struct ReportUseCase<R: WorkflowExecutionRepository> { repository: R }`
  （`use-case-rules.md` §2 の静的束縛。`dyn` 禁止）。
- 入力は**正規化済みの型**で受ける。CLI のフラグ解析・綴りの揺れ（`approved` / `completed` /
  `complete` / `done` が同義であること）の受理は **U7 の仕事**なので、U5 のシグネチャに
  生の文字列を持ち込まない。
- 出力は**型**で返す。「Committed approve for "..." (scope: ...)」のような**文言を U5 に持たせない**
  （B9 で文言は出す側の持ち物と裁定済み。描画は U7 の Presenter）。
- 経路:
  | 入力 | 集約コマンド | 備考 |
  |---|---|---|
  | awaiting-approval | `open_gate` | 既に awaiting なら**コミットせず**その旨を型で返す（golden `awaiting-approval-repeat`） |
  | forward（approved / completed 系） | `gated(stage)` が真なら `approve_gate`、偽なら `complete_stage` | どちらを打つかは**集約のクエリを見て決める**フロー制御であり、業務判断の複製ではない |
  | rejected | `reject_gate` | feedback を渡す |
  | revised | `revise_stage` | |
  | skipped | `skip_stage` | reason を渡す |
  | resumed | **コミットしない** | ルーティングのみ。集約に触れず型で返す |
- 冪等・no-op 経路: カーソル通過済み completed への再報告は集約の `stale_report`
  （BR1.9）を使い、**コミットしない**。
- 保存は `store(&event, &aggregate, expected_version)`。`expected_version` は
  `find_by_id` が返した `RehydratedWorkflowExecution::version` の値**そのもの**を渡す
  （`aggregate.seq_nr()` から導かない — ポート doc の 3 か条）。
- 集約が `Err` を返したら**そのまま伝播**する。ユースケースで握り潰さない・言い換えない。
- **`Conflict` は 1 回だけ再試行する**（**2026-08-29 訂正**: 初版ブリーフは「`Conflict` も再試行
  しない」と書いていたが、これは C3 ③ の誤読であり撤回した。正本は `contract-design-questions.md`
  Q6 = A（オーナー承認済み）「楽観 version 競合は即 `Err`（**ユースケースが 1 回だけ再水和して
  再試行**、それでも競合なら CLI がエラー終了）」。C3 ③「`Conflict` **以外**のエラーはリトライ
  しない」は「Conflict だけが再試行の対象で、その政策はユースケースが持つ」の意である）。
  再試行は**再構成（`find_by_id`）からやり直す** — 古い集約に `store` だけ打ち直すのは楽観
  ロックの意味を壊す。2 回目も `Conflict` なら伝播する。再試行後に集約コマンドが `Err` を返す
  場合（別クローンが先に承認しゲートが閉じた等）もそのまま伝播する。
  `repository_error.rs` の `Conflict` doc は**正しいので変更しない**。

### (C) テスト（TDD）

- `project.md` Mandated のとおり **red-green-refactor**（失敗するテストを先に書く）。
- テストダブルは **use-case クレート内の `#[cfg(test)]` に置く**。
  `core-command-interface-adapter` を dev-dependency に足すのは**禁止**（`use-case-rules.md` §1 の
  機械強制「`core-use-case` の Cargo.toml に `core-interface-adapter` が無いこと」が壊れ、
  依存も循環する）。既存の `FakeRepository` を育てるか、共有可能な
  `InMemoryWorkflowExecutionRepository` として整えるかは任せる。
- 各経路の正常系に加え、異常系として最低: **`find_by_id` が `NotFound`**、
  **1 回目 `Conflict` → 2 回目成功（再試行が効く）**、**2 回とも `Conflict` → 伝播する**
  の 3 本（construction.md「happy path + 2 つ以上の異常系」）。`Conflict` を意図的に起こすため、
  fake は**応答をスクリプトできる**必要がある。
- テストピラミッド: ユニット層を厚く。
- **結線テスト 1 本**（2026-08-29 追加）: 契約 C3 ④（ADR-010 改訂「テストダブル型は無く、テストは
  `XxxUseCase<WorkflowExecutionRepositoryImpl<…>>` で組む」）を満たすため、
  `modules/core/command/interface-adapter/tests/` に、実物の
  `WorkflowExecutionRepositoryImpl::in_memory()` と `ReportUseCase` が組めることを示すテストを
  1 本だけ足す（使用例: `tests/workflow_execution_repository_contract.rs:41`）。網羅は不要 —
  網羅は use-case 側の fake テストが持つ。

## 明示的な非スコープ（触らない）

- RMU の起動、`core-read-model-updater` への変更
- CLI のフラグ解析・ROUTES 表・Presenter の文言（すべて U7）
- `report --single` / `report --skeleton-stance` の経路（U5 責務外）
- FR2.2（レシート述語・verification 面）
- `StoreVersion` newtype 化（却下）
- 「再水和」の一括置換（別 PR）
- 投影キャッチアップのクラッシュ窓（U4 既知事項）
- ゴールデンのバイト（1 バイトも変更禁止。読むだけ）
- upstream ピン（`3c3146cf` / `=3.0.0`）を動かすこと。**本家リポジトリへの接触も禁止**

## 所有ファイル・規律

- 書いてよい: `modules/core/command/use-case/**`、`modules/core/command/domain/**`（裁定 2 の
  範囲のみ）、報告書 `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/
  u5-report-use-case/developer-report-1.md`。
- **例外承認（2026-08-29）**: 裁定 2 で `approve_gate` の引数が 1 つ減る結果、所有ツリー外の
  呼出側 5 箇所がコンパイル不能になる（`no-backward-compatibility.md` により互換オーバーロードは
  残さない）。**`None,` 引数の削除に限り**次の 5 箇所の編集を承認する。他の行は 1 行も触らない:
  `read-model-updater/tests/support/mod.rs:194` /
  `command/interface-adapter/tests/support/contract.rs:40` /
  `app/aidlc/tests/crash_reconstruction_test.rs:79, 206` /
  `app/aidlc/tests/journal_protocol_conformance.rs:265`。
  所有ツリー内の `command/domain/tests/upstream_event_store_conformance.rs:271` と
  `command/domain/tests/engine_loop_conformance.rs:306` も同様に追随する（呼出は全 7 箇所）。
  加えて上記「結線テスト 1 本」のための
  `modules/core/command/interface-adapter/tests/` への**ファイル 1 本追加**を承認する。
- 禁止: `docs/**`・`formal/**`・`coding-rules/**`・`aidlc/**`（上記報告書を除く）・
  `tests/golden/**`・`.claude/**`・`modules/core/read-model-updater/**`・
  `modules/app/**`・`modules/harness/**`。
- `git add -A` 禁止（明示パスのみ）。**push 禁止**。検証は `CARGO_TARGET_DIR=$PWD/target-delegate`。
- コミットは意味単位・日本語・`b11: ` 接頭辞。**私は完了報告までコミットしない**（委任中は
  委任者側のコミットを凍結している）。
- 迷ったら**止めて報告する**。裁定に触れる判断を勝手にしない。

## 受入基準（すべて緑）

1. `cargo fmt --all --check`（`tools/lint` も）
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo lint`
4. `cargo test --workspace`（既存 234 テストの退行ゼロ）
5. `scripts/quint-gate.sh`
6. `scripts/coverage.sh`（絶対 90% 床 + 相対ゲート）
7. プロダクトコードに `unwrap` / `expect` 0 件
8. **外形不変の証明**（裁定 2）: `GateApproved` の payload 形が変わっていないこと、
   RMU の投影ゴールデン（`projection_golden_test.rs` 19 本）が**無改変で全緑**であること
9. `core-command-use-case/Cargo.toml` の依存が `core-command-domain` **1 本のまま**
   （dev-dependencies を含め RMU / interface-adapter が現れないこと）
10. `StoreVersion` の grep 0 件、ポート doc の newtype 候補記述が却下理由付きで更新済み
11. 報告書に: 実装した経路表、裁定 2 の導出規則と upstream 突き合わせ根拠、
    テスト一覧、残った申し送り、迷った点

## 報告

`developer-report-1.md` に上記 11 を書く。**完了報告は鵜呑みにされない** — メインセッションが
全ゲートを独立再実行して受入判定する。
