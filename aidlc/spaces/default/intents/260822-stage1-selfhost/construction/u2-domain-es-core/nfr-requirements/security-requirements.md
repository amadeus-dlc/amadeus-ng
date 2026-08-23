# security-requirements — U2 ドメイン ES コア（`u2-domain-es-core`）

> NFR Requirements（Construction 3.2）成果物（Unit: U2、kind: library）。出典: `../functional-design/functional-spec.md`
> （W1〜W7、§4 状態遷移、§5 エラー一覧）、`../functional-design/rules.md`（BR1.0〜BR1.9 不変条件 / BR2.1〜BR2.5 イベントと
> リプレイ / BR3.x next_decision / BR4.x PlanAction / BR5.x 型・スナップショット）、`../../../inception/requirements-analysis/
> requirements.md`（NFR1 upstream 互換・NFR2 品質ゲート・NFR3 監査完全性・NFR4 セキュリティ/サプライチェーン・NFR5 性能非目標、
> FR1.3 / FR2.1 / FR3.1 / FR3.3 / FR8.3 / FR8.4）、`../../../inception/contract-design/contract-summary.md`（C3 Repository ポート、
> C5 イベント語彙、C6 snapshot 列）、`aidlc/spaces/default/codekb/docs/technology-stack.md`（既存依存、ドメインは serde 非依存、
> proptest 1.11、Quint 0.32.0）、`aidlc/spaces/default/memory/team.md`（Testing Posture: TDD・3 層品質保証・カバレッジ 90% 床）、
> `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/`、確認事項 `nfr-requirements-questions.md`（前提 P1〜P6、Looks correct）。
>
> 各要求は Inception の NFR ID を継承し枝番を付ける（NFR1.x / NFR2.x / NFR3.x / NFR4.x）。NFR5 は非目標として §5 に置く。
> 2026-08-23 改訂: オーナー裁定（ADR-008 — WorkflowDefinition のエンティティ ID と集約間の ID 参照）を NFR3.4 として追加し、レビュー所見（依存ベースライン /
> ITF 対象 / snapshot 列挙 / カバレッジ根拠 / lint 数 / `start` は記録のみ）を是正した。

## 1. 範囲と信頼境界

- U2 は `core-domain` クレート内の **純粋な集約**（`WorkflowExecution`）とそのイベント・スナップショット・`StageIndex`。I/O なし・
  同期・serde なし（BR5.2）。ネットワーク・認証・認可・永続化・ログ出力を持たない。
- 入力は (a) `WorkflowDefinition`（U3 の `WorkflowDefinitionRepositoryImpl` が 3 入力を parse-don't-validate で検証し、識別子 `WorkflowDefinitionId` と
  内容版 `DefinitionRevision` を付与した集約 — 信頼境界の内側。集約は `definition_id` で間接参照する — BR2.6 / ADR-008）、(b) ユースケース（U5 / U6）からのコマンド引数（scope / request / user_input / feedback / reason / artifacts /
  phase_boundary / target / flips / mode）、(c) 再水和時のスナップショットとイベント列（U3 が SQLite から復号したもの）。
- 出力は (a) ドメインイベント（U3 がジャーナルへ、U4 が投影へ）、(b) `WorkflowExecutionSnapshot`（U3 が C6 snapshot.payload へ）、
  (c) `NextDecision` / `JumpDirection`（U6 / U7 が directive に写す）。JSON 化・監査行の描画は U3 / U4 の責務で、U2 の境界には
  バイト列は現れない。
- 秘密情報・鍵は扱わない。イベントペイロードの人間入力（`request` / `user_input` / `feedback` / `reason`）は upstream が監査行に
  逐語記録する（`audit-format.md`「Human decisions recorded verbatim」）ものと同じで、集約は内容を解釈・検証・加工せず運ぶ。

## 2. 要求

| ID | 要求 | 合格基準 | 出典 |
|---|---|---|---|
| NFR1.1 | **engine_loop 契約の維持** — Quint `engine_loop.qnt` の ITF トレースを decide → apply 経路（`start` + 各コマンド、Err はトレースの拒否遷移に対応）で再生し、BR2.5 の射影表で状態（status / parkedAt / autonomous / checkbox / cursor / lastDirective）を突き合わせる。書き換え対象は `modules/core/domain/tests/engine_loop_conformance.rs` **1 ファイルのみ**（現行 API `report_forward` 等 → 新 API）。同ディレクトリの `audit_lock_conformance.rs` は `LockProtocol` 対象で `WorkflowExecution` に触れず、改訂版 `audit_lock.qnt` の ITF 準拠は U3 の合格基準（FR1.2）— 本 Unit の管轄外 | `engine_loop_conformance.rs` が新 API で全緑。Quint ゲート（`scripts/quint-gate.sh`）は不変（モデルは変更しない） | NFR1, BR2.5, team.md Testing Posture（ITF は TDD の外側の受け入れゲート） |
| NFR1.2 | **ゲート判定の upstream 一致** — `gated(stage) = phase ≠ initialization`（オーナー裁定 A）。Quint の「stage 0 = 非ゲート」は ITF 準拠テストが使う合成計画（initialization 1 ステージ）上でのみ 1:1 で、実グラフ（initialization 3 ステージ）では upstream の next ラダー（initialization フェーズは gate = false、jump 禁止）と一致する | 実グラフ由来の `StageEntry` 列で (a) 索引 0〜2 が非ゲート、(b) 索引 3 以降が gated、(c) jump の target に initialization を指定すると InvalidTarget、になるテストが緑。birth は `start` + `complete_stage` ×3 で C5 の Started 投影と同じ監査行（U4 の投影テストで確認 — C5 改訂提案） | NFR1, BR1.3 / BR1.6, C5, orchestration-next-ladder |
| NFR1.3 | **イベント語彙の契約安定性** — 12 変種（C5 の 11 + StageCompleted）、封筒 `schema_version = 1`、ペイロードは C5 の形（ステージ参照は StageSlug）。C5 からの逸脱は `entities.md` の `c5_revision_proposal`（StageCompleted 追加 / Started.stages / Started.definition_id・definition_revision / 投影規則の改訂提案）に列挙したものに限り、U4 との合意事項として code-generation の計画に引き継ぐ | 変種名・ペイロードのフィールド集合が C5（+ 改訂提案）と一致することを U3 のワイヤ構造体テスト／U4 の投影テストで固定（U2 側は列挙型の網羅 match で変種数 12 をコンパイル時固定） | NFR1, BR2.4, C5 |
| NFR2.1 | **TDD** — レイヤーごとに red → green → refactor（失敗するテストを先に書く）。ITF 準拠テスト・ゴールデンは TDD の外側の受け入れゲートとして維持し red を代替しない | Bolt B3 の PR でテスト先行のコミット列（または 1 コミット内で `tests` → `src` の順）が確認でき、`cargo test --workspace` 全緑 | NFR2, team.md Testing Posture（Methodology tdd） |
| NFR2.2 | **決定的 PBT** — proptest を `PROPTEST_RNG_SEED` 固定下で実行し、任意のコマンド列について (a) decide 後の状態 == 旧状態 + `apply_event(event)`（BR1.1）、(b) `replay(events) == execute(commands)`（BR2.3）、(c) seq_nr の単調性と `SequenceGap` 検出（BR2.1）、(d) Quint 不変条件 cursor_in_scope / at_most_one_active / no_gate_bypass / parked_position / unpark_restores_position（BR1.2 / BR1.3 / BR1.7）、(e) Err は状態不変（BR1.1）を固定する | 5 性質の proptest が CI で緑。同一コードの 2 回計測でテスト結果・カバレッジ行が一致（シード固定、U10 NFR2.4） | NFR2, BR1.1 / BR1.2 / BR2.1 / BR2.3, team.md（PBT は集約本体同居） |
| NFR2.3 | **カバレッジ** — 絶対 90% 床を維持する（実測はワークスペース全体 94.87〜95.29%、`scripts/coverage.sh` — ドメインクレート単独の実測値は未取得）。除外設定は composition root（`main.rs`）のみで、U2 のコードに除外を足さない | `scripts/coverage.sh` が `[PASS] absolute gate` + PR 相対ゲート（`TOLERANCE` は U10 の暫定値）緑。Bolt B3 の着手時に `cargo llvm-cov --package core-domain` を 1 回取り、ドメイン単独の基準値を計画に記録する（以後の下限） | NFR2, team.md（カバレッジ） |
| NFR2.4 | **規則の機械強制を緑で通す** — `cargo clippy --workspace --all-targets -- -D warnings`（`[workspace.lints]` 実測 48 ルール deny = rust 5（`unsafe_code` / `missing_docs` / `unsafe_op_in_unsafe_fn` / `dropping_copy_types` / `unreachable_pub`）+ rustdoc 1 + clippy 42 — team.md の「rust 4 / 計 47」は数え漏れ、§13 で訂正候補）、`cargo lint`（no-public-fields / checkbox-vocabulary / reap-decision-locality）、`cargo fmt --all --check`。BR4.1 の判定式 `grep -rnE 'enum PlanAction\|pub use .*PlanAction' modules/core/domain/src/orchestration` が 0 件 | CI `check` ジョブ緑 + grep 0 件（Bolt B3 の受入手順に記載） | NFR2, FR8.3, coding-rules/module-visibility, BR4.1 |
| NFR3.1 | **リプレイの決定性** — `from_snapshot(S)` に seq_nr 以降のイベントを順に apply した集約と、同じコマンド列を通常実行した集約は `PartialEq` で同値。apply は純関数で、時刻・乱数・環境を読まない（`occurred_at` は呼出側から受け取る封筒値） | NFR2.2(b) の PBT + ITF 準拠テスト。`core-domain` に `std::time` / 乱数 / `std::env` の利用が無いことをレビューで確認 | NFR3, BR2.3, ADR-001/002 |
| NFR3.2 | **再構成の健全性検査** — 不正なイベント列・スナップショットは panic ではなく Err で拒否する: `ApplyError::SequenceGap{expected, actual}` / `UnknownStage` / `InvariantViolation`、`SnapshotError::InvariantViolation{reason}`（長さ不一致・cursor 範囲外・active が 2 つ以上・gated Completed に承認なし・parked_at ≠ cursor）。U3 はこれを `RepositoryError::Corrupt` に写す（C3） | 各 Err 変種に対する単体テスト（happy path + エラー 2 件以上）が緑。`# Panics` セクションを持つ公開 API が無い | NFR3, BR2.1, BR5.1 / BR5.2, C3 |
| NFR3.3 | **スナップショットの完全性** — `snapshot()` は集約の全状態（intent_id / definition_id / definition_revision / stages / plan / overlay / conditional / checkbox / cursor / status / parked_at / autonomy / approved / revision_count / seq_nr / version — `entities.md` の WorkflowExecution 属性と同じ集合）を含み、`from_snapshot(snapshot())` は元の集約と同値。C6 `snapshot.payload` はこの値オブジェクトの正準 JSON（U3） | PBT: ∀ 到達可能状態 `from_snapshot(agg.snapshot()) == agg`。スナップショットのフィールド集合がエンティティ定義（`entities.md`）と一致 | NFR3, BR5.2 / BR5.3, C6 |
| NFR4.1 | **依存追加なし** — `core-domain` の `[dependencies]` は実測のベースライン（workspace 内部クレート `audit-events` / `directive-schema` / `message-catalog` の 3 つ — `message-catalog` は `orchestration/autonomy_mode.rs` が使用中）から**外部クレートを 1 つも足さない**（serde / serde_json / canon-json を足さない — `DefinitionRevision` の計算はアダプタ層）。dev-dependencies は既存の `proptest` + `serde_json`（ITF 準拠テストが ITF JSON を読むため）のまま。`Cargo.lock` 不変が期待値 | `modules/core/domain/Cargo.toml` の差分レビュー（runtime / dev とも追加 0）、CI `audit` ジョブ緑（影響なし） | NFR4, 前提 P1, BR5.2 |
| NFR4.2 | **`unsafe_code = "forbid"`**（workspace lint、U10 で昇格済み）を維持 | clippy / rustc で violation ゼロ（U10 NFR4.3 の継続） | NFR4 |
| NFR4.3 | **panic しない・境界で Err** — 範囲外の索引は `StageIndex`（`stage_index(usize) -> Option<StageIndex>`）で型保証し、ガード不成立は `CommandError` / `StartError` / `ApplyError` / `SnapshotError` で返す。プロダクトコードで `unwrap` / `expect` / `panic!` / 添字アクセスの範囲外を生まない（`clippy::indexing_slicing` は deny 対象外だが、`StageIndex` 経由に限定） | `# Panics` doc 0 件、`missing_panics_doc` 緑、エラー経路テスト緑 | NFR4, BR5.1, project.md Mandated（unwrap/expect 禁止） |
| NFR4.4 | **ペイロードの取り扱い** — 人間入力は逐語で運び、集約内で加工・切詰め・ログ出力をしない（ドメインにログ基盤は無い）。秘密情報・トークンをペイロードに載せる経路は設けない（`user_input` 等は人間の承認文言であって資格情報ではない — upstream 同等の平文監査行になる） | ペイロード型が `String` / `Option<String>` の素通しで、`Display` 実装が内容を要約しないことをレビューで確認 | NFR4, audit-format.md「No sensitive data」 |
| NFR3.4 | **定義の来歴と同一性** — `Started` に `definition_id`（WorkflowDefinitionId — 内容が変わっても不変のエンティティ ID）と `definition_revision`（内容版）を記録し、`start` は定義の id / revision を無条件に記録する（比較対象の既存状態が無い静的コンストラクタ — `StartError` に DefinitionMismatch は無い）。`next_decision`（Started 適用後に `&WorkflowDefinition` を受け取る唯一のクエリ）は引数の定義の id が `definition_id` と一致しなければ `Err(CommandError::DefinitionMismatch)`。revision の差は Err にせず観測のみ（計画は `Started` で自己完結） | `next_decision` が id 不一致で Err、一致で Ok、revision 差で Ok のテストが緑。`start` は渡した id / revision が Started と集約に写るテストが緑。`from_snapshot` の不変条件に definition_id の存在を含む | NFR3, BR2.6, ADR-008, C4 / C5 改訂 |
| NFR4.5 | **デシリアライズ面を持たない** — 集約は serde に依存せず、外部バイト列を直接受け取らない。parse-don't-validate の境界は U3 のワイヤ構造体（イベント / スナップショットの JSON → ドメイン型）であり、U2 は型で受け取った値にのみ不変条件検証を適用する | `core-domain` の `Cargo.toml` に serde 系が無い（NFR4.1 と同一の証跡）。`from_snapshot` / `apply_event` が不変条件違反を Err にする（NFR3.2） | NFR4, BR5.2, coding-rules/tell-dont-ask（境界での写像） |

## 3. 脅威の検討（STRIDE、ライブラリ規模）

| 区分 | 該当 | 扱い |
|---|---|---|
| Spoofing / Elevation of Privilege | 該当なし（認証・認可を持たない。誰がコマンドを発行したかは U7 / フックの HUMAN_TURN 側の関心） | — |
| Tampering | ジャーナル・スナップショットの改竄や欠落（順序違反・飛び・不変条件違反） | NFR3.2 の健全性検査（`SequenceGap` / `InvariantViolation`）で再水和を拒否する。暗号学的な完全性（署名・HMAC）は要求しない — ローカル単一ユーザのワークスペースで、真実源の SQLite は `.gitignore` 配下（C6 制約 4）。暗号学的完全性が要るなら後続 intent |
| Repudiation | 該当なし（監査行の描画は U4、人間の決定の逐語記録は upstream 契約どおりペイロードで運ぶ — NFR4.4） | 来歴は封筒（intent_id / seq_nr / occurred_at）で追える |
| Information Disclosure | ペイロードの人間入力が平文で監査行に出る | upstream 同等（D6 範囲の互換面）。集約はログ出力を持たないので漏洩経路は U3 / U4 / U7 の I/O 側 |
| Denial of Service | 巨大なイベント列の replay、ジャンプ・recompose の繰返しによる状態爆発 | replay は O(イベント数) の線形で再帰なし。状態は stage_count（実グラフ 33）に比例する固定長ベクタで、イベント 1 件が増やせる状態は高々 1 ステージ分。巨大入力の上限は U3 のスナップショット頻度で抑える（NFR5 非目標のため数値は定めない） |

## 4. データ分類

| データ | 分類 | 扱い |
|---|---|---|
| ドメインイベント（封筒 + ペイロード） | Internal（ワークスペース内 SQLite、git 管理外） | 人間入力を逐語に含む。秘密情報は載せない前提（NFR4.4）。投影後は upstream 互換ファイル（git 管理）へ出る |
| スナップショット | Internal | 全状態の値オブジェクト。秘密情報なし |
| `NextDecision` / `JumpDirection` | Internal | 純粋な判断結果。U6 / U7 が directive へ写す |

## 5. 適用外

- NFR5（性能）: 数値目標なし（requirements.md NFR5 — 定性基準「体感で upstream と同等以上」）。replay は線形で計測しない。
  スナップショット頻度・ジャーナル I/O は U3 の設計事項。
- 観測可能面（CLI 出力・状態ファイル・監査行）の逐語一致・ゴールデンパリティは U4 / U6 / U7 の NFR1 要求であり、U2 では
  NFR1.1〜1.3（契約の維持と語彙の安定）に限定する。

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-08-23T01:13:50Z
**Iteration:** 2（advisory, recovery, unit: u2-domain-es-core）

### Findings

| # | Severity | Location | Finding | Recommendation |
|---|---|---|---|---|
| 1 | Major | security-requirements.md NFR3.4（+ 波及: NFR4.3）、functional-design/rules.md BR2.2 / BR2.6、functional-design/functional-spec.md エラー一覧表 | NFR3.4 の合格基準は「id 不一致で Err、一致で Ok のテストが緑」を **`start` / `next_decision` の両方**に要求するが、`start` は自身より前の集約状態を持たない静的コンストラクタである。実測（`modules/core/domain/src/orchestration/workflow_execution.rs:101`）でも現行 `pub fn start(plan, conditional) -> Result<Self, StartError>` は `&mut self` を取らず新しい `Self` を返す。ADR-008 後の新シグネチャ `start(&definition, scope)`（rules.md BR2.2 logic）も同様に集約を新規構築するだけで、比較対象となる既存の `self.definition_id` は Started 適用前には存在しない。加えて型の面でも矛盾がある — rules.md BR2.6 logic は `DefinitionMismatch` を `CommandError::DefinitionMismatch` として書いており、functional-spec.md のエラー一覧表（157 行目）も `DefinitionMismatch` を `CommandError` の変種として掲載しているが、`start` の戻り値型は `StartError`（BR2.2 の `violation` フィールドで明記）であり、別のエラー enum である。したがって「`start` が `DefinitionMismatch` を返す」という記述は、(a) 比較対象が構造的に存在しない、(b) 返すべきエラー型が現行設計のどの enum にも収まらない、という二重の理由で文字通りには実装できない。NFR3.4 と NFR4.3 はこの矛盾を検証・解消せずそのまま合格基準に取り込んでいる。 | NFR3.4 の記述を「`next_decision`（および Started 適用後に `&WorkflowDefinition` を受け取る以後のクエリ／コマンド、戻り値は `CommandError`）が id 一致を検査する。`start` は self.definition_id / definition_revision を引数の値から無条件に代入するだけで、一致検査は行わない（比較対象がまだ存在しないため）」に訂正する。もし「二重 start 呼び出しの検出」のような別の意図があるなら、それは集約 API ではなく Repository / ユースケース側の責務として別途明記する。functional-design 側（rules.md BR2.6、functional-spec.md エラー一覧表）にも同じ訂正が波及するため、code-generation 着手前に一次資料として反映すること。 |

### iteration 1 所見の解消状況

| iter1 # | Severity | 判定 | 根拠 |
|---|---|---|---|
| 1（依存ベースラインの実測不一致・NFR4.1「追加0」の再定義） | Major | 解消 | `modules/core/domain/Cargo.toml` を実測。`[dependencies]` = `audit-events` / `directive-schema` / `message-catalog`（3 内部クレート）、`[dev-dependencies]` = `proptest` / `serde_json`。NFR4.1 本文の記載と一字一句一致。`message-catalog` が `autonomy_mode.rs:7`（`use message_catalog::bolt as msg;`）で実際に使用されていることも確認 |
| 2（NFR1.1 の ITF 書き換え対象の範囲） | Major | 解消 | `modules/core/domain/tests/engine_loop_conformance.rs` は `core_domain::orchestration::{AutonomyMode, EngineSignal, PlanAction, Status, WorkflowExecution}`（現行 API）を import しており書き換え対象であることを確認。同ディレクトリの `audit_lock_conformance.rs` は `core_domain::workspace::LockProtocol` のみを対象とし `WorkflowExecution` に一切触れていないことを確認 — 「1 ファイルのみ」の主張は正確 |
| 3（NFR3.3 の snapshot 列挙と entities.md の不一致） | Major | 解消 | entities.md `WorkflowExecution` の属性列（intent_id, definition_id, definition_revision, stages, plan, overlay, conditional, checkbox, cursor, status, parked_at, autonomy, approved, revision_count, seq_nr, version — 16 属性）と NFR3.3 の列挙が完全に一致 |
| 4（NFR2.3 のカバレッジ根拠） | Minor | 部分的に解消（許容） | ドメインクレート単独の実測値は依然未取得だが、artifact はこれを正直に明記し（「ドメインクレート単独の実測値は未取得」）、Bolt B3 着手時に `cargo llvm-cov --package core-domain` を 1 回取る具体的なアクションを合格基準に組み込んだ。コードがまだ存在しない NFR 段階では妥当な着地点であり、追加の指摘は不要と判断 |
| 5（NFR2.4 の lint 数 48） | Major | 解消 | `Cargo.toml` `[workspace.lints]` を実測: rust 5（`unsafe_code` / `missing_docs` / `unsafe_op_in_unsafe_fn` / `dropping_copy_types` / `unreachable_pub`）+ rustdoc 1（`broken_intra_doc_links`）+ clippy 42（列挙して実数を確認）= 48。NFR2.4 本文の内訳と完全一致 |

### Validation Tool Results

| Tool / 確認 | 結果 | 解釈 |
|---|---|---|
| `bun .claude/tools/aidlc-sensor-traceability.ts --stage nfr-requirements` | `{"pass":true,"gaps":[],"orphans":[],...,"findings_count":0}` | traceability.json は upstream_ids（NFR1〜5）を過不足なく被覆。NFR3 の target に NFR3.4 が追加されていることも確認 |
| `bun .claude/tools/aidlc-sensor-required-sections.ts`（security-requirements.md） | `{"pass":true,"h2_count":5,...}` | 必須見出し 5 本が揃っている |
| `bun .claude/tools/aidlc-sensor-required-sections.ts`（tech-stack-decisions.md） | `{"pass":true,"h2_count":3,...}` | 必須見出し 3 本が揃っている |
| `cat modules/core/domain/Cargo.toml` | runtime = audit-events / directive-schema / message-catalog、dev = proptest / serde_json | NFR4.1「追加なし」の主張と一致（iter1 #1 の裏取り） |
| `grep -n 'lints' -A 60 Cargo.toml`（[workspace.lints] 集計） | rust 5 + rustdoc 1 + clippy 42 = 48 | NFR2.4 の lint 数と一致（iter1 #5 の裏取り） |
| `cat aidlc/spaces/default/codekb/docs/technology-stack.md` | proptest 1.11.0、Quint 0.32.0、「ドメインは serde 非依存が規約」 | tech-stack-decisions.md / security-requirements.md の技術選定記載と一致 |
| `head -15 modules/core/domain/tests/engine_loop_conformance.rs` / `audit_lock_conformance.rs` | engine_loop 側は旧 API import、audit_lock 側は `LockProtocol` のみ | NFR1.1 の書き換え対象限定の主張を裏付け（iter1 #2 の裏取り） |
| `grep -n 'pub fn start' modules/core/domain/src/orchestration/workflow_execution.rs` | `pub fn start(plan, conditional) -> Result<Self, StartError>`（静的コンストラクタ） | 所見 #1 の根拠 — `start` に比較対象となる既存 `self` が無いことを実装で確認 |
| entities.md `WorkflowExecution` 属性列 ↔ security-requirements.md NFR3.3 | 16 属性が完全一致 | iter1 #3 の裏取り |
| rules.md BR2.2（`violation: StartError`）↔ BR2.6（`Err(CommandError::DefinitionMismatch)`）↔ functional-spec.md エラー一覧表（`DefinitionMismatch` は `CommandError` 変種） | 3 資料間で `start` の戻り値型 `StartError` と `DefinitionMismatch` の所属型 `CommandError` が食い違う | 所見 #1 の根拠 |

### Summary

iteration 1 の Major 所見 4 件・Minor 所見 1 件はすべて実測で解消（Minor 1 件は NFR 段階として妥当な形で着地）を確認した。依存ベースライン・lint 数・snapshot 属性列挙・ITF 書き換え範囲はいずれも実コード（`Cargo.toml`・テストファイル・`entities.md`）と一字一句一致しており、過大主張は見当たらない。新設 NFR3.4（定義の来歴と同一性）は ADR-008 / BR2.6 / C4 / C5 とおおむね整合するが、合格基準が `start` 関数にも id 一致検査を要求している点で、BR2.2（`start` の戻り値型は `StartError`）・functional-spec.md のエラー一覧表（`DefinitionMismatch` は `CommandError` 変種）と矛盾し、`start` には比較対象となる既存状態も無い（Major 所見 #1）。この矛盾は upstream（rules.md BR2.6）由来だが、NFR3.4 がそれをそのまま合格基準として固定しており、code-generation 段階で開発者が実装に詰まって設計者に確認せざるを得ない具体的な箇所である。advisory 基準（Critical 0 かつ Major ≤ 2 なら READY）に照らすと Major 1 件のみのため READY と判定するが、この 1 件は code-generation 着手前に安価に訂正できる（NFR3.4 の文言修正 + rules.md BR2.6 の同期）ため、人間の承認ゲートで訂正を求めることを推奨する。
