# fold-usage 有効経路の実装と検証（担当 `u2_fold_usage`）

2026-09-10。承認済み実装計画 Step 7 の「fold-usage」を、本家 2.7.1（固定コミット `a277af218f0df7f325d3b8be7b6d90fce2c5bd40`）の実走行採取 `tests/golden/selfhost-stage1/fold-usage.json` とバイト一致する形で `aidlc hook fold-usage` へ接続した。前セッションが残した Red（台帳 `aidlc/.aidlc-sessions/usage-ledger.json` が生成されない）から再開し、Testing Contract（TDD: red → green → refactor）に従って進めた。

結論: 採取 40 観測（`bare` 17・`intent` 17・`rotate` 5・`disabled` 1）すべてで、終了コード・stdout・stderr・`aidlc/` 配下の全ファイル（台帳の全バイト、session と transcript のポインタ）・`.aidlc-sessions` の生成物一覧が本家と一致した。`cargo fmt --all --check`・`cargo clippy --workspace --all-targets -- -D warnings`・`cargo lint`・`cargo test -p aidlc --test fold_usage_contract`・`cargo test -p harness-claude --lib` はすべて終了 0 である。完了時の監査への利用量欄（`Tokens In` 等）は両コーパスに観測が無いため実装せず、差として記録した（§7）。

## 1. 実装した変更

所有範囲の外は編集していない（`runtime.rs`・`intent_execution_event.rs`・`wording.rs`・`layout.rs`・`session_navigation.rs`・他担当のファイルは無変更）。

| ファイル | 区分 | 責務 |
| --- | --- | --- |
| `modules/app/aidlc/src/runtime/fold_usage.rs` | 変更 | フック入口。停止フラグ → 封筒の解釈 → 封筒の session で配置を解決 → 状態ファイルの `Current Stage` → ポインタ書込 → 帰属（stage / `transcript:<path>` / `intent:<uuid>` または `record:<space>/<dir>`）→ ロック下で台帳を読み・畳み・原子的に書く。失敗は握り潰し常に終了 0・stdout 空 |
| `modules/app/aidlc/src/usage_ledger.rs` | 変更 | `UsageLedger`（cursors・workspace 集計・workflows）。`load`（版が 3 未満・`byteOffset` の無い cursor・壊れた JSON は空へ作り直し）・`to_json`/`render`（`JSON.stringify(ledger, null, 2)` と同じ鍵順、末尾改行なし）・`write`（tmp+rename）・`fold_source`（保留 / 締め / 帰属の捕捉 / 切り詰め時の保留破棄 / cursor 前進） |
| `modules/app/aidlc/src/usage_ledger/model_rates.rs` | 新規 | `ModelRates`。既定表 8 世代 + 配布 `tools/data/model-rates.json` + `AIDLC_MODEL_RATES` の重ね合わせ、`normalizeModel` 相当の正規化（別名 → `converse/` → `<region>.anthropic.` → `claude-` → 長い鍵から語境界で照合）、`computeCost` と同じ順の加算 |
| `modules/app/aidlc/src/usage_ledger/transcript_chunk.rs` | 新規 | `TranscriptChunk`。`[byteOffset, size)` のバイト読取、LF 分割と絶対バイト位置、切り詰め検出（`reset`）、末尾断片の保留、flush 時だけ完全な JSON 値を改行なしでも受理 |
| `modules/app/aidlc/src/usage_ledger/message_group.rs` | 新規 | `MessageGroup`。連続する同一 `message.id` の群化と代表行（総量最大・同点は後）。id 無し行は単独、離れて再出現した id は別群 |
| `modules/app/aidlc/src/usage_ledger/fold_attribution.rs` | 新規 | `FoldAttribution`（stage / session / workflow の 3 つ組）と `captured_over`（保留時の帰属を優先し、stage だけは `??` で現在値へ落ちる） |
| `modules/app/aidlc/src/usage_ledger/fold_source.rs` | 新規 | `FoldSource`。main / sub-agent の別、cursor 鍵（ファイルパスの逐語）、`byAgent` の鍵（`main` / sidecar の `agentType` / `subagent`）、締めるか |
| `modules/app/aidlc/src/usage_ledger/transcript_session.rs` | 新規 | `TranscriptSession`。main と `<dir>/<session>/subagents/agent-*.jsonl` の列挙（ファイル名順）、`.meta.json` の `agentType` |
| `modules/app/aidlc/src/usage_ledger/ledger_lock.rs` | 新規 | `LedgerLock`。OS 一時領域の flock によるプロセス間ロック（5 秒待ち）。解放時にファイルも消し、待機側は inode を照合して開き直す |
| `modules/app/aidlc/src/usage_ledger/pending_group.rs` | 変更 | 帰属 3 欄を `FoldAttribution` へ寄せた（JSON の形は不変） |
| `modules/app/aidlc/src/usage_ledger/{stage_bucket,totals,usage_aggregate,workflow_usage,ordered_map}.rs` | 変更 | 未使用アクセサの除去、clippy 指摘（`const fn`・let-else・match guard）の是正。JSON の形は不変 |
| `modules/app/aidlc/tests/fold_usage_contract.rs` | 変更 | `#![allow(clippy::unwrap_used, clippy::panic)]`（既存テストファイルと同じ許可の書き方。期待値は無変更） |
| `modules/harness/claude/src/{fold_usage_envelope,lifecycle_boundary_command,transcript_token_counts,lib}.rs` | 変更 | `rustfmt` の整形のみ（意味の変更なし） |

`runtime.rs` の `run_hook` は前セッションで配線済みで、変更していない。

## 2. Red の再現

いずれも `fold-usage-logs/` に全文がある。既存の `red-01` / `red-02` は上書きしていない。

| ログ | 対象 | 失敗の要点 |
| --- | --- | --- |
| [red-03-ledger-rerun.log](fold-usage-logs/red-03-ledger-rerun.log) | `cargo test -p aidlc --test fold_usage_contract` | 6 件中 4 件失敗。`bare/post/holdback-first` 等で `aidlc/.aidlc-sessions/usage-ledger.json` を読めない（前担当の red-02 と同じ Red を自分で再現） |
| [red-04-model-rates.log](fold-usage-logs/red-04-model-rates.log) | `ModelRates` | 5 件中 4 件失敗（正規化・単価・重ね合わせ・bucket）。`unknown_shapes_stay_unpriced` は `None` を返すスタブで通ってしまった（§8） |
| [red-05-chunk-and-groups.log](fold-usage-logs/red-05-chunk-and-groups.log) | `TranscriptChunk` / `MessageGroup` | 10 件中 9 件失敗。`no_new_bytes_and_a_missing_file_yield_nothing` は `None` を返すスタブで通った（§8） |
| [red-06-ledger.log](fold-usage-logs/red-06-ledger.log) | `UsageLedger` / `TranscriptSession` | 8 件失敗（空台帳の形・作り直し・保留と締め・帰属の捕捉・切り詰め・往復・列挙）。`FoldAttribution` の 1 件は値型なので実装と同時に書いた（§8） |
| [red-12-ledger-lock.log](fold-usage-logs/red-12-ledger-lock.log) | `LedgerLock` | 3 件失敗（保持中の存在と解放時の削除・待機・タイムアウト） |

## 3. Green の一覧

| ログ | 内容 | 結果 |
| --- | --- | --- |
| [green-04-model-rates.log](fold-usage-logs/green-04-model-rates.log) | `ModelRates` 実装後 | 5 件成功 |
| [green-05-chunk-and-groups.log](fold-usage-logs/green-05-chunk-and-groups.log) | `TranscriptChunk` / `MessageGroup` 実装後 | 10 件成功 |
| [green-06-ledger.log](fold-usage-logs/green-06-ledger.log) → [green-06b-ledger.log](fold-usage-logs/green-06b-ledger.log) | `UsageLedger` 実装後。06 では切り詰めテストの前提（保留位置 325 より長い 329 バイトへ差し替えていたので切り詰めにならない）が誤りで 1 件失敗し、テスト側の前提を直して 06b で成功 | 33 件成功 |
| [green-07-contract.log](fold-usage-logs/green-07-contract.log) | `fold_usage::run` を台帳へ接続（red-03 の Green） | 6 件成功 |
| [green-08-harness-claude.log](fold-usage-logs/green-08-harness-claude.log) | `cargo test -p harness-claude --lib` | 73 件成功 |
| [refactor-09-clippy-findings.log](fold-usage-logs/refactor-09-clippy-findings.log) → [green-09b-clippy.log](fold-usage-logs/green-09b-clippy.log) | clippy の所見（未使用アクセサ 6 箇所・`const fn` 2・let-else 1・collapsible match 1、すべて自分の所有ファイル）を是正 | 終了 0 |
| [green-10-lint.log](fold-usage-logs/green-10-lint.log) | `cargo lint` | 終了 0 |
| [green-11-after-refactor.log](fold-usage-logs/green-11-after-refactor.log) | refactor 後の再確認（単体 33・契約 6・harness 73） | すべて成功 |
| [green-12-ledger-lock.log](fold-usage-logs/green-12-ledger-lock.log) | `LedgerLock` 実装と fold-usage の切替後（単体 36・契約 6） | すべて成功 |
| [green-13-final-gate.log](fold-usage-logs/green-13-final-gate.log) | 最終ゲート（fmt / clippy / lint / harness-claude / fold_usage_contract / usage_ledger 単体） | すべて終了 0 |

## 4. 本家との突き合わせ

### 確認できたこと（採取 40 観測とのバイト一致）

契約テスト `fold_usage_contract.rs` は各観測を独立に再現して `aidlc/` 配下の全ファイルを比較し、さらに筋書きごとに 1 本のワークスペースで採取順に撃って最終台帳を比較する。次の振る舞いが本家と一致した。

- PostToolUse の holdback: 最後の群を保留し、cursor の `byteOffset` を群の先頭へ巻き戻し、`pending`（`byteOffset` / `messageId` / `stageSlug` / `sessionKey` / `workflowKey`）に保留時の帰属を捕まえる。
- 通常 PreToolUse の seal-main: main の最後の群を締める（`pending` が消え、`byteOffset` が末尾へ進む）。工程更新を伴う PreToolUse（`bun .claude/tools/aidlc-orchestrate.ts report …`）の flush-all: sub-agent の群も締める。
- 新しいバイトが無い・会話履歴が無い・封筒が JSON でない・`transcript_path` が無い呼出し: 台帳は変わらない（無関係なツールの PostToolUse はポインタだけ更新）。
- 分割行: 先頭行が 0 で最後の行に実測値がある新形式の群を 1 回として数え、途中で切れた書きかけ行は次回へ残す。
- 未知世代（`claude-opus-9-9`）と `<synthetic>`: トークンは `byModel` に逐語鍵で数え、USD は足さない。`message.id` の無い行は単独の群で、cursor の `lastMessageId` を上書きしない。
- Bedrock 形（`converse/us.anthropic.claude-opus-4-8`・`global.anthropic.claude-opus-4-8[1m]`）: `opus-4-8` へ正規化。
- sub-agent: `.meta.json` の `agentType`（`aidlc-developer-agent`）で `byAgent` を分け、sidecar が無ければ `subagent`。cursor 鍵は sub-agent ファイルのフルパス、`sessionKey` は main の transcript。
- 切り詰め（`rotate/flush/truncated`）: 先頭から読み直し、保留を捨てる。欠落（`removed`）: 何も変えない。再増加（`grown-again`）: 位置の途中から読んで行として成立しないバイトを読み飛ばし、`byteOffset` だけ進める。
- 帰属: record の無い workspace は `record:default/legacy`、bugfix の record は `intent:<uuid>` と `byStage.reverse-engineering`。
- 停止フラグ `AIDLC_DISABLE_USAGE_TRACKING=1`: 1 バイトも書かない。
- USD の浮動小数（`0.019613000000000002`・`3.5000000000000004e-5` 等）: 加算順と ECMAScript の数値表記を揃えたので逐語一致。

### 確認できなかったこと（採取に無い経路）

- `AIDLC_MODEL_RATES` と配布 `tools/data/model-rates.json` の重ね合わせは単体テストだけで、本家実走行との比較は無い（採取 workspace には `.claude/` が無く既定表で一致した。本リポジトリの配布ファイルは既定表と同値であることを実測した）。
- ロックの競合は単体テスト（スレッド 2 本）だけで、実際の並行フック同士の観測は無い。
- 監査完了欄・statusline など消費側の観測は無い（§7）。

## 5. 範囲外にした分岐と理由

| 本家の分岐 | 扱い | 理由 |
| --- | --- | --- |
| `updateLedger`（行ベースの API。cursor を `"main"` / `"agent-<id>"` で鍵付け） | 実装せず | フックは `foldTranscriptIntoLedger`（ファイルパス鍵）だけを使う。採取に無い |
| `readTranscript` / `readClaudeSession` / `readClaudeTranscript`（全文読取） | 実装せず | 消費側・テスト用の読取り。producer は位置付き読取りだけを使う |
| `buildAgentTypeMapFromParent`（親会話の `Task` 呼出しから agentType を引く） | 実装せず | 本家の producer も呼んでいない（sidecar の型だけを使う） |
| `sessionUsageAggregate` / `stageUsageAuditFields` / `workflowUsageAuditFields` | 実装せず | 消費側。観測が無い（§7）。C6 の再利用判断は U4 へ |
| Stop フックの flush-all（本家 `aidlc-continue-workflow.ts:1334-1338`） | 未接続 | Rust の Stop 入口は `runtime/continuation.rs` で、所有範囲外。残課題（§9） |
| `BARE_ALIASES` の JS オブジェクト継承（`constructor` 等の名前が真になる） | 再現せず | 言語の事故であり契約ではない |

## 6. 検査の結果表

| 検査 | 結果 | 根拠 |
| --- | --- | --- |
| `cargo fmt --all --check` | 終了 0 | green-13 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 終了 0 | green-13（所見は refactor-09 で是正） |
| `cargo lint` | 終了 0 | green-10・green-13 |
| `cargo test -p aidlc --test fold_usage_contract` | 6 件成功 | green-07・green-11・green-12・green-13 |
| `cargo test -p harness-claude --lib` | 73 件成功 | green-08・green-13 |
| `cargo test -p aidlc --lib -- usage_ledger` | 36 件成功（新規 27・既存 9） | green-13 |
| `cargo test --workspace`・カバレッジ床・Quint/ITF・release バイナリ | 未実施 | B1 統合前の共通検査（承認済み計画 Step 9）。親の判断で実行する |

閾値・ゴールデンの期待バイト・正規化設定（`normalization: []`）・clippy / lint の設定は変えていない。

## 7. 完了時の監査への利用量欄（Step 5 の調査）

- 本家: `aidlc-state.ts` は `stageRollupFields` / `workflowRollupFields`（`:231-245`）を `STAGE_COMPLETED`（`:3916`・`:4279`・`:4889`）と `WORKFLOW_COMPLETED`（`:1428`・`:5517`）の欄へ足す。欄は `Tokens In` / `Tokens Out` / `Cache Read` / `Cache Write` / `Cost USD`（値付き・`null`・省略の 3 状態）/ `By Model` / `By Agent` / `Tokens By Model` / `Tokens By Agent`（`aidlc-usage.ts:1213-1264`）。台帳が無い・停止中・当該 workflow の集計が無いときは欄を足さない。
- 本 build: RMU の `STAGE_COMPLETED` は `Stage` / `Details`（+ Validation Basis 等）だけを描き、利用量欄は描かない（`modules/core/read-model-updater/src/workspace/audit_block.rs` / `projection.rs`。Rust 側に `Tokens In` 等の文字列は存在しない）。
- 観測: `tests/golden/selfhost-stage1/fold-usage.json` は監査を含まず、`tests/golden/upstream-a277af21/` は 207 件の完了ブロックのいずれにも利用量欄が無い（base64 を復号して走査）。U1 の 75 観測はすべて `AIDLC_DISABLE_USAGE_TRACKING=1` で採取されている。
- 判断: 観測が無いので実装しない。有効時に完了監査へ欄が足される差は残るが、推測で欄を足さず、親と人間の裁定へ渡す。裁定に必要なら、固定コミットで有効時の `approve` / `complete-workflow` を追加採取する（既存の期待バイトは書き換えない）。

## 8. TDD の順序で守れなかった箇所

- `unknown_shapes_stay_unpriced`（red-04）と `no_new_bytes_and_a_missing_file_yield_nothing`（red-05）は、`None` を返すスタブで Red の時点から通ってしまった。振る舞いは Green の実装で他のテストと同時に検証されている。
- `FoldAttribution::captured_over` の単体テストは値型の実装と同時に書き、Red を採っていない（red-06 のログで最初から成功している）。
- green-06 で切り詰めテストの前提（差し替え後のファイルが保留位置より長かった）が誤りで、テスト側を直した。実装は変えていない。
- 契約テスト（40 観測）は前担当が先に置いたものを red-03 で再現し、Green まで期待値を変えていない。

## 9. 残課題

- Stop フックの flush-all（本家は `aidlc-continue-workflow.ts` が `foldTranscriptIntoLedger(..., "flush-all")` を呼ぶ）を Rust の `runtime/continuation.rs` へ接続する。本担当の所有範囲外。接続時は `usage_ledger::{UsageLedger, LedgerLock, ModelRates, TranscriptSession, FoldAttribution}` と `fold_usage.rs` の帰属解決を再利用できる（帰属の関数は現在 `fold_usage.rs` 私有）。
- 完了監査の利用量欄（§7）の裁定と、必要なら有効時の追加採取。
- `cargo test --workspace`・カバレッジ床 90.0% と相対ゲート・Quint/ITF・release バイナリでの契約確認は B1 統合前の共通検査として親が実行する。
- 本家との既知の差（§Assumptions）を人間へ提示し、必要なら追加採取で裁定する。

## Sources

- 本家固定コミット `a277af218f0df7f325d3b8be7b6d90fce2c5bd40`（`git -C vendor/aidlc-workflows show <commit>:dist/claude/.claude/<path>` で実バイトを参照）: `hooks/aidlc-fold-usage.ts:1-135`、`tools/aidlc-usage.ts:80-89,141-150,161-178,193-253,268-283,316-380,439-458,487-505,584-597,659-769,780-808,813-952,954-1011,1013-1025,1213-1264,1290-1327,1354-1693`、`tools/aidlc-state.ts:219-245,1428,3916,4279-4280,4889,5517`、`tools/aidlc-lib.ts:2557-2575,1557-1652,2350-2400,2448-2486,3219-3260,3605-3700,3818-3843,16913-16916,18166-18186,18632-18690,20622-20655`、`hooks/aidlc-continue-workflow.ts:164,1334-1338`、`tools/data/model-rates.json`。
- 採取: `tests/golden/selfhost-stage1/fold-usage.json`（`source.commit` = 固定コミット、`normalization: []`）、採取器 `scripts/goldens/capture-fold-usage.ts`、来歴 `fold-usage-logs/provenance.log`。
- U1 コーパス: `tests/golden/upstream-a277af21/`（利用量欄の不在と `AIDLC_DISABLE_USAGE_TRACKING=1` の実測）。
- 前担当の記録: `usage-applicability.md`、`fold-usage-logs/red-01-hook-entry.log`、`fold-usage-logs/red-02-ledger.log`。
- 本 build の実測: `modules/app/aidlc/src/runtime.rs:1033-1117`（`run_hook` の配線）、`modules/app/aidlc/src/layout.rs:130-200`（`resolve_for_session` / `shared` / `bound_selection`）、`modules/app/aidlc/src/intent_location.rs`、`modules/app/aidlc/src/runtime/session_hooks.rs:97-103`（`field`）、`modules/core/infrastructure/src/{atomic,exclusive_file_lock,ecmascript}.rs`、`modules/core/infrastructure/src/canon_json/`、`modules/core/read-model-updater/src/workspace/{audit_block,projection}.rs`、`.claude/tools/data/model-rates.json`、`.claude/settings.json:29,148`（fold-usage の登録）、`Cargo.toml`（workspace lints）、`clippy.toml`、`rustfmt.toml`。
- 規則: 承認済み `code-generation-plan.md`（Step 7・Testing Contract）、`unit-test-instructions.md`、`aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/`（README・abstract-data-type・field-visibility・module-visibility・command-query-separation・error-handling・factory-naming・first-class-collections）、`aidlc/spaces/default/memory/{org,team,project}.md`。

## Assumptions & Open Questions

- [assumption] sub-agent ファイルの畳み込み順はファイル名の辞書順とした。本家は `readdirSync` の順（OS 依存）で、採取には同じ呼出しで複数の新規 sub-agent ファイルが現れる観測が無い。`cursors` の鍵順に影響しうる。
- [assumption] sub-agent の cursor 鍵は `Path::join` で組んだ。本家は `path.join` で正規化する（`..` や重複 `/` の解消）。Claude Code が渡す会話履歴パスは絶対で正規化済みなので差は出ない前提。
- [assumption] `Current Stage` は既存フックと同じ `session_hooks::field`（`- **Current Stage**: ` 行）で読んだ。本家は `/Current Stage\*{0,2}:?\s*`?([^\n`]*)`?/` の最初の一致を採る。エンジンが書く状態ファイルでは同じ値になる。
- [assumption] workflow の帰属は既存の `Layout::resolve_for_session` と `IntentLocation::current` で解いた。本家 `activeIntent` のカーソル無し時の「唯一の record」フォールバックと、`intents.json` の行に `dirName` が無い旧形の `<slug>-<id8>` 照合は再現していない（採取に無い）。
- [assumption] JSON の読取りは `serde_json`（会話履歴・sidecar）と `canon_json::parse`（台帳・単価ファイル、鍵順保持のため）で、`JSON.parse` と極端な入力（範囲外の数値 `1e999` 等）で挙動が異なりうる。採取には無い。
- [assumption] ロックは本家と同じく workspace の外（OS 一時領域）に置き、解放時に消す。本家の mkdir ロックとは相互排他しない（配布設定はどちらか一方のフックだけを登録する前提）。
- [assumption] 台帳は新しいバイトが無くても本家と同じく毎回書き直す（内容は同一。契約テストは内容で比較する）。
- Open: 完了監査の利用量欄（§7）を本 build に足すかどうか。足すなら有効時の追加採取が要る。
- Open: Stop フックの flush-all の接続（§9）を誰がいつ行うか。
