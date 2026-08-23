# security-requirements — U3 SQLite EventStore と WorkflowExecutionRepository（`u3-event-store-repository`）

> NFR Requirements（Construction 3.2）成果物（Unit: U3、kind: library、Bolt: B5）。出典: `../functional-design/functional-spec.md`（§3 フロー、§4 ワイヤ、§5 モデル、
> §7 テスト）、`../functional-design/rules.md`（BR1.1〜BR5.2）、`../../../inception/requirements-analysis/requirements.md`（NFR1〜NFR5、FR1.2 / FR1.3）、
> `../../../inception/contract-design/contract-summary.md`（C3 / C6）、`aidlc/spaces/default/codekb/docs/technology-stack.md`、`aidlc/spaces/default/memory/team.md`
> （Testing Posture / CI ゲート / サプライチェーン裁定）、確認事項 `nfr-requirements-questions.md`（P1〜P6、Looks correct）。
>
> 各要求は Inception の NFR ID を継承し枝番を付ける（NFR1.x〜NFR4.x）。NFR5 は非目標として §5 に置く。

## 1. 範囲と信頼境界

- U3 は `core-use-case`（ポート 3 本 + エラー + 値 2 型）と `core-interface-adapter`（SQLite ストア・ワイヤ・Repository 実装・InMemory）、`core-domain` の是正 2 型と改名、
  `formal/orchestration/journal_protocol.qnt`、ロック系の退役。入力は (a) ユースケース（U5 / U6）からの `store(event, aggregate)` / `find_by_id(id)`、(b) ディスク上の
  SQLite ファイル（`aidlc/spaces/<space>/intents/.aidlc-store.sqlite` — `.gitignore` 配下、ローカル単一ユーザ）、(c) composition root から注入されるパスと Clock。
  出力は (a) 再構成した集約（ドメイン型）、(b) ジャーナル / スナップショット / チェックポイントの行、(c) エラー（材料のみ）。
- 信頼境界: ストアファイルは**信頼しない入力**として扱う（復号は parse-don't-validate、from_state / apply_event の不変条件検査が最終防衛線）。ユースケースから受ける
  ドメイン型は信頼境界の内側（Always Valid）。
- 秘密情報・資格情報・ネットワークを扱わない。環境変数を読まない（パスは注入）。ログ出力を持たない（P6）。

## 2. 要求

| ID | 要求 | 合格基準 | 出典 |
|---|---|---|---|
| NFR1.1 | **逸脱の確定** — ストア追加（`aidlc/spaces/<space>/intents/.aidlc-store.sqlite`、git 管理外）とロック dir 非生成は `deviations.md` # 4 のとおり。既存の upstream 互換ファイル（`aidlc-state.md` / 監査シャード / `intents.json`）の形式・内容には触れない（投影は U4） | `deviations.md` # 4 のパスが確定値に更新され「相当」が消える。`git status` に新規の追跡対象が増えない（`.aidlc-store.sqlite` が ignore される実測） | NFR1, FD BR2.1 / BR5.1 |
| NFR1.2 | **ロック dir を生成しない** — 退役後のコードに mkdir ロック・`.aidlc-lock` 生成経路が無い | FD BR3.1 / BR3.2 の grep = 0 件、`fs_workspace_lock_test` 等の削除 | NFR1, ADR-007 |
| NFR2.1 | **TDD** — 契約テスト（InMemory / SQLite 両実装で同一関数群）を先に赤で書き、実装で緑にする。ITF・PBT は外側のゲート | PR のコミット列（tests → src）と `cargo test --workspace` 全緑 | NFR2, team.md Testing Posture |
| NFR2.2 | **決定的 PBT** — `PROPTEST_RNG_SEED` 固定で (a) 任意イベント / 状態の encode→decode 恒等、(b) 正準 JSON のバイト決定性、(c) 任意のコマンド列で store×n → 新接続 find_by_id → 状態同値 | PBT 緑、失敗時は seed が再現 | NFR2, FD BR2.5 / BR2.7 |
| NFR2.3 | **カバレッジ** — 絶対 90% 床を維持。adapter クレートに除外を足さない。退役で消えるテスト分の低下は新テストで補う | `scripts/coverage.sh` が PASS（絶対 + PR 相対） | NFR2 |
| NFR2.4 | **規則の機械強制** — clippy 全ルール deny（`unwrap_used` / `expect_used` / `missing_errors_doc` / `missing_panics_doc` / `unreachable_pub` …）で警告 0、`cargo lint` 自己テスト緑（`reap-decision-locality` 削除後にルール表と README を同期）、rustfmt | CI `check` ジョブ緑 | NFR2, team.md Code Style |
| NFR2.5 | **Quint ゲート** — `journal_protocol.qnt` の typecheck / invariants run（8 本）/ witness（4 本、負形式）が `scripts/quint-gate.sh` で緑。mutation 検出を named invariant ごとに記録 | CI `quint` ジョブ緑、code-summary に mutation 表 | NFR2, FD BR3.3 / BR3.4, ADR 0003 |
| NFR3.1 | **再構成の決定性** — `find_by_id` は最新スナップショット + 差分 replay で、同じ DB から何度読んでも `PartialEq` で同値。時刻・乱数・環境を読まない（`updated_at` は書込のみ、再構成に使わない） | ラウンドトリップ契約テスト + PBT (c) | NFR3, FD BR1.2 |
| NFR3.2 | **健全性検査** — 欠損・破損・順序違反は panic ではなく `Corrupt { cause }`（MissingSnapshot / UndecodablePayload / UnknownEventType / SchemaVersion / InvariantViolation / SequenceGap）で拒否、`NotFound` と区別する。`user_version` 不一致は `Schema` | 各原因のテスト（行の直接改竄・削除で再現） | NFR3, FD BR1.2 / BR2.5 |
| NFR3.3 | **原子性と楽観 version** — store は BEGIN IMMEDIATE の単一 Tx（journal + snapshot）。競合は `Conflict { expected, actual }` で、状態は変わらない（rollback）。クラッシュ（COMMIT 前）は何も残さず、COMMIT 後は新接続で同一状態が読める | Conflict テスト（2 再水和の競合）、クラッシュ再構成テスト、ITF journal_protocol（conflict_rejected / snapshot_tracks_journal / version_equals_journal / no_lost_update） | NFR3, FD BR1.3 / BR2.3 |
| NFR3.4 | **チェックポイントの単調性と投影の冪等性の土台** — `advance_checkpoint` は単調（後退は `CheckpointRegression`）、`events_after` は global_seq_nr 昇順で欠落なし。投影の冪等性（U4）はこの 2 性質に依存する | チェックポイント契約テスト + ITF（checkpoint_monotone / checkpoint_bounded / projection_idempotent / truth_is_journal） | NFR3, FD BR1.4 |
| NFR3.5 | **登録簿の直列化** — `within_write_transaction` は同一 DB への並行書込を busy_timeout（5000ms）内で直列化し、超過は `Io(WouldBlock)` として呼出側へ返す（黙って失敗しない） | 同一 DB 2 接続のテスト（片方が Tx を握る間、他方は待つ / タイムアウトで Err） | NFR3, FD BR2.4 |
| NFR4.1 | **依存の差分は 2 追加 1 除去** — workspace 依存に `rusqlite`（`bundled`）と `tokio`（`rt`, `macros`）を固定版で追加、adapter の `md5` を除去。`core-domain` / `core-use-case` には外部クレートを足さない（use-case は `core-domain` + 既存内部クレートのみ） | `Cargo.toml` / `Cargo.lock` の diff がこの 3 点に限られ、`cargo audit` が CI で緑 | NFR4, team.md サプライチェーン, P1 |
| NFR4.2 | **`unsafe_code = "forbid"` 維持** — 自クレートに unsafe を書かない（SQLite の unsafe は `libsqlite3-sys` の内部で、依存として受け入れる） | clippy / rustc 緑 | NFR4 |
| NFR4.3 | **panic しない** — 範囲・減算（`seq_nr − 1`）・索引は事前検査し、失敗は `Err`。`unwrap` / `expect` / `panic!` / `indexing_slicing` を生まない。rusqlite の `Error` は `Io { kind }` / `Corrupt` / `Conflict` / `Schema` へ写す（`Busy` → `WouldBlock`） | clippy deny + レビュー、エラー写像のテスト | NFR4, error-handling.md |
| NFR4.4 | **改竄の検出（完全性）** — ストアは信頼しない入力。復号後の不変条件検査（`from_state` / `apply_event`）を省略しない。暗号学的完全性（署名 / HMAC）は要求しない（ローカル単一ユーザ、git 管理外 — U2 NFR と同じ立場、必要なら後続 intent） | NFR3.2 のテスト | NFR4, P3 |
| NFR4.5 | **秘密情報・ログ・環境変数を扱わない** — パスと Clock は注入、`std::env` を読まない、ログ基盤なし、ペイロードの人間入力は逐語で保存（加工・要約しない） | `core-interface-adapter::orchestration` に `std::env` / `println!` / `eprintln!` が無いことをレビュー・grep で確認 | NFR4, P6, audit-format「Human decisions recorded verbatim」 |
| NFR4.6 | **ファイル権限とパス** — ストアは umask 既定で作成（upstream のワークスペースファイルと同じ）。親ディレクトリ `intents/` が無ければ作らずに `Io(NotFound)`（ディレクトリ構造の権威は upstream レイアウト） | 親 dir 欠落テスト | NFR4, FD BR2.1 |

## 3. 脅威の検討（STRIDE、ライブラリ規模）

| 区分 | 該当 | 扱い |
|---|---|---|
| Spoofing / Elevation of Privilege | 該当なし（認証・認可を持たないローカル CLI。誰がコマンドを発行したかは U7 / フック側） | — |
| Tampering | ストアファイルの改竄・欠損・部分破損、ジャーナルとスナップショットの不整合、並行書込による上書き | NFR3.2（Corrupt 検出）、NFR3.3（単一 Tx + 楽観 version、Conflict）、NFR3.5（直列化）。暗号学的完全性は非要求（NFR4.4） |
| Repudiation | 該当なし（来歴は封筒 `intent_id / seq_nr / occurred_at` と global_seq_nr で追える。人間の決定は逐語保存） | — |
| Information Disclosure | ペイロードの人間入力が平文で SQLite に入る | upstream 同等（監査シャードも平文）。ストアは git 管理外・ローカル。ログ出力なし（NFR4.5） |
| Denial of Service | 巨大なジャーナルの replay、busy_timeout を超える長期ロック保持 | replay は通常 0 件（スナップショット毎 store）、`events_after` は差分のみ。長期保持はプロセス 1 回の Tx に閉じる（within_write_transaction の f はファイル 1 つの read-modify-write）。性能は非目標（§5） |

## 4. データ分類

| データ | 分類 | 扱い |
|---|---|---|
| ジャーナル / スナップショット（ドメインイベント・集約状態） | Internal（ローカル、git 管理外） | 平文。ワークスペースの他ファイルと同じ権限 |
| チェックポイント | Internal | 投影の進捗のみ |
| ペイロード内の人間入力（request / user_input / feedback / reason） | upstream の監査行と同等（平文で逐語） | 加工しない。秘密情報を載せる経路は設けない |

## 5. 非目標（NFR5）

- 数値の性能目標は立てない。bundled SQLite のビルド時間増は CI キャッシュで吸収。実測で明確な劣化があれば課題化。

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-08-23T09:08:14Z
**Iteration:** 1（advisory, unit: u3-event-store-repository）

### Findings

| # | Severity | Location | Finding | Recommendation |
|---|---|---|---|---|
| 1 | Major | security-requirements.md NFR2.3 / tech-stack-decisions.md §2 | `scripts/coverage.sh` の相対ゲート許容誤差 `TOLERANCE` は現在 0.05 に設定されており、そのインラインコメント（同スクリプト冒頭）は「U3 のロック退役（ADR-007）でジッタ源が消えたら 0.01 へ引き締める（NFR2.4）」と明記している（実測: `TOLERANCE=0.05` — ジッタ源は `fs_workspace_lock.rs:237` の並行テスト）。この 0.05→0.01 のタイトニングは team.md Testing Posture が「stage-1 スコープでシード固定により 0.01 へ引き締める」と確約した項目でもあり、まさに本 Unit（ロック退役の実行主体）の Bolt B5 で条件が満たされる。しかし NFR2.3 の合格基準は「`scripts/coverage.sh` が PASS（絶対 + PR 相対）」のみで、TOLERANCE の値そのものには触れていない。0.05 のまま実装しても gate は形式的に PASS してしまうため、このタイトニング作業が実装時に見落とされるリスクがある。 | NFR2.3（または新設の NFR2.x）に「ロック退役完了後、`scripts/coverage.sh` の `TOLERANCE` を 0.05 → 0.01 へ引き締め、該当コメントを更新する」ことを明示的な受け入れ基準として追加する。 |
| 2 | Major | security-requirements.md NFR4.3 | 要求は「範囲・減算（`seq_nr − 1`）・索引は事前検査し…`unwrap` / `expect` / `panic!` / `indexing_slicing` を生まない」ことを求め、合格基準を「clippy deny + レビュー」としている。しかし実測（`Cargo.toml` `[workspace.lints.clippy]`、45 行の全リストを確認）では `unwrap_used` / `expect_used` は deny 済みだが、`indexing_slicing` と `panic`（`clippy::panic`）はいずれも設定されていない（`grep -n "indexing_slicing\|panic" Cargo.toml` = 0 件）。`coding-rules/README.md` 自身が「オーナーの指摘は可能な限り機械的な強制へ落とし込む（型→既存 lint→`cargo lint` の優先順）」と明言しているにもかかわらず、この 2 パターンは「レビュー」という人力チェックのみに依存しており、BR1.3 の `event.seq_nr() − 1` のような u64 減算アンダーフロー（`[profile]` に `overflow-checks` の明示設定も無く実測、release ビルドでは黙って wrap しうる）や配列アクセスでの実際の panic を防ぐ機械的保証が要求の記述と一致しない。 | `indexing_slicing = "deny"` と `panic = "deny"`（clippy）を `[workspace.lints.clippy]` に追加することを本 Bolt（B5）のスコープに明記するか、機械強制を意図的に見送るならその理由を NFR4.3 に明記する。 |
| 3 | Minor | tech-stack-decisions.md §2 | `cargo audit` を CI `audit` ジョブの required status check 化していない点（`.github/workflows/ci.yml` の `audit` ジョブは `ci-success` の `needs` に含まれず advisory 扱い — コメントで意図的と明記）は本 Unit の設計ではなく既存の CI 設計判断であり問題ではないが、NFR4.1 の合格基準「`cargo audit` が CI で緑」は「緑でなくてもマージ可能」という現状の運用（advisory）と字面上ややズレる。 | NFR4.1 に「`audit` ジョブは advisory（required 化なし、`ci-success` からも除外済み）」という既存運用を一行注記し、誤読を防ぐ。 |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| `aidlc-sensor-traceability.ts --stage nfr-requirements` | PASS（`{"pass":true,"gaps":[],"orphans":[],...}`） | traceability.json の upstream_ids（NFR1〜5）と coverage が過不足なく対応 |
| `aidlc-sensor-required-sections.ts`（security-requirements.md） | PASS（h2_count=5） | §1〜§5 の H2 見出し検出、登録簿既定（≥2）を満たす |
| `aidlc-sensor-required-sections.ts`（tech-stack-decisions.md） | PASS（h2_count=3） | §1〜§3 の H2 見出し検出 |
| `aidlc-sensor-upstream-coverage.ts`（`--consumes functional-spec,rules,requirements,contract-summary`） | PASS（unreferenced=[]） | 4 つの上流成果物すべてが security-requirements.md 本文で参照されている |
| `linter` / `type-check` | 該当なし | 本ステージの成果物に TS/JS・TSX コードスニペットが無いため対象外（Rust/SQL のみ） |
| 実ファイル突合（`Cargo.toml` `[workspace.lints.clippy]`、`.gitignore`、`.github/workflows/ci.yml`、`scripts/coverage.sh`、`scripts/quint-gate.sh`、`rust-toolchain.toml`） | 概ね整合、2 件の乖離を Findings #1/#2 に記録 | `unsafe_code = "forbid"` / `permissions: contents: read` / `cargo audit` ジョブ / `.gitignore` の `.aidlc-*` パターンは既に存在し NFR4.1〜4.2・NFR1.1 の前提と一致（新規追加不要）。TOLERANCE と indexing_slicing/panic lint の 2 点のみ乖離 |

### Summary

要求は上流（requirements.md NFR1〜5、functional-design の BR1.x〜BR5.x、C3/C6、ADR-006/007）と広く整合し、traceability・required-sections・upstream-coverage の 3 センサーは全て PASS、STRIDE・データ分類・信頼境界の記述も本 Unit の規模（ローカル単一ユーザ・認証なし）に対して妥当である。一方で、実ファイル突合により 2 件の Major を検出した: (1) `scripts/coverage.sh` の TOLERANCE タイトニング（0.05→0.01）という、コード内コメントが名指しで本 Unit に紐付けている既存 TODO が要求に反映されていない、(2) NFR4.3 が「clippy deny」で防げると主張する `indexing_slicing` / `panic!` が実際にはワークスペース lint に存在しない。いずれも Critical ではなく（ビルド・CI を壊さず、実装完了の見落としリスクに留まる）、Major 2 件で advisory の READY 閾値（Critical 0 かつ Major ≤ 2）内に収まるため、Verdict は READY とする。ただし Bolt B5 の実装ゲートで上記 2 点への対応（または明示的な見送り理由の記載）を確認することを強く推奨する。
