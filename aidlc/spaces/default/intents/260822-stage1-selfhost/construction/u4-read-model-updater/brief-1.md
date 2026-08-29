# B8 委任ブリーフ 1 — U4 ReadModelUpdater と CQRS 層の側分割

Conversation language: 日本語
委任先モデル: Opus（クレート再編 + 逐語互換投影 — 複雑・高リスク実装）
最終責任: Fable 5 メインセッション（全 diff レビュー・検証の独立再実行・受入判定）

## 目的（2 つで 1 Bolt — オーナー裁定で B8 一本）

1. **CQRS 層の側分割**（2026-08-29 オーナー裁定）: `core-use-case` / `core-interface-adapter` を
   コマンド側とクエリ側に分割し、`core-{command,query}-` 接頭辞で改名する
2. **U4 = ReadModelUpdater の実装**: journal 差分読取 → 状態ファイル・監査シャード投影 →
   checkpoint 前進の冪等コンポーネント

## 構成の正本（必読・変更禁止）

`construction/u4-read-model-updater/crate-structure-proposal.md` — クレート配置・依存グラフ・
共有部品の行き先はここが正。要点:

- `modules/core/command/use-case` = `core-command-use-case`（旧 core-use-case の残部）
- `modules/core/command/interface-adapter` = `core-command-interface-adapter`（旧 core-interface-adapter の残部）
- `modules/core/query/read-model-updater` = `core-query-read-model-updater`（クエリ側の全実体:
  読取語彙 + `JournalReaderImpl` + 取得ループ + 純粋投影核 + 投影ライタ）
- 共有部品: エラー分類（旧 `CorruptCause`）と `io_kind` 写像は**側ごとに専用化**（実使用
  変種だけを持つ — 無用変種を持ち込まない）、`StorePath` と `EVENT_MANIFEST` は
  **`core-domain` へ移動**（workspace 文脈 / イベント enum の隣）
- 依存規則はコード化済みの `coding-rules/cqrs-boundaries.md`（2026-08-29 改訂版）に従う

## その他の必読

- coding-rules 全 16 規則（README から）。特に cqrs-boundaries（改訂版）/
  no-backward-compatibility / field-visibility / ubiquitous-language / tell-dont-ask
- ADR-009 の 2026-08-28 / 2026-08-29 追記、ADR-010（decisions.md）
- `inception/units-generation/unit-of-work.md` U4（責務の正本 — 原文が正）
- `docs/specs/11-workspace.md` §3（`render_audit_block` / `state_writers` の逐語表 — **これらは
  core-domain の workspace 文脈に実装済み**であり、U4 は投影 API への転居 + 投影駆動の実装。
  `find_all_events` / `classify_state_version` は domain に残す — 仕様に明記）
- `inception/contract-design/contract-summary.md` C5（投影規則: ドメインイベント → 監査行 N 行・
  86 語彙逐語互換）と C6
- ゴールデン: `tests/golden/`（U1 採取の upstream 実出力）と既存 `golden_parity_test.rs`

## 固定裁定（変更禁止。矛盾を見つけたら読み替えず止めて報告）

1. **後方互換ゼロ**（オーナー再指示 2026-08-29「後方互換コードは残すな」）: 旧クレート名・
   旧パスの再輸出・shim・`#[deprecated]` 禁止。改名は呼出側一斉修正で行う
2. RMU は**二層**: 取得ループ（`&mut self` — checkpoint 読取 → `events_after` → 投影核 →
   `advance_checkpoint`）+ 純粋投影核 `project(entries: &[JournalEntry], read_model: &mut …)`
   （2026-08-28 裁定 + 論点 B = A）。投影核はストレージ・接続・checkpoint を知らない
3. `JournalReaderImpl` の**挙動は移動で変えない**: rowid カーソル / `amadeus_projection_checkpoint` /
   (aid, seq_nr) アンカー照合 / busy_timeout / CREATE なし接続はそのまま
4. 投影出力は **0a 逐語契約**: 監査シャードは 86 語彙・見出し・フィールド順・行終端エスケープを
   逐語互換で描画（W9）。状態ファイルは tmp+rename 原子性。**冪等**（同じ checkpoint から
   何度流しても同一バイト — NFR3）
5. 監査シャード横断の位置付き読取（timestamp ソート + バッファ位置 tiebreak — FR1.1）
6. U7（合成ルート）は実装しない — B8 では RMU をテストから直接駆動する
7. Quint モデル不変。`journal_protocol` ITF のフェイク投影を実 RMU に差し替えても緑にする
8. TDD（red-green-refactor、レイヤーごと）。テストは実装の移動先へ追従させる

## 所有ファイルと作業規律

- 書いてよい: `Cargo.toml`（workspace members）/ `Cargo.lock` / `modules/**` /
  報告書 `construction/u4-read-model-updater/developer-report-1.md`
- 禁止: `docs/**`・`formal/**`・`aidlc/**`（報告書以外）・`.github/**`。push 禁止。
  本家リポジトリへの接触禁止
- **`git add -A` 禁止** — 明示パスで add（監査シャード等の巻き込み事故防止）
- **検証は `CARGO_TARGET_DIR=$PWD/target-delegate`** で実行（委任者の検証と衝突させない）
- コミットは意味単位で分割（例: 側分割 → RMU 骨格 → 投影本体 → 検収）。メッセージは
  日本語で「b8: 」接頭辞

## 受入基準（全部を自分で実行して報告書へログを貼る）

1. `cargo fmt --all --check` / 2. `cargo clippy --workspace --all-targets -- -D warnings` /
3. `cargo lint` / 4. `cargo test --workspace` 全緑 / 5. `bash scripts/quint-gate.sh` 緑 /
6. `CARGO_TARGET_DIR=$PWD/target-delegate bash scripts/coverage.sh --base origin/main` 両ゲート PASS
   （新設エラー枝は到達テストを書いてから測る）/ 7. プロダクトコードに unwrap/expect なし、
   `#[allow]` は理由必須 / 8. 投影出力が `tests/golden/` の該当ゴールデンとバイト一致

## 報告書に必ず書くこと

変更概要（クレート対応表・行数増減）/ 固定裁定 1〜8 の実施箇所（file:line）/
受入基準 1〜8 の実行ログ末尾 / 独自解釈の列挙（空欄可）/ 仕様とのドリフト（doc-sync 向け）
