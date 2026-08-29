# B8 委任ブリーフ 4 — 是正版の最小手戻り（改名・移動 2 件のみ）

Conversation language: 日本語
委任先モデル: Opus（B8 全体の文脈を持つ現任者による機械的改名）
最終責任: Fable 5 メインセッション（全 diff レビュー・検証の独立再実行・受入判定）

## 経緯（1 段落）

ブリーフ 3（RMU の wire parse 化）は**私の誤導出につき全面撤回**。オーナーの裁定は
(1) ドメインの移動・改名 (2) クエリ側クレートのドメイン依存絶対禁止（恒久制約）
(3) RMU は中間 — コマンド側のドメインイベントにもクエリ側にも依存できる、の 3 点であり、
**B8 実装の依存構造（RMU がドメインイベント型に依存）は正しく、手戻り不要**。
残る作業は改名・移動 2 件だけ。是正済みの `cqrs-boundaries.md` / ADR-009 改訂 2 /
`crate-structure-proposal.md` を先に読むこと。

## 固定裁定（変更禁止）

1. **ドメインの移動・改名**: `modules/core/domain` → **`modules/core/command/domain`**、
   パッケージ `core-domain` → **`core-command-domain`**。全参照の一斉修正。後方互換ゼロ
2. **RMU の改名**: `modules/core/query/read-model-updater` → **`modules/core/read-model-updater`**、
   パッケージ `core-query-read-model-updater` → **`core-read-model-updater`**（RMU は中間 —
   側接頭辞を持たない）。全参照の一斉修正。後方互換ゼロ
3. **それ以外のコード変更禁止**（依存関係・型・投影・テストの挙動は現状維持。参照更新に伴う
   機械的な追随のみ可）。doc コメント内の旧クレート名・旧パスも更新すること
4. ブリーフ 3 で着手した変更が残っていれば**全て破棄**してから始める

## 所有ファイル・規律（ブリーフ 1 と同じ）

`Cargo.toml` / `Cargo.lock` / `modules/**` / 報告書 `developer-report-2.md`（簡潔でよい —
改名対応表・受入基準ログ・ブリーフ 3 の破棄確認）。`git add -A` 禁止。push 禁止。
検証は `CARGO_TARGET_DIR=$PWD/target-delegate`。私は完了報告までコミットしない。

## 受入基準

1〜7 はブリーフ 1 と同一（fmt / clippy / lint / test / quint-gate / coverage 両ゲート / unwrap 0）。
追加:
8. `grep -rn "core-query-read-model-updater\|modules/core/domain\b\|\"core-domain\"" modules/ Cargo.toml` が 0 件
9. ゴールデン検収（42 ブロック + 10 ケース両面一致）が改名後も緑のまま
