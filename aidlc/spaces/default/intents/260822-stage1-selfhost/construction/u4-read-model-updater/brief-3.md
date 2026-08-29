# B8 委任ブリーフ 3 — 第 2 裁定の手戻り（ドメインはコマンド側・クエリ側のドメイン依存禁止）

Conversation language: 日本語
委任先モデル: Opus（クエリ側の wire parse 新設 — 複雑・高リスク）
最終責任: Fable 5 メインセッション（全 diff レビュー・検証の独立再実行・受入判定）

## 裁定（2026-08-29 第 2 裁定、オーナー逐語）

「`modules/core/domain` ではなく `modules/core/command/domain` です。ドメインはコマンド側なのです。
クエリ側からドメインに絶対依存しないで。」クレート名は `core-command-domain` 系で可。

正本は改訂済み: `coding-rules/cqrs-boundaries.md`（判定表・図式）、ADR-009 第 2 改訂、
`crate-structure-proposal.md`。先に読むこと。

## 固定裁定（変更禁止。矛盾があれば止めて報告 — 前回どおり）

1. **改名**: `modules/core/domain` → `modules/core/command/domain`、パッケージ
   `core-domain` → `core-command-domain`。後方互換ゼロ（旧名 alias 禁止・呼出側一斉修正）
2. **RMU の `Cargo.toml` から `core-domain`（新名含む）を完全除去**。依存してよいのは
   `core-infrastructure` / `audit-events` / `message-catalog` / 外部ライブラリのみ
3. **クエリ側は wire を自前の型で parse**: ジャーナル payload（本家既定 serde_json 形式）を
   RMU 自前のイベント型（12 変種 + Started の計画・表示属性・走査結果）へ復号する。
   `JournalEntry` の intent_id / 各値も RMU 自前の型に。**wire の綴りはコマンド側の serde 出力が
   正本** — 推測せず、実直列化から逆算すること
4. **コントラクトテストを合成ルート（app/aidlc の tests）に新設**: コマンド側の全 12 変種
   （境界値込み）を直列化 → RMU の parse → 投影に使う全材料が同値、を機械検証。manifest 定数も
   両側の値の同値をここで固定
5. **リードモデル語彙の移動**: `audit_field.rs`（AuditFieldKey/Value/Fields）・
   `audit_ordering.rs`（順序付け純関数）・単一行プリミティブ（クエリ側が使う分）を
   domain → RMU へ。コマンド側にしか使い手が残らないものは domain に残してよい
   （`StageDisplay` の単一行保証はコマンド側の関心なので domain 側に残す）
6. **`StorePath` はコマンド側に残す**。RMU の open 系 API はストアの場所を自前の型
   （例: `StoreLocation`）で受け取る。合成ルート（現状はテスト）が結線する
7. **挙動不変**: 投影の出力バイト・ゴールデン検収（42 ブロック + 10 ケース両面一致）・
   rowid カーソル・アンカー照合・Quint 不変（quint-gate 緑）はすべて維持
8. TDD。テストは移動先へ追従

## 所有ファイル・規律（ブリーフ 1 と同じ）

`Cargo.toml` / `modules/**` / 報告書 `developer-report-2.md`。docs・formal・aidlc（報告書以外）
禁止。push 禁止。`git add -A` 禁止。検証は `CARGO_TARGET_DIR=$PWD/target-delegate`。
コミット分割・日本語「b8: 」接頭辞。**私はあなたの完了報告までコミットしない**（凍結）。

## 受入基準

ブリーフ 1 の 1〜8 と同一（全部を自分で実行しログを報告書へ）+ 追加 2 点:
9. `grep -rn "core-command-domain" modules/core/query/` が 0 件（ドメイン依存ゼロの機械確認）
10. コントラクトテスト（固定裁定 4）が全 12 変種で緑
