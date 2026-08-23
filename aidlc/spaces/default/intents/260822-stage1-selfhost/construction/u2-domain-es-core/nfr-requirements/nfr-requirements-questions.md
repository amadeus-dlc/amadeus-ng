# nfr-requirements-questions — U2 ドメイン ES コア（`u2-domain-es-core`）

> NFR Requirements（Construction 3.2）の質問票（Unit: U2、kind: library）。出典: `../functional-design/functional-spec.md`
> （W1〜W7、§5 エラー一覧）、`../functional-design/rules.md`（BR1.0〜BR5.4）、`../../../inception/requirements-analysis/requirements.md`
> （NFR1〜NFR5、FR1.3 / FR2.1 / FR3.1 / FR3.3 / FR8.3 / FR8.4）、`../../../inception/contract-design/contract-summary.md`
> （C3 / C5 / C6）、`aidlc/spaces/default/codekb/docs/technology-stack.md`（既存依存: proptest 1.11 / Quint 0.32.0、ドメインは
> serde 非依存が規約）、`aidlc/spaces/default/memory/team.md`（Testing Posture: TDD、3 層品質保証、カバレッジ 90% 床）。
>
> **質問なし。** U2 は `core-domain` の純粋な集約（I/O なし・同期・serde なし）であり、適用される NFR は NFR1（upstream
> 互換 — engine_loop 契約の維持）・NFR2（品質ゲート）・NFR3（監査完全性の集約側前提 — 決定論的 replay）・NFR4（サプライ
> チェーン — 依存追加なし）で、いずれも先行ステージ（ADR-001〜007、機能設計 BR2.3 / BR2.5 / BR5.x、team.md）で方針が
> 確定している。NFR5（性能）は非目標。構築フェーズの質問は本当の空白だけに絞る方針に従い、次の前提を確認して成果物へ進む。

## 前提（確認事項）

- P1. 技術選定: **新規クレート依存なし**。集約・イベント（12 変種）・`StageIndex`・`WorkflowExecutionSnapshot` は手書きの Rust 型で、
  `core-domain` は引き続き serde / serde_json に依存しない（BR5.2 — JSON 化は U3 のワイヤ構造体）。PBT は既存 proptest 1.11、
  ITF 準拠テストは既存 Quint 0.32.0（`engine_loop.qnt`）のトレースを再生する。非同期ランタイムは不要（純粋・同期）。
- P2. upstream 互換（NFR1）の担保方法: 集約自体は観測可能面（CLI 出力・状態ファイル・監査行）を持たず、互換は U4 の投影と
  U7 の CLI が担う。U2 の NFR1 義務は **engine_loop 契約の維持** — ITF 準拠テスト（`modules/core/domain/tests/` の engine_loop
  2 ファイル）を decide → apply 経路で通し、BR2.5 の射影表（Quint の stage 0 ⇔ initialization 1 ステージの合成計画）で
  状態射影を突き合わせる。`gated(stage) = phase ≠ initialization` の実グラフ適用は upstream の next ラダー（initialization
  フェーズは gate = false）と一致する（オーナー裁定 A、2026-08-23）。
- P3. 品質ゲート（NFR2）: TDD（失敗するテストを先に書く）、PBT を `PROPTEST_RNG_SEED` 固定下で決定的に実行し、次の性質を
  固定する — (a) decide 後の状態 == 旧状態 + apply_event(event)（BR1.1）、(b) replay(events) == execute(commands)（BR2.3）、
  (c) seq_nr の単調性と SequenceGap 検出（BR2.1）、(d) Quint 不変条件（cursor_in_scope / at_most_one_active / no_gate_bypass /
  parked_position / unpark_restores_position）が任意のコマンド列で保たれる。カバレッジ 90% 床（ドメインクレートは既存水準
  95% 前後を維持）。CI 3 ジョブ + audit 緑、`cargo lint`（no-public-fields ほか）緑。
- P4. 監査完全性（NFR3）の集約側前提: 再構成は「最新スナップショット + seq_nr 以降の replay」（C3）。集約側が保証するのは
  決定論的 replay・SequenceGap / UnknownStage / InvariantViolation の検出・`from_snapshot` の不変条件検証であり、クラッシュ
  再構成テスト（ジャーナル → 集約 → 投影）は U3 / U4 が担う。
- P5. セキュリティ（NFR4）: `unsafe_code = "forbid"`（workspace lint、U10 で昇格済み）。依存追加なしのため `cargo audit` への
  影響なし。入力（`WorkflowDefinition`、コマンド引数）は信頼境界の内側 — 境界検証は `StartError` / `CommandError` /
  `ApplyError` / `SnapshotError` の Err で行い panic しない（BR5.1: `StageIndex` で範囲を型保証、`# Panics` なし）。
  イベントペイロードに載る人間入力（`request` / `user_input` / `feedback` / `reason`）は upstream が監査行へ逐語記録する
  ものと同じで、集約は内容を解釈・検証せず運ぶだけ（PII の扱いは upstream 同等 — 平文、秘密情報は載せない前提）。
- P6. 性能（NFR5）: 数値目標なし。replay は O(イベント数)、1 intent あたり数百イベント・33 ステージ規模で計測しない。
  スナップショット頻度・ジャーナル I/O は U3 の設計事項。

## Consolidated Summary Confirmation

- U2 に固有の NFR 質問はなし。適用 NFR は NFR1（engine_loop 契約の維持 — ITF 準拠テストを decide → apply 経路で通す、
  BR2.5 の射影表）、NFR2（TDD + 決定的 PBT で 1 コマンド 1 イベント / replay 決定性 / seq_nr 単調性 / Quint 不変条件を固定、
  カバレッジ 90% 床、CI 3 ジョブ + audit + cargo lint 緑）、NFR3（集約側: 決定論的 replay と不変条件検証、クラッシュ再構成
  テストは U3 / U4）、NFR4（依存追加なし、unsafe forbid、Err で拒否し panic しない）
- 技術選定（P1）: 新規クレート依存なし、core-domain は serde 非依存のまま、PBT は proptest、ITF は Quint 0.32.0
- 互換（P2）: 非ゲート = initialization フェーズ（オーナー裁定 A）、Quint の stage 0 は合成計画で 1:1
- 性能（P6）: 数値目標なし（NFR5）

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
