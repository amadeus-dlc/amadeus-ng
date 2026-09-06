# security-requirements — U2 ドメイン ES コア（`u2-domain-es-core`）

> NFR Requirements（Construction 3.2）成果物（Unit: U2、kind: library）。**2026-09-07 再走（Modify）** — 2026-08-23 の初版を、
> 2026-09-05 是正・2026-09-07 再走後の機能設計と現行コード・現行 CI へ同期した（質問票 P7〜P12、Looks correct）。
> 旧世代（`WorkflowExecution`・12 変種・snapshot 値オブジェクト・「panic しない」・`DefinitionMismatch`）の記述は失効し、
> 旧 READY レビュー節は `security-requirements-review-history-2026-08-23.md` へ退避した。
>
> 出典: `../functional-design/functional-spec.md`（§2 API、W1〜W7、§4〜§5、§9 引継ぎ、末尾の 2026-09-06 レビュー所見 R-01〜R-10）、
> `../functional-design/rules.md`（BR1.0〜BR1.9 受理条件 / BR2.1〜BR2.6 イベント・再生・ID 参照 / BR3.x next_decision /
> BR4.x PlanAction / BR5.1〜BR5.5 型・永続化中立・読取版・規則・FCC）、`../functional-design/entities.md`（16 変種、FCC 型）、
> `../../../inception/requirements-analysis/requirements.md`（NFR1〜NFR5、FR1.3 / FR2.1 / FR3.1 / FR3.3 / FR8.3 / FR8.4）、
> `../../../inception/contract-design/contract-summary.md`（C3 / C5 / C6 — 2026-08-30 の全再生追記は 2026-09-05 裁定で上書き済み）、
> 実コード `modules/core/command/domain/`（`Cargo.toml`、`src/orchestration/`、`tests/engine_loop_conformance.rs`、
> `tests/collection_contract_test.rs`）、`formal/orchestration/engine_loop.qnt`（v2.7）、U10 の CI 実測
> （`rust-toolchain.toml` 1.95.0、workspace lints 50、`scripts/coverage.sh` 90 床 / TOLERANCE 0.01 / `PROPTEST_RNG_SEED=20260823`）、
> `aidlc/spaces/default/memory/team.md`（Testing Posture: TDD・3 層品質保証・カバレッジ 90% 床）、
> `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/`。
>
> 各要求は Inception の NFR ID を継承し枝番を付ける（NFR1.x / NFR2.x / NFR3.x / NFR4.x）。NFR5 は非目標として §5 に置く。

## 1. 範囲と信頼境界

- U2 は `core-command-domain` クレート（`modules/core/command/domain`）内の **純粋な集約** — `Intent`（依頼・定義参照・静的計画）と
  `IntentExecution`（実行 FSM）、そのドメインイベント（`IntentEvent` 1 変種 / `IntentExecutionEvent` 16 変種）、位置型 `StageIndex`、
  BR5.5 で新設する FCC（StageEntries / StageSlots / StageIndexSet / ArtifactPaths / StageSlugSet / PromotedSections / RuleLines /
  TransitionSteps ほか）。I/O なし・同期・serde なし（BR5.2）。ネットワーク・認証・認可・永続化・ログ出力を持たない。
- 入力は (a) `WorkflowDefinition`（`Intent::create` と `Intent::resolve_review_policy` が受ける。U3 の Repository が復号した集約 —
  信頼境界の内側。実行は `intent_id`、Intent は `definition_id` で ID 参照する — BR2.6）、(b) ユースケース（U5 / U6）からのコマンド引数
  （StartRequest / user_input / feedback / reason / ArtifactPaths / target / StageIndexSet / mode / HumanTurns / PracticesPromotion）、
  (c) 再構成時の最新スナップショット（DTO → `IntentExecution::new`）と差分イベント列（U3 が SQLite から復号し封筒・通番を検査したもの）。
- 出力は (a) ドメインイベント（U3 がジャーナルへ、U4 が投影へ）、(b) 集約自身（U3 が DTO へ写してスナップショット行へ）、
  (c) `NextDecision` / `JumpDirection` / `ReportDecision`（RMU が投影し、U6 / U7 が directive に写す）。JSON 化・監査行の描画は
  U3 / U4 の責務で、U2 の境界にバイト列は現れない。
- 秘密情報・鍵は扱わない。イベントペイロードの人間入力（`request` / `user_input` / `feedback` / `reason` / 成果物パス / 規則行）は
  upstream が監査行に逐語記録する（`audit-format.md`「Human decisions recorded verbatim」）ものと同じで、集約は内容を解釈・検証・加工せず運ぶ。

## 2. 要求

| ID | 要求 | 合格基準 | 出典 |
|---|---|---|---|
| NFR1.1 | **engine_loop 契約の維持** — Quint `engine_loop.qnt`（v2.7）の ITF トレースを decide → apply 経路で再生し、BR2.5 の射影表（rules.md 第 3 節）で状態・会計・指令を突き合わせる。対象は `modules/core/command/domain/tests/engine_loop_conformance.rs` **1 本**。U2 の再走（FCC 化・`next_decision` の Result 化）ではモデルを変更せず、射影は `StageSlots.at` で同じ観測を読む（functional-spec §9） | `engine_loop_conformance.rs` が改修後の API で全緑（既存 1 テスト、全トレース）。Quint ゲート（`scripts/quint-gate.sh`）は不変。R-02 の追随（同テストの `stages()` / `stage_keys()` / `next_decision` 呼出の書換え）を同じ Bolt で完了 | NFR1, BR2.5, team.md Testing Posture（ITF は TDD の外側の受け入れゲート） |
| NFR1.2 | **ゲート判定の upstream 一致** — `gated = phase ≠ initialization`（`StageEntry::is_gated` / `StageKey::is_gated`）。誕生（`From<(Started, occurred_at)>`）が initialization 全段を Completed（approved = false）にし、最初の実効対象の実ステージを InProgress にする。初期化完了用のコマンド・イベントは存在しない（BR1.3） | 実グラフ由来の計画で (a) initialization 3 段が誕生時 Completed、(b) 索引 3 以降が gated、(c) jump の target に initialization を指定すると InvalidTarget、のテストが緑（現行テスト群を維持） | NFR1, BR1.3 / BR1.6, orchestration-next-ladder |
| NFR1.3 | **イベント語彙の契約安定性** — `IntentExecutionEvent` は 16 変種（`StageCompleted` は廃止）。ペイロードは entities.md の payloads が正本（列は FCC: Started.stages = StageEntries、GateOpened.artifacts = ArtifactPaths、Recomposed = StageSlugSet、PracticesAffirmed = PromotedSections / RuleLines）。通番・時刻・`schema_version`・直列化はアダプタ封筒の責務（BR2.1 / BR2.4）。共有 C5 の旧 12 変種・旧封筒は再導入しない | 変種の網羅 match で 16 をコンパイル時固定。保存 DTO（command interface-adapter）と RMU 専用 DTO の対応は横断適合テストで固定（U3 / U4 側、BR2.4）。FCC 化で DTO 境界の要素列挙が `fold_left` に置き換わっても DTO のバイト表現（正準 JSON）は不変 — ゴールデン・往復テストが緑 | NFR1, BR2.1 / BR2.4, C5（後続裁定と同期） |
| NFR2.1 | **TDD** — レイヤーごとに red → green → refactor（失敗するテストを先に書く）。ITF 準拠テスト・ゴールデン・契約試験ハーネスは TDD の外側の受け入れゲートとして維持し red を代替しない | U2 code-generation 再走の PR でテスト先行のコミット列（または 1 コミット内で `tests` → `src` の順）が確認でき、`cargo test --workspace` 全緑 | NFR2, team.md Testing Posture（Methodology tdd） |
| NFR2.2 | **決定的 PBT** — proptest を `PROPTEST_RNG_SEED=20260823` 固定下で実行し、任意のコマンド列について (a) decide の事後状態 == 旧状態に返された同じイベント・通番・時刻を `apply_event` した状態（BR1.1 事後条件）、(b) `replay(snapshot, delta)` == 同じコマンド列を通常実行した状態（version と新規イベント ID を除いて同値 — BR2.3）、(c) 通番は常に current+1 で進み、飛び・逆行は panic（`#[should_panic]` で固定）、(d) Quint 不変条件（cursor_in_scope / at_most_one_active / gate_lifecycle / parked_position / unpark_restores_position / review_attempt_floor / practices_receipt_floor）が任意列で保たれる、(e) Err は状態不変（BR1.1）を固定する | 5 性質の proptest が CI で緑。同一コードの 2 回計測でテスト結果・カバレッジ行が一致（シード固定、U10 NFR2.4 で差 0.00 を実証済み） | NFR2, BR1.1 / BR1.2 / BR2.1 / BR2.3, team.md（PBT は集約本体同居） |
| NFR2.3 | **カバレッジ** — ワークスペース絶対 90% 床 + PR 相対ゲート（TOLERANCE 0.01）を維持する（直近実測 99.14%、U10）。ドメインクレート単独の基準値は **2026-09-06 実測: 行 98.66% / 関数 98.20% / リージョン 98.69%**（`cargo llvm-cov --package core-command-domain`、seed 固定）。除外設定は composition root（`modules/app/aidlc/src/main.rs`）のみで、U2 のコードに除外を足さない | `scripts/coverage.sh` が `[PASS] absolute gate` + 相対ゲート緑。FCC 化後のドメインクレート単独の行カバレッジが 98.66% を下回らない（code-generation の受入手順に同コマンドで再計測を含める） | NFR2, team.md（カバレッジ）, U10 NFR2.4 |
| NFR2.4 | **規則の機械強制を緑で通す** — `cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -D warnings`（`[workspace.lints]` 実測 50 ルール deny = rust 5 + rustdoc 1 + clippy 44。旧 48 は失効）、`cargo lint`（coding-rules 6 規則の機械強制分）。BR4.1 の判定式は現行配置 `modules/core/command/domain/src/orchestration` に対する `rg -n 'enum PlanAction\|pub use .*PlanAction'` 0 件（不在・検索エラーは失敗） | CI `check` ジョブ緑 + 判定式 0 件（code-generation の受入手順に記載） | NFR2, FR8.3, coding-rules/module-visibility, BR4.1 |
| NFR2.5 | **FCC の契約試験（新規）** — BR5.5 で新設する FCC を既存の契約試験ハーネス `modules/core/command/domain/tests/collection_contract_test.rs` に登録し、共通契約（`len` / `at` / `fold_left` / `filter`）を検査する。集合型（StageIndexSet / StageSlugSet）は `combine` / `divide` について Monoid 則（結合法則・左右単位元・冪等・交換）と差集合則（`A \ A = empty`、`A \ empty = A`）を性質試験で固定する。文書順の列（StageEntries / StageSlots）は `combine` の連結順序と slug 衝突の Result 拒否、`map` の衝突拒否を固定する。使われない共通メソッドを機械的に足さない。リードモデル側（read-model-updater / クエリ側）は FCC を使わない | 契約試験ハーネスが新設型を含めて緑。性質試験が seed 固定で緑。レビューで「生の Vec / スライス公開 0 件（DTO 境界の理由付き例外を除く）」「read-model-updater に FCC 参照 0 件」を確認 | NFR2, BR5.5, coding-rules/first-class-collections.md（2026-09-06）, 質問票 Q4 / Q4a |
| NFR3.1 | **再生の決定性** — `apply_event` は純関数で、時刻・乱数・環境を読まない（`occurred_at` は封筒値として引数で受ける）。時計・乱数の利用はコマンド内のイベント ID 採番（`IntentExecutionEventId::generate` = UUIDv7、オーナー裁定 2026-09-02 の例外）に限る。決定性は BR1.1 の事後条件と BR2.3 の差分再生で定義し、別実行同士のイベント ID 同値を前提にしない | NFR2.2(a)(b) の PBT + ITF 準拠テスト。`core-command-domain` の `src/` で `std::time` / `std::env` / 乱数の利用が `*EventId::generate` 以外に無いことをレビューで確認 | NFR3, BR1.1 / BR2.3, ADR-001 / 002 |
| NFR3.2 | **失敗境界の二層** — (1) DTO → 集約基底の検査付き変換 `IntentExecution::new` は不変条件違反を `IntentExecutionError` の Err で返し、Repository の封筒・通番・aggregate_id 検査の不整合とともに `RepositoryError::Corrupt` へ写す（C3）。(2) 型変換後の壊れた歴史（通番の飛び・未知ステージ・不変条件違反）は `replay` / `apply_event` が回復せず panic する（オーナー裁定 2026-08-30、BR2.1 / BR5.2）。回復用の公開 `ApplyError` は置かない | `IntentExecutionError` の各変種に単体テスト（正常系 + 異常系 2 件以上）。壊れた歴史は `#[should_panic]` テストで固定。`replay` / `apply_event` に `# Panics` ドキュメントがあり `missing_panics_doc` 緑 | NFR3, BR2.1 / BR5.2, C3 |
| NFR3.3 | **スナップショットの完全性** — スナップショットは通番時点の `IntentExecution` 自身（ドメインに双子の memento 型は無い）。アダプタの DTO が全状態（id / intent_id / slots の各位置の記録 / cursor / status / parked_at / autonomy / skeleton_stance / last_gate_resolution_at / seq_nr / last_updated_at — entities.md の属性集合。version は封筒側）を写し、`new` を通した復元が元の集約と同値になる。FCC 化で slots が 1 列になっても DTO の列表現は変えない | PBT: ∀ 到達可能状態 `to_domain(to_dto(agg)) == agg`（横断適合テスト、U3 側で所有）。`new` の不変条件（cursor 範囲・active ≤ 1・gated Completed に承認・parked_at = cursor）が entities.md と一致 | NFR3, BR5.2 / BR5.3, C6 |
| NFR3.4 | **集約参照の照合** — `Intent` は `definition_id` / `definition_revision` を来歴として持ち（Created）、`IntentExecution` は `intent_id` のみを持つ。`&Intent` を受ける全コマンド・書込前ガード・`next_decision` は ID 不一致を `IntentMismatch` の Err で拒否する（Q5 = A、現行の `next_decision` は code-generation で Result 化）。定義 ID の照合は `Intent::resolve_review_policy(&WorkflowDefinition, ..)` が担い、不一致は `IntentReviewError::DefinitionMismatch`（現行実装名。質問票 P10 の括弧書き「LineageMismatch」は誤記で、本表の実装名が正）。revision の差は Err にせず来歴として観測のみ | `next_decision` が ID 不一致で Err・一致で Ok のテストが緑（新規）。既存の各コマンドの IntentMismatch テスト（`intent_execution.rs` のテストモジュール）が緑。`resolve_review_policy` の `DefinitionMismatch` テストが緑 | NFR3, BR2.6, ADR-008, 質問票 Q5 |
| NFR4.1 | **依存追加なし（再ベースライン）** — `core-command-domain` の runtime 依存は実測ベースライン `chrono` / `uuid`（v7）/ `core-infrastructure`（言語拡張: canon_json と collections）、dev は `proptest` / `serde_json`（ITF JSON 読取のみ）。FCC 化・`next_decision` 改修で**外部クレートを 1 つも足さない**。serde derive・ストア trait はドメインに置かない（domain-persistence-neutrality の機械強制 = この `Cargo.toml` の不在） | `modules/core/command/domain/Cargo.toml` の差分レビュー（runtime / dev とも追加 0）、`Cargo.lock` 不変、CI `audit` ジョブ緑（影響なし） | NFR4, 質問票 P7, BR5.2 |
| NFR4.2 | **`unsafe_code = "forbid"`**（workspace lint、U10 で昇格・実証済み）を維持 | clippy / rustc で violation ゼロ（U10 NFR4.3 の継続） | NFR4 |
| NFR4.3 | **panic の方針** — プロダクトコードで `unwrap` / `expect` を使わない（project.md Mandated）。公開位置は `StageIndex` で型保証し、コマンド入口の不正位置・他実行の位置は `CommandError` の Err で拒否する。panic はオーナー裁定の射程（apply の内部検査違反 = 壊れた歴史）に限り、該当 API に `# Panics` を明記する。添字アクセスは FCC の `at`（Option）を通す | `missing_panics_doc` 緑、`# Panics` を持つ公開 API が `replay` / `apply_event` に限られること、エラー経路テスト緑 | NFR4, BR5.1 / BR5.5, project.md Mandated（unwrap/expect 禁止） |
| NFR4.4 | **ペイロードの取り扱い** — 人間入力は逐語で運び、集約内で加工・切詰め・ログ出力をしない（ドメインにログ基盤は無い）。秘密情報・トークンをペイロードに載せる経路は設けない（`user_input` 等は人間の承認文言であって資格情報ではない）。FCC（ArtifactPaths / RuleLines）も要素を素通しで保持し、順序と重複規則以外の加工をしない | ペイロード型が `String` / `Option<String>` / 文字列 FCC の素通しで、`Display` 実装が内容を要約しないことをレビューで確認 | NFR4, audit-format.md「No sensitive data」 |
| NFR4.5 | **デシリアライズ面を持たない** — 集約は serde に依存せず、外部バイト列を直接受け取らない。parse-don't-validate の境界は U3 の DTO（イベント / スナップショットの JSON → ドメイン型）であり、U2 は型で受け取った値にのみ不変条件検査（`new`、FCC の構築検査）を適用する | `core-command-domain` の `Cargo.toml` に serde 系が無い（NFR4.1 と同一の証跡）。`new` と FCC の構築が不変条件違反を Err にする（NFR3.2 (1)） | NFR4, BR5.2 / BR5.5, coding-rules/tell-dont-ask（境界での写像） |

## 3. 脅威の検討（STRIDE、ライブラリ規模）

| 区分 | 該当 | 扱い |
|---|---|---|
| Spoofing / Elevation of Privilege | 該当なし（認証・認可を持たない。誰がコマンドを発行したかは U7 / フックの HUMAN_TURN 側の関心。人間の操作の確認 I11 は HumanTurns を材料に集約が判断する — BR1.8） | — |
| Tampering | ジャーナル・スナップショットの改竄や欠落 | 復号・封筒・通番・aggregate_id の不整合は Repository が `Corrupt` で拒否（NFR3.2 (1)）。型変換後の壊れた歴史は panic で停止し、壊れた状態で進まない（NFR3.2 (2)）。暗号学的完全性（署名・HMAC）は要求しない — ローカル単一ユーザのワークスペースで、真実源の SQLite は git 管理外。要るなら後続 intent |
| Repudiation | 該当なし（監査行の描画は U4、人間の決定の逐語記録は upstream 契約どおりペイロードで運ぶ — NFR4.4） | 来歴はイベント固有 ID（UUIDv7）と封筒（aggregate_id / seq_nr / occurred_at）で追える |
| Information Disclosure | ペイロードの人間入力が平文で監査行に出る | upstream 同等（D6 範囲の互換面）。集約はログ出力を持たないので漏洩経路は U3 / U4 / U7 の I/O 側 |
| Denial of Service | 巨大なイベント列の再生、jump・recompose の繰返しによる状態爆発 | 再構成は最新スナップショット + 差分で、差分は O(イベント数) の線形・再帰なし（BR2.3）。状態は stage_count（実グラフ 33）に比例する固定長の StageSlots で、イベント 1 件が増やせる状態は高々 1 位置分。StageIndexSet の集合演算は O(stage_count)。巨大入力の上限は U3 のスナップショット頻度で抑える（NFR5 非目標のため数値は定めない） |

## 4. データ分類

| データ | 分類 | 扱い |
|---|---|---|
| ドメインイベント（アダプタ封筒 + ペイロード） | Internal（ワークスペース内 SQLite、git 管理外） | 人間入力を逐語に含む。秘密情報は載せない前提（NFR4.4）。投影後は upstream 互換ファイル（git 管理）へ出る |
| スナップショット（集約自身の DTO 写し） | Internal | 全状態。秘密情報なし |
| `NextDecision` / `JumpDirection` / `ReportDecision` | Internal | 純粋な判断結果。RMU が投影し U6 / U7 が directive へ写す |

## 5. 適用外

- NFR5（性能）: 数値目標なし（requirements.md NFR5 — 定性基準「体感で upstream と同等以上」）。再生は線形で計測しない。
  スナップショット頻度・ジャーナル I/O は U3 の設計事項。
- 観測可能面（CLI 出力・状態ファイル・監査行）の逐語一致・ゴールデンパリティは U4 / U6 / U7 の NFR1 要求であり、U2 では
  NFR1.1〜1.3（契約の維持と語彙の安定）に限定する。
- FCC の `combine` / `divide` / `map` を共通 trait `FirstClassCollection` へ盛り込む改修（オーナーの最終方針、質問票 Q4a）は
  本 Unit の NFR に含めず、積み残しとして機能設計 §9 に記録済み。

## Review

**Verdict:** NOT-READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-06T16:03:44Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action |
|---|---|---|---|---|
| R-01 | Major | `construction/u2-domain-es-core/nfr-requirements/security-requirements.md` > 第 2 節 NFR2.5 の合格基準「read-model-updater に FCC 参照 0 件」 | この合格基準は現行コードでも、U2 が命じる改修後でも成立しない。実測: `modules/core/read-model-updater/src/` の 10 ファイルが FCC 型（`AuditFields` / `Checkboxes` / `OrderedAuditEvents`）を参照する（`workspace/projection.rs:31,472`、`workspace/audit_block.rs:29`、`workspace/audit_shard.rs:72`）。さらに BR5.5 が `Intent.stages` を StageEntries へ FCC 化するのに対し、RMU の生産コード `read_tables.rs:239` が `intent.stages().iter().enumerate()`、同 `:284` が `.stage_keys()` を読むため、改修後は RMU が新設 FCC を触らざるを得ない。機能設計レビュー R-06（Minor、未解決）が同じ矛盾を BR5.5 violation に対して既に指摘しており、本 NFR はそれを機械判定の合格基準へ格上げしてしまっている。正本 `coding-rules/first-class-collections.md`「適用例」も投影側が `find` / `has_completed` / `OrderedAuditEvents.fold_left` を使うと明記する | 合格基準を「BR5.5 で新設するコマンド側 FCC（StageEntries / StageSlots / StageIndexSet / ArtifactPaths / StageSlugSet ほか）を RMU の**型として**保持・構築しない」等、実測可能な射程へ限定して書き直す。`workspace` 文脈の既存 FCC と、改修後 RMU が読取専用で通過させる操作を明示的に除外する |
| R-02 | Major | `.../nfr-requirements/security-requirements.md` > 第 2 節 NFR1.1 の合格基準「R-02 の追随（同テストの `stages()` / `stage_keys()` / `next_decision` 呼出の書換え）」 | 引き継いだと明記しながら、R-02 が実測で名指しした追随対象の半分が落ちている。R-02 は `core-command-use-case` の**生産コード** `commit_verdict_use_case.rs:212,218`（`steps.contains(&TransitionStep::Approve)` と `apply_report(.., &steps, ..)`）および `test_support.rs:114,856,889`（`original.stages().to_vec()`）を挙げ、「この 2 か所の欠落はそのまま CI 赤になる」と警告している。実測で該当箇所の存在を確認した。本成果物の全文に `core-command-use-case` の語は 1 件も現れない。NFR2.1 の `cargo test --workspace` 全緑が結果的に検出はするが、Bolt の作業範囲見積りが誤ったまま着手される | NFR1.1 の合格基準に `core-command-use-case`（`commit_verdict_use_case.rs` / `test_support.rs`）を追随対象として明記し、R-02 の要求アクション全文と一致させる。または NFR1.1 は ITF テストに限る旨を書き、残りを別 NFR として立てる |
| R-03 | Major | `.../nfr-requirements/security-requirements.md` > 第 2 節 NFR3.4（`next_decision` の Result 化）および第 1 節「範囲と信頼境界」 | NFR3.4 は `next_decision` を Result 化すると定めるが、合格基準はドメインクレート内のテストだけを挙げる。実測では `modules/core/read-model-updater/src/read_tables/next_answer_row.rs:58` が `execution.next_decision(intent, &kind.to_request())` を**生産コード**で呼んでおり、Result 化はこの呼出をコンパイルエラーにする。この呼出元は R-02 にも本成果物にも現れない。加えて第 1 節は U2 を `core-command-domain` 内の集約に限ると宣言する一方、NFR1.1 / NFR3.4 が命じる公開 API 変更の影響は 3 つの兄弟クレートに及ぶ。機能設計レビュー R-10（Info、未解決）が同じ境界矛盾を指摘しており、実装者は「U2 の Bolt が RMU を改修してよいか」を設計者に問い直さざるを得ない | `next_answer_row.rs` を含む全呼出元を NFR3.4 の合格基準に列挙する。あわせて第 1 節に、U2 の Bolt が兄弟クレート（use-case / interface-adapter / read-model-updater）の追随改修を含むか否かの裁定を書く（R-10 の未解決を引き継ぐ場合はその旨と裁定先を明記する） |
| R-04 | Minor | `.../nfr-requirements/security-requirements.md` > 第 2 節 NFR2.4 の判定式 `rg -n 'enum PlanAction\|pub use .*PlanAction'` | 生ファイル上の `\|` は Rust 正規表現ではリテラルのパイプであり、選択肢にならない。実測: `PlanAction` が定義されている `modules/core/command/domain/src/workflow_definition` に対してこの式を流しても一致 0 件（真の選択肢 `|` では 2 件一致）。すなわちこの判定式は**常に 0 件**を返す空振りのゲートで、BR4.1 違反を検出できない。表セル内のエスケープが原因だが、成果物の生バイトを転記する実装者は空振り版を得る | 判定式をコードブロックで示すか、`rg -n -e 'enum PlanAction' -e 'pub use .*PlanAction'` のようにパイプを含まない形へ書き換える。合格判定に「同じ式を `workflow_definition` へ流すと 1 件以上一致する」という検出力の裏取りを添える |
| R-05 | Minor | `.../nfr-requirements/security-requirements.md` > 第 1 節「BR5.5 で新設する FCC（… TransitionSteps ほか）」および NFR2.5 の合格基準「契約試験ハーネスが新設型を含めて緑」 | 検収対象の型集合が確定しない。第 1 節は末尾を「ほか」で閉じ、機能設計レビュー R-01（Major、未解決）は TransitionSteps / ReviewAttempt の pending・closed / PromotedSections / RuleLines の 4 系統に不変条件・操作・結果型の定義が無いと指摘している。`tech-stack-decisions.md` §3 は R-01 を未決として引き継ぐと書くのみで、本成果物の合格基準は未決の型を含んだまま「新設型を含めて緑」と判定を求める | 第 1 節の「ほか」を排し、契約試験ハーネスへ登録する型を確定列挙する。R-01 が未決の 4 系統は、定義確定を NFR2.5 の**前提条件**として明記する（定義未確定のまま検収に入らない） |
| R-06 | Minor | `.../nfr-requirements/security-requirements.md` > 第 2 節 NFR2.5 の要求文（`combine` / `map` の固定）と同末尾「使われない共通メソッドを機械的に足さない」 | 同一セル内で相反する指示になっている。前段は文書順の列（StageEntries / StageSlots）に `combine` の連結順序と `map` の衝突拒否の固定を課すが、後段と `coding-rules/first-class-collections.md`「検証と適用」は未使用の共通メソッド追加を禁じる。機能設計レビュー R-07（Minor、未解決）が「設計本文が用途を示しているのは StageIndexSet の `combine` / `divide` だけ」と指摘済み。実装者は、用途の無い `combine` を試験のためだけに実装するか、要求を満たさないかの二択に置かれる | 型ごとに `combine` / `divide` / `map` の**業務上の用途**を持つものだけを列挙し、その型に限って契約試験を課す。用途の無い型については試験対象外である旨を明記する |
| R-07 | Minor | `.../nfr-requirements/security-requirements.md` > 第 2 節 NFR2.3「ドメインクレート単独の基準値 … 行 98.66%」 | 基準値の測定範囲が U2 の範囲と一致しない。`cargo llvm-cov --package core-command-domain` は同クレートの 3 文脈すべてを測る。実測行数は orchestration 14,112 行 / workflow_definition 8,205 行 / workspace 4,243 行で、U2 の対象である orchestration はクレート全体の約 53% にすぎない。他 2 文脈が希釈するため、FCC 化で orchestration のカバレッジが数 pp 下がっても 98.66% の床は素通りしうる | 合格基準に orchestration 配下に絞った計測（`--ignore-filename-regex` で他 2 文脈を除く等）を加えるか、クレート全体値である旨と希釈の限界を明記して、U2 の退行検出は NFR2.2 の性質試験が担うと書く |
| R-08 | Info | `.../nfr-requirements/security-requirements.md` > 第 5 節「適用外」および `traceability.json` の NFR3 = OK | 上流 `requirements.md` NFR3 の合格基準は「改訂版 `audit_lock.qnt` の ITF 準拠 + クラッシュ再構成（ジャーナル → 集約 → 投影）テスト」だが、本成果物の NFR3.1〜3.4 はいずれもそれを検収しない。NFR3.3 の合格基準に「横断適合テスト、U3 側で所有」と一言あるのみで、第 5 節「適用外」は audit_lock の ITF とクラッシュ再構成に触れない。traceability は NFR3 を OK としており、読み手には上流の検収先が見えない | 第 5 節に「audit_lock.qnt の ITF 準拠とクラッシュ再構成テストは U3 / U4 の検収」を明記する（質問票 P4 の確認事項を本文へ昇格させる） |

### Validation Tool Results

| 検査 | 結果 | 解釈 |
|---|---|---|
| `aidlc-sensor-required-sections.ts --stage nfr-requirements`（security-requirements.md） | pass、H2 5 本、所見 0 | 必須見出しは充足。`validation-20260907.md` の記録と一致 |
| `aidlc-sensor-required-sections.ts --stage nfr-requirements`（tech-stack-decisions.md） | pass、H2 3 本、所見 0 | 同上 |
| `aidlc-sensor-traceability.ts --stage nfr-requirements` | pass、gaps / orphans / missing / invalid すべて空 | NFR1〜NFR5 の被覆と枝番は構造的に整合。R-08 は構造ではなく検収先の記載漏れであり、センサーの守備範囲外 |
| 依存ベースライン（`modules/core/command/domain/Cargo.toml`） | runtime = chrono / uuid(v7) / core-infrastructure、dev = proptest / serde_json | NFR4.1 / NFR4.5 の記載どおり。serde 系の不在も確認 |
| workspace lints 実測（`Cargo.toml`） | rust 5 + rustdoc 1 + clippy 44 = 50 | NFR2.4 の「実測 50」は正確（旧 48 の失効も正しい） |
| `rust-toolchain.toml` / `scripts/coverage.sh` | channel 1.95.0、ABSOLUTE_THRESHOLD 90.0、TOLERANCE 0.01、PROPTEST_RNG_SEED 20260823、除外は `modules/app/aidlc/src/main.rs` のみ | NFR2.3 の記載と完全一致 |
| イベント変種の実測（`src/orchestration/intent_execution_event/`） | 16 ファイル | NFR1.3 の「16 変種」は正確 |
| 時計・乱数の利用箇所（`src/` の grep） | `Uuid::now_v7` は 4 つの `*EventId::generate` のみ。`std::time` / `std::env` / `rand` の利用なし | NFR3.1 の主張を裏付ける |
| `next_decision` の現行署名（`intent_execution.rs:1897`） | `pub fn next_decision(&self, intent: &Intent, request: &NextRequest) -> NextDecision` | NFR3.4 の「現行は Result ではない」は正確。ただし呼出元の列挙が不足（R-03） |
| `resolve_review_policy` の不一致エラー名（`intent.rs:68-75`） | `IntentReviewError::DefinitionMismatch` | NFR3.4 の実装名は正しく、質問票 P10 の「LineageMismatch」が誤記という注記も正しい |
| `ApplyError` の可視性（`apply_error.rs:15`） | `pub(crate) enum ApplyError` | NFR3.2「回復用の公開 `ApplyError` は置かない」は現行実装と整合 |
| Quint 不変条件名（`formal/orchestration/engine_loop.qnt`、v2.7） | NFR2.2(d) の 7 名すべて実在 | モデル版・不変条件名ともに正確 |
| BR4.1 判定式の検出力（`rg` を `workflow_definition` へ流す） | 生バイトの `\|` 版は 0 件、選択肢 `|` 版は 2 件 | 判定式が空振りであることの実証（R-04） |
| RMU の FCC 参照・集約参照（`modules/core/read-model-updater/src/`） | FCC 型を参照するファイル 10 件、`intent.stages()` / `stage_keys()` / `next_decision` の生産コード呼出を確認 | NFR2.5 の合格基準が成立しないことの実証（R-01）、および Result 化の未列挙呼出元（R-03） |

### Summary

実測との一致という点では本成果物は非常に精度が高い。依存・lints 50・toolchain 1.95.0・カバレッジ床とシード・16 変種・`now_v7` の閉じ込め・`next_decision` の現行署名・`DefinitionMismatch` の実装名・`ApplyError` の `pub(crate)` まで、確認したすべてが現行コードと合致し、旧世代 NFR の失効も正しく反映されている。センサー 3 本も緑である。

NOT-READY とする理由は精度ではなく**射程**にある。第 1 節は U2 を `core-command-domain` 内の集約に限ると宣言する一方、NFR1.1 / NFR2.5 / NFR3.4 が命じる変更（FCC 化・`next_decision` の Result 化）は 3 つの兄弟クレートの生産コードを壊す。実装者はコンパイルエラーに突き当たった時点で「U2 の Bolt が RMU / use-case を改修してよいか」を設計者に問い直すことになり、これは READY の基準（設計者への追加照会なしに実装できる）を満たさない。R-01 の合格基準はさらに、その改修後も成立し得ない条件を機械判定として課している。いずれも機能設計の未解決所見（R-02 / R-06 / R-10）に根を持つため、是正は本成果物の文言修正と、凍結中の機能設計への折り戻し裁定の両方を要する。
