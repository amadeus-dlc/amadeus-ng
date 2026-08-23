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
