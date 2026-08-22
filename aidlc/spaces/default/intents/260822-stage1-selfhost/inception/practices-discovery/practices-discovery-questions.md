# Practices Discovery — インタビュー質問

リード草案と独立レビュー3件（品質・開発者・DevSecOps）で確定できなかった論点。
証拠が答えを示唆するものは推奨を明記しているが、チームの意思決定は人間の判断。

## Q1. 最初に薄い縦串（walking skeleton）を作りますか？

Walking skeleton とは、全体を最初から最後まで貫いて動く最小版を最初に作り、部品がつながることを実証してから本機能を入れる進め方です。本リポジトリはコア〜アダプタ層まで三層品質保証（Quint/ITF/ゴールデンパリティ）で実証済みですが、**ユースケース〜CLI の縦串はテスト0本**（品質レビュー指摘）。

- A. 作らない（skeleton: off）— Bolt 1 も通常 Bolt。縦串はクリティカルパス項目6（doctor→ドッグフード）で自然に通る
- B. 作る（skeleton: on）— Bolt 1 を「Next/Report 1本を CLI から通す最小縦串」とし、単独ゲートで形を確認してから残りへ
- X. Other (please specify)

[Answer]: A. 作らない（skeleton: off）— Bolt 1 も通常 Bolt。縦串はクリティカルパス項目6（doctor→ドッグフード）で自然に通る（省略可能であることを確認のうえ選択）

## Q2. Testing Posture の確定 — TDD の適用範囲と Ordering 文言

オーナー明言（t_wada 流 TDD・テストピラミッド意識）を前提に、機械読取される `- **Ordering**:` を自己完結の1文で確定します。品質レビューの置換案:「新規プロダクションコードはレイヤーごとに red-green-refactor（失敗するテストを先に書く）で実装する。Quint モデル検査・ITF 準拠テスト・ゴールデンパリティは TDD サイクルの**外側**の受け入れゲートとして維持し、TDD の red を代替しない」

- A. この文言で確定（Methodology: tdd）
- B. 修正したい（X に修正文を記述）
- X. Other (please specify)

[Answer]: A. この文言で確定（Methodology: tdd）

## Q3. テストピラミッドの比率を定量化しますか？

- A. 定性のみ（「単体テスト優位・統合は境界ごと・E2E は最小」）— 比率は縛らない
- B. 比率を明文化（例: 単体 75% / 統合 20% / E2E 5% を目安として記載）
- X. Other (please specify)

[Answer]: A. 定性のみ（「単体テスト優位・統合は境界ごと・E2E は最小」）— 比率は縛らない

## Q4. main のマージゲート機械強制（品質レビュー・重大指摘）

実測で `main` に branch protection / ruleset が**未設定**。CI は走るが、赤のままでもマージできてしまう状態です（現状は人間運用で守られている）。

- A. branch protection（required checks: check / quint / coverage）を設定して機械強制する
- B. 現状のまま（人間運用）を明文化して受容する
- X. Other (please specify)

[Answer]: A. branch protection（required checks: check / quint / coverage）を設定して機械強制する

## Q5. カバレッジ 90% 床と未テスト層（composition root / CLI）

`coverage.sh` に除外設定はありません。今後 `modules/app/aidlc`（composition root・CLI）を実装すると 90% 床に直接効いてきます。

- A. CLI・composition root も TDD + ゴールデン出力テストの対象とし、90% 床を維持する
- B. composition root（main.rs の配線部分）だけカバレッジ除外を許し、それ以外は床維持
- X. Other (please specify)

[Answer]: B. composition root（main.rs の配線部分）だけカバレッジ除外を許し、それ以外は床維持

## Q6. サプライチェーン整備（DevSecOps 提案。select all that apply）

いずれも低コスト・規模相応の候補。採用するものをすべて選んでください。

- A. `cargo audit`（依存脆弱性監査）を CI に追加 — `tools/lint` の独立 Cargo.lock も対象
- B. `rust-toolchain.toml` でツールチェーンを固定（floating stable + `-D warnings` による CI 突然赤リスクを解消）
- C. `unsafe_code = "forbid"` を workspace lints へ昇格（現状はクレート単位で app スタブに漏れ）
- D. CI に `permissions: contents: read` を明示
- E. どれも採用しない
- X. Other (please specify)

[Answer]: A, B, C, D（cargo audit CI 追加 / rust-toolchain.toml 固定 / unsafe_code forbid の workspace 昇格 / permissions: contents: read 明示）

## Q7. stage-1 スコープに含める CI/リンタ整備（select all that apply）

設計監査 D 束ほか。Bolt 化して本 intent で消化するものを選んでください。

- A. tools/lint への CI 3ステップ追加（fmt/clippy/自己テスト — 監査 C27、現状どれも届いていない）
- B. PBT シード固定でカバレッジ相対ゲート許容 0.5pp → 0.01 へ引き締め
- C. macOS CI ジョブ追加（セルフホスト先は macOS 実機、CI は ubuntu のみ）
- D. main への push トリガー追加（現状 pull_request のみ）
- E. どれも stage-1 には含めない（後続 intent へ）
- X. Other (please specify)

[Answer]: A, B（tools/lint への CI 3ステップ追加 / PBT シード固定で 0.5pp → 0.01。C macOS ジョブと D push トリガーは stage-1 に含めない）

## Q8. エラーハンドリング様式を coding-rules 正本に追加しますか？

現状の実態: 手実装のエラー enum（thiserror / anyhow 不使用）。開発者レビューは「オーナー裁定を得て正本へ1ファイル追加する候補」と提案。

- A. 追加する（現行様式を規則として明文化。文面は統合時に起草しオーナー確認）
- B. 見送り（実態のまま、規則化しない）
- X. Other (please specify)

[Answer]: A. 追加する（現行様式を規則として明文化。文面は統合時に起草しオーナー確認）

## Consolidated Summary Confirmation

- Looks correct
- Request changes

[Answer]: Looks correct

## Requested Changes Feedback

[Answer]: q5の内容みたい。キー入力ミスったかも（Q5 を再提示して確認）
