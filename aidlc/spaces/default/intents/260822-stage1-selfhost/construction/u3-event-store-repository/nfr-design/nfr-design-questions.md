# nfr-design-questions — U3 SQLite EventStore と WorkflowExecutionRepository（`u3-event-store-repository`）

> NFR Design（Construction 3.3）の質問票（Unit: U3、kind: library、Bolt: B5）。出典: `../nfr-requirements/security-requirements.md`（NFR1.1〜NFR4.6 + レビュー所見 1 =
> TOLERANCE 引き締め）、`../nfr-requirements/tech-stack-decisions.md`、`../functional-design/functional-spec.md`（§1 配置、§3 フロー、§4 ワイヤ、§5 モデル、§6 退役、
> §7 テスト）、`../../../inception/contract-design/contract-summary.md`（C3 / C6）。
>
> **質問なし。** 基盤選択は機能設計 Q1〜Q4 で、前提は NFR 要求 P1〜P6 で確定済み。次の設計前提を確認して成果物へ進む。

## 前提（確認事項）

- P1. **検査点の設計**（NFR3.2 / NFR4.3 / NFR4.4）: 信頼しない入力（ストア）からドメイン型へ至る経路に 3 段の検査を置く — (1) 行の形（schema_version / type タグ /
  未知フィールド拒否 = ワイヤ復号）、(2) 値の形（Domain Primitive の parse: IntentId / StageSlug / PhaseId / PlanAction / CheckboxState …）、(3) 集約の不変条件
  （`from_state` / `apply_event`）。どの段の失敗も `Corrupt { cause }` へ写し、panic しない。
- P2. **障害ドメイン**: (a) ストア I/O（`Io`）— 呼出側へ返す、再試行しない。(b) 競合（`Conflict`）— ユースケースが再水和して 1 回だけ再試行（U5）。(c) 破損（`Corrupt` /
  `Schema`）— 中断。投影の修復（U4）はジャーナルから冪等。(d) `Busy` 超過（`Io(WouldBlock)`）— 中断して利用者に再実行を促す（文言は U7）。
- P3. **論理コンポーネント**: use-case `orchestration`（ポート + エラー + 値）、adapter `orchestration::{sqlite_event_store, wire, store_path, workflow_execution_repository_impl,
  memory}`、domain の是正、`formal/orchestration/journal_protocol.qnt` + fixtures + conformance test、`tools/lint` のルール削除、`scripts/{quint-gate,coverage}.sh` の更新
  （TOLERANCE 0.01）。Clock は機構モジュール（Gateway に数えない）。
- P4. **退役の安全手順**: 削除 → ビルド → grep 0 件 → 既存テスト（ゴールデン / engine_loop ITF / WorkflowDefinitionRepository）緑の順で、ロック系の削除を先に 1 コミット
  にまとめる（後続の差分をレビューしやすくする）。後方互換の残置なし。
- P5. **Quint DoD の設計**: 不変条件ごとに 1 変異モデル（code-summary に表で記録）、状態遷移レベルの不変条件、in-module witness。ITF fixture は `#meta` 正規化のうえ
  コミット（既存の engine_loop と同じ採取手順）。

## Consolidated Summary Confirmation

- 設計 = 3 段の検査点、障害ドメイン 4 種の扱い、論理コンポーネントの配置（層 = クレート）、退役の安全手順、Quint DoD。成果物は security-design.md /
  logical-components.md / traceability.json。

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
