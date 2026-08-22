# nfr-requirements-questions — U1 canon-json とゴールデン（`u1-canon-json-goldens`）

> NFR Requirements（Construction 3.2）の質問票（Unit: U1、kind: library）。出典: `../functional-design/functional-spec.md`、
> `../functional-design/rules.md`（BR1.x / BR2.x）、`../../../inception/requirements-analysis/requirements.md`（NFR1〜NFR5）、
> `../../../inception/contract-design/contract-summary.md`（C1 / C7）、`aidlc/spaces/default/codekb/docs/technology-stack.md`
> （既存依存: serde 1.0.229 / serde_json 1.0.151、unsafe forbid、cargo audit は U10 で追加）、`docs/adr/0001-canonical-json-serializer.md`。
>
> **質問なし。** U1 は依存ゼロの純粋ライブラリで、適用される NFR は NFR1（upstream 互換）・NFR2（品質ゲート）・
> NFR4（サプライチェーン）だけであり、いずれも先行ステージと ADR で数値・方針が確定している。NFR3（監査完全性）と
> NFR5（性能は非目標）は本 Unit に固有の要求を持たない。構築フェーズの質問は本当の空白だけに絞る方針に従い、
> 次の前提を確認して成果物へ進む。

## 前提（確認事項）

- P1. 技術選定: sha256 は `sha2`（pure Rust・RustSec 既知脆弱性なし・広く使われる）、JSON の読取は既存の
  `serde` + `serde_json`（`preserve_order` をワークスペース全体で有効化 — ADR 0001 決定 3）、数値と文字列の
  書き出しは canon-json 内の JS 互換ライタ（ryu/serde_json 既定フォーマッタを契約経路では使わない — ADR 0001 決定 4）。
  新規のランタイム依存はこの 2 つのみ。bun は 0b 採取の開発時ツールでプロダクト依存にしない（D1）。
- P2. セキュリティ: canon-json が読む JSON はワークスペース内の契約ファイルとゴールデン（信頼境界の内側）だが、
  入力検証は境界で行う — 不正 JSON は `ParseError` で拒否、serde_json の再帰深さ上限（既定 128）を維持して
  深いネストによるスタック枯渇を防ぐ。非有限数・制御文字は BR1.3 / BR1.4 の規則で決定的に扱う。秘密情報・
  PII を扱わない。`unsafe_code` forbid はワークスペース lint（U10）で強制。
- P3. 品質ゲート（NFR2）: TDD（ゴールデン先行 = red を作ってから green）、カバレッジ 90% 床、CI 3 ジョブ green、
  PBT（ラウンドトリップ: parse → serialize の決定性、hash の冪等性）。
- P4. 性能（NFR5）: 数値目標なし。ワンショット CLI の 1 回の直列化は KiB 規模で、計測は行わない。

## Consolidated Summary Confirmation

- U1 に固有の NFR 質問はなし。適用 NFR は NFR1（upstream 互換 — 3 プロファイルのバイト一致をゴールデンで固定）、NFR2（品質ゲート）、NFR4（サプライチェーン: sha2 / serde / serde_json のみ追加、unsafe forbid、cargo audit clean）
- 技術選定（P1）: sha2、serde + serde_json（preserve_order 有効）、JS 互換の自前ライタ。bun は開発時のみ
- セキュリティ（P2）: 不正 JSON は ParseError、再帰深さ上限 128 を維持、非有限数・制御文字は規則で決定的に処理、秘密情報・PII なし
- 品質（P3）: ゴールデン先行の TDD、カバレッジ 90% 床、PBT でラウンドトリップと冪等性
- 性能（P4）: 数値目標なし（NFR5）

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
