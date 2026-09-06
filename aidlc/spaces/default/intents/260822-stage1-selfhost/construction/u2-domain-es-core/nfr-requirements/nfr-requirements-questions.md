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

## 以前の前提（2026-08-23 の記録 — 旧 WorkflowExecution 世代。失効項目は下の再走で上書き）

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

## 以前に確認済みのまとめ（2026-08-23）

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

## 2026-09-07 再走（Modify）— 前提の更新

> 出典: 2026-09-05 是正済み・2026-09-07 再走の機能設計（`../functional-design/entities.md` / `rules.md` / `functional-spec.md`
> — 16 変種、最新スナップショット + 差分再生、壊れた歴史は panic、`IntentMismatch`、BR5.5 FCC 化、第 9 節の引継ぎ、
> advisory レビュー所見 R-01〜R-10）、現行コード（`modules/core/command/domain/Cargo.toml`: runtime = chrono / uuid(v7) /
> core-infrastructure、dev = proptest / serde_json。`tests/` = `engine_loop_conformance.rs` + `collection_contract_test.rs`）、
> U10 で実測した CI（`rust-toolchain.toml` 1.95.0、workspace lints rust 5 + rustdoc 1 + clippy 44 = 50、`scripts/coverage.sh`
> 90 床 / TOLERANCE 0.01 / PROPTEST_RNG_SEED 20260823、全体カバレッジ 99.14%、`cargo audit` clean）、
> `formal/orchestration/engine_loop.qnt` v2.7。**質問なし** — 以下はすべて既存裁定・実測の適用であり、前提として確認する。

- P7. **クレート配置と依存ベースラインの更新**: 対象クレートは `core-command-domain`（`modules/core/command/domain`。旧
  `core-domain` は失効）。runtime 依存の実測ベースラインは `chrono`（時刻の値）、`uuid`（v7 — 識別子の検査とイベント ID の採番、
  オーナー裁定 2026-09-02）、`core-infrastructure`（言語拡張: canon_json と collections。serde derive・ストア語彙は持たない）、
  dev は `proptest` / `serde_json`（ITF JSON 読取のみ）。旧 NFR4.1 の「内部クレート 3 つ」ベースラインは失効し、
  **FCC 化・`next_decision` 改修で外部クレートを 1 つも足さない**を新しい NFR4.1 とする。
- P8. **失敗境界の裁定同期**: 旧 NFR3.2 / NFR4.3「panic しない・境界で Err」は 2026-08-30 のオーナー裁定（壊れた歴史は回復せず
  クラッシュ、`replay` / `apply_event` は Result を返さない）で失効。新しい NFR3.2 は「DTO → 集約基底（`IntentExecution::new`）
  と封筒・通番検査は Err（アダプタが `RepositoryError::Corrupt` へ写す）、型変換後の壊れた歴史は panic」と二境界で書く。
  プロダクトコードの `unwrap` / `expect` 禁止（project.md）は維持し、panic は明示裁定の射程（apply の内部検査違反）に限る。
- P9. **再生方式と決定性**: 旧 NFR3.1「apply は時刻・乱数を読まない」は apply について維持。コマンドは UUIDv7 のイベント ID を
  採番する（時刻 + 乱数）ため、決定性の性質は「同じ旧状態に返された同じイベントと通番・時刻を適用した結果と同値」（BR1.1 事後条件）
  で定義し、別実行同士のイベント ID 同値を前提にしない。再構成は最新スナップショット + `seq_nr` より大きい差分（BR2.3、
  オーナー裁定 2026-09-05）。旧 NFR3.3 の `snapshot()` / `from_snapshot` 値オブジェクトは撤去済みで、スナップショットは
  `IntentExecution` 自身（DTO はアダプタ所有）。
- P10. **契約の更新**: NFR1.3 のイベント語彙は 16 変種（`StageCompleted` は廃止、`schema_version` はドメイン属性ではない）。
  NFR1.1 の ITF 準拠テストは `modules/core/command/domain/tests/engine_loop_conformance.rs` 1 本、モデルは `engine_loop.qnt`
  v2.7（旧「モデル不変」は失効 — v2.1〜v2.7 の改訂は各 Bolt のゲートで採択済み。U2 の再走ではモデルを変更しない）。
  旧 NFR3.4（`next_decision` の `DefinitionMismatch`）は機能設計 Q5 = A の `IntentMismatch` に置換し、定義 ID の照合は
  `Intent::resolve_review_policy(&WorkflowDefinition, ..)` 側（LineageMismatch）に残る。
- P11. **FCC 化の品質ゲート（新規 NFR2.5）**: BR5.5 で新設する FCC（StageEntries / StageSlots / StageIndexSet / ArtifactPaths /
  StageSlugSet / PromotedSections / RuleLines / TransitionSteps ほか）は既存の契約試験ハーネス
  `modules/core/command/domain/tests/collection_contract_test.rs` へ登録し、集合型は `combine` / `divide` の Monoid 則
  （結合法則・左右単位元・冪等・交換）と差集合則（`A \ A = empty`、`A \ empty = A`）を性質試験で固定する。順序付き列は
  連結の順序保持と slug 衝突の Result 拒否を固定する。レビュー所見 R-01（4 系統の型定義不足）/ R-02（use-case と ITF テストの追随）/
  R-03（StageSlugSet は辞書順）は code-generation 計画の必須入力として NFR の合格基準に引き継ぐ。
- P12. **CI 実測値の更新**: workspace lints 50（rust 5 + rustdoc 1 + clippy 44 — 旧 48 は失効）、`cargo lint` 6 規則、
  `rust-toolchain.toml` 1.95.0、カバレッジ床 90% + 相対ゲート TOLERANCE 0.01 + `PROPTEST_RNG_SEED=20260823`、直近実測 99.14%
  （U10、全体）。ドメインクレート単独の基準値は本再走で `cargo llvm-cov --package core-command-domain` を 1 回計測して記録する
  （計測できなければ code-generation 着手時に繰り延べ）。CI は 7 ジョブ・必須 4 コンテキスト（U10）。

## Consolidated Summary Confirmation

2026-09-07 再走（Modify）。質問なし。旧世代（2026-08-23）の前提・まとめは履歴として残し、以下を現行の確認事項とする。

- 適用 NFR は変わらず NFR1（契約の維持）/ NFR2（品質ゲート）/ NFR3（監査完全性の集約側前提）/ NFR4（サプライチェーン・失敗境界）、
  NFR5 は非目標。枝番は現行裁定へ同期し、新規 NFR2.5（FCC の契約試験・Monoid 則）を追加する
- P7: クレートは `core-command-domain`、依存ベースラインは chrono / uuid(v7) / core-infrastructure + dev proptest / serde_json。
  外部クレートを 1 つも足さない
- P8: 失敗境界は二層 — DTO・封筒・通番の検査は Err（Corrupt）、型変換後の壊れた歴史は panic（オーナー裁定 2026-08-30）。
  unwrap / expect 禁止は維持
- P9: 決定性は BR1.1 事後条件で定義（イベント ID は UUIDv7 採番、別実行同士の ID 同値を前提にしない）。再構成は最新
  スナップショット + 差分（2026-09-05 裁定）。snapshot 値オブジェクトは撤去済み
- P10: 16 変種、ITF は `engine_loop_conformance.rs` 1 本・モデル v2.7（U2 では変更しない）、`next_decision` は `IntentMismatch`
- P11: FCC の契約試験ハーネス登録と Monoid 則・差集合則の性質試験（NFR2.5）。R-01〜R-03 は code-generation 計画の必須入力
- P12: lints 50、toolchain 1.95.0、coverage 90 床 / 0.01 / seed 固定、全体 99.14%。ドメイン単独の基準値を本再走で計測
- 旧 READY レビュー節（2026-08-23）は `security-requirements-review-history-2026-08-23.md` へ退避し、独立レビューは同期後の本文に行う

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
