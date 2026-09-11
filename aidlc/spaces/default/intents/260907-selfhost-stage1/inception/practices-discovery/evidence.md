# 開発規則の根拠

**状態: 回答確認済み・工程承認待ち。** 2026-09-07 の読取り調査。担当は `aidlc-pipeline-deploy-agent`。独立した3名の確認とインタビューの回答を統合済み。内容確認は `Looks correct`。memoryへの承認反映と工程承認は未実施。

## 調査の基準と範囲

`git rev-parse HEAD main` の双方は `1dc727e00a26c27258d916cb3a5e0592c6c48c1c`。作業ツリーには親が作成した今回の計画・作業記録があるため、作業ツリー全体がクリーンとは記録しない。`mise trust` は「No untrusted config files found」を返した。

`aidlc-state.md` は Brownfield、`selfhost-stage1`、現在工程 `practices-discovery`、承認時刻は空。既存 `memory/team.md` と `project.md` の5領域はテンプレートであり、例示コメントを承認済み規則として採用しない。`memory/org.md` と `phases/inception.md` を読み、依頼原文の明示要件を優先した。

削除済みの reverse-engineering 成果物は使用していない。この工程は承認済み計画で省略されているため、設定と必要経路だけを直接確認した。全コードの再文書化・設計監査・テスト実行は実施していない。

## 読み取った事実

以下のパスはリポジトリルート相対。

| 領域 | 現物と位置 | 実測・適用上の限界 |
|---|---|---|
| 作業方式 | `git log -12 --format='%h %s'`、`memory/org.md` の Way of Working、依頼原文の進め方の規律 | main の履歴には番号付きの統合コミットが並ぶ。これだけで過去の全統合方式は証明できない。今回の直列1本・squashは原文で明示されている。 |
| 最小の通し経路 | `.codex/scopes/aidlc-selfhost-stage1.md`、`memory/org.md` の Walking Skeleton | 調査時点の専用スコープに `skeleton` キーはなかった。実行方針は推定せず、後述のQ1回答によって通常の作業単位と確定した。 |
| CI起動 | `.github/workflows/ci.yml:3` | main向け変更、merge_group、手動起動を定義。push起動・自動配備処理はこのワークフローにはない。 |
| CI構成 | `.github/workflows/ci.yml` の jobs | aidlc-distribution、check、quint、coverage、audit、review-thread-resolution、ci-success の7ジョブ。別ワークフロー `review-thread-resolution.yml` には再評価の refresh ジョブがある。 |
| 形式・単体検証 | `.github/workflows/ci.yml` の check、`.cargo/config.toml` | fmt、clippy、cargo lint、workspace test、および workspace外の tools/lint に同じ検査がある。今回これらは再実行していない。 |
| カバレッジ | `scripts/coverage.sh:37`–`54`、`ci.yml` の coverage | 床90.0%、相対許容0.01ポイント、シード20260823。唯一の明示除外はappのmain.rs。変更レビュー時はbase比較、その他は絶対床。コメントが参照する古いteam.mdは現在テンプレートのため、根拠は定数と条件分岐である。 |
| モデル検査 | `scripts/quint-gate.sh:25`–`27`、同 invariants・witness・deterministic scenarios | engine_loop・stop_hook・journal_protocol を扱う。型検査、不変条件、反例経路、決定的シナリオを検査する設定がある。 |
| 実装との照合 | `modules/core/command/domain/tests/engine_loop_conformance.rs`、`modules/app/aidlc/tests/journal_protocol_conformance.rs` | ITF再生検査が存在する。ファイルの存在を成功証拠にはしない。 |
| 依存監査 | `.github/workflows/ci.yml` の audit と ci-success | workspaceとtools/lintのCargo.lockを監査する。auditは集約のneedsに含まれない。今回の完了条件は全ジョブ成功なので集約成功だけでは不足する。 |
| レビュー収束 | `.github/workflows/ci.yml` の review-thread-resolution、`.github/workflows/review-thread-resolution.yml` | 再利用ワークフローをSHAで固定し、未解決コメントの検査を集約へ接続する。外部定義本文やGitHub側の保護設定は本調査では未取得。 |
| ツール固定 | `rust-toolchain.toml`、`rustfmt.toml`、`Cargo.toml:29`–`47` | Rust1.95.0、rustfmt/clippy/llvm-tools、幅100・Unix改行。unsafe禁止、unreachable_pub、unwrap/expect等のdeny設定を確認した。 |
| 設計検査 | `tools/lint/src/check.rs:27`–`76`、同冒頭、`tools/lint/src/domain_getter.rs` | check.rsの6規則と別実装のuse-case-domain-getterで7規則。規則本文より機械検出が狭い箇所がある。lint成功だけで全規則順守を保証しない。 |
| 配布版 | `git submodule status vendor/aidlc-workflows` | `801c570062f67dc8f4952ee5fc601381d09db7ec`。 |
| グラフの版差 | `.claude/tools/data/stage-graph.json`、`vendor/aidlc-workflows/dist/claude/.claude/tools/data/stage-graph.json`、`tests/golden/upstream-3c3146cf/stage-graph.json` | 前2つのMD5は `38310798f41e8173e6e27b9d3078e840`、ゴールデンは `3ee59d7a177bd55d2e8392fb9028561d`。版差がある。全配布ファイルのバイト一致までは今回再検査していない。 |
| CLIの未接続 | `modules/app/aidlc/src/cli/request.rs` のRequest | decision/answer/link は LogNotWired。doctorの未対応文法を冒頭コメントも明記する。今回、全到達経路の列挙は実施せず要求分析へ残す。 |
| フック配置 | `modules/harness/claude/src/lib.rs`、`modules/harness/infrastructure/src/lib.rs` | 説明・crate属性だけであり、4フックの動作実装はない。 |

## 正本の扱い

`coding-rules/README.md` の優先順は、観測互換 → 射程内で例外を認めない規則 → 抽象データ型の目的 → 新しい裁定日の本文 → 表・語彙の既定。版差をこの優先順で独断解決してはいけない。依頼どおり人間へ提示する。

`no-public-fields` のソースコメントは制限付き公開の許容を述べる一方、正本の field-visibility はそれを禁止する。機械検出範囲の狭さを禁止の緩和と読み替えない。今回その検出範囲を拡張する仕事は追加しない。

## 利用者が既に決めたこと

原文の「進め方の規律」から TDD、直列1本、squash、CIと収束、必要契約差分のみ、日本語を採用する。「基準と正本」から配布再利用・文書配置・裁定を採用する。「目的と到達条件」から実地スモーク・自己診断・全ジョブ成功・2版運用を採用する。これらを同意のない推定回答として記録せず、再質問もしない。

## 独立レビューの統合

以下は各支援担当の読取り結果を統合したものであり、主担当が全ケースを再実行した報告ではない。

### 品質担当の異議と対応

[品質担当の確認](contributions/aidlc-quality-agent.md) の異議は「既存ゴールデンの比較粒度・既知の欠落と、runtime呼出しによる統合テストの限界をまだ記録していない」という記載不足だった。次の根拠と限界を本節およびチーム規則に追記し、主担当として記載不足を解消した。品質担当の寄稿にある元の `OBJECT` は履歴として保持し、同担当による再判定や完全互換の確認が済んだとは扱わない。

- `modules/app/aidlc/tests/cli_golden_test.rs:20` 以降は駆動できないケースを明記し、同35行以降は `conductor_persona` / `narration` の欠落を明示する。`load-steering` / `run-stage` の一部はキー集合の比較である。`modules/core/read-model-updater/tests/projection_golden_test.rs` のバイト比較と、このキー集合比較を区別する。既存ゴールデン検査の成功だけでは、実配布資産の完全互換や切替条件2の達成は証明できない。比較粒度・未検証ケース・明示された差は、採用版および実装対象を独断で選ばず要求分析へ渡す。
- `modules/app/aidlc/tests/steering_across_processes.rs:7` 以降は、プロセス内状態の観点で `runtime::run` の反復が同値であるとの検査前提を説明しており、実OSプロセスを起動する試験ではない。`intent_lifecycle.rs:191` の `append_human_turn` はフックの代わりに監査へ事実を置く試験装置である。これらは releaseバイナリ・Claude Code・実フック・人間の承認を使う実地スモークの成功証拠ではない。main.rs のカバレッジ除外も実地スモークを省く根拠にならない。
- engine_loop と journal_protocol にITF再生検査があることを、3モデルすべてのRust実装との照合完了へ拡大しない。Quintの到達性検査は `[violation]` を確認して解析エラー等と区別している。
- 切替時のCI結果は対象コミット・起動イベント・各ジョブの結果を添えて読む。現在、`pull_request` は相対カバレッジとレビュー検査を使い、`merge_group` / `workflow_dispatch` は絶対床とレビュー検査の意図的スキップを使う。非適用の検査を成功と書き換えず、条件との関係が切替判断に影響するなら人間へ提示する。これは既存イベント条件の観測であり、新たな適用除外の承認ではない。

### 開発担当の補足

[開発担当の確認](contributions/aidlc-developer-agent.md) は草案自体に異議なし。正本参照を維持し、次の射程上の注意を引き継ぐ。

- コマンド側のユースケースはポートtraitへのジェネリクス依存とRepository保持を使う。ユースケースからのドメインgetter禁止を、アダプタやクエリ側Viewへ拡大しない。
- クエリアダプタの通常依存と投影テスト用dev-dependenciesを区別する。`Cargo.toml` 全域で相手側への参照がないとは断定しない。
- `thiserror` / `anyhow` 不使用は自分たちのエラー型の規則であり、推移依存を禁止しない。壊れた履歴の再構成に対する理由付きpanicの既存例外を、一般の入力失敗へ広げない。
- Claude固有のJSON契約・発火条件と、汎用的なプロセス・stdio機構を既存の配置境界に従って扱う。新しい型・配置の設計は本工程では行わない。
- Command戻り値の正本と現コードの差は、下記の要求・契約分析で確認する未決事項へ分離した。

### セキュリティ担当の補足

[セキュリティ担当の確認](contributions/aidlc-devsecops-agent.md) は草案自体に異議なし。次の点を現設定の事実として引き継ぎ、新しい検査の導入要件にはしない。

- `clippy.toml` はテスト内の unwrap / expect を許容する。unsafe禁止等のworkspace設定とテスト向けの射程を分ける。
- `.github/`、`scripts/`、`.cargo/` と対象manifestの検索では、専用SAST（ソースコードの脆弱性検査）・DAST（実行中のアプリの検査）・秘密情報検査の実行設定を確認できなかった。GitHub側の設定は未確認であり、無効と断定しない。
- 配布元の版の正本は gitlink（親リポジトリが持つsubmoduleのコミット参照）である。`installed.json` はファイルのSHA-256・実行権限・保持設定の台帳。同期は配布元の未コミット変更、管理パス、シンボリックリンク等を検査し、パッチを適用する。独自変更は既存の同期パッチ管理と配布検査を通す。
- 一部ActionはSHA固定だが、`actions/checkout@v4` や `dtolnay/rust-toolchain@master` も残る。全Action固定済みとは書かない。Rust版は1.95.0、`event-store-adapter-rs` は `=3.0.0` だが、他依存までmanifestで完全固定とは扱わない。

## インタビューと内容確認

[確認事項](practices-discovery-questions.md) Q1 への入力は「推奨」、記録された選択は「A. 通常の作業単位で進める」。独立した先行の最小通し実装を設けず、必須差分を依存順に実装し、自律実行を行わない。TDD、完了時の実地スモーク、既存品質基準は維持する。

同ファイルの `Consolidated Summary Confirmation` に `[Answer]: Looks correct` が記録され、親が対応する確認受領の成功を確認した。ゴールデン採用版・必要呼出集合・担当範囲の裁定は、この回答に含めず後続へ残す。主担当が回答・状態・監査・memoryを書き換えることはしていない。

## 未決事項

1. 要求分析で裁定: ゴールデンをfork tipへ再採取するか、2.6.40に据え置くか。版依存の契約確定前に回答を得る。
2. 要求分析で実測: bugfix一周が呼ぶ動詞・フック・受領の集合と、バイナリ／配布TypeScript補助の担当範囲。規則確認の段階で機能を想像して追加しない。
3. 実行計画・切替工程で具体化: 安定タグ、ホストとターゲットの参照先、復帰条件。この工程でタグ名や配備周期を推定しない。
4. 要求・契約分析で影響を確認: `aggregate-commands.md` が定めるユースケースのCommandの `Result<(), E>` と、`CommitVerdictUseCase::execute` が材料を返す現行契約の差。追加する受領動詞に影響する場合、現コードと正本を添えて人間へ裁定を求める。ここでは既存実装の変更や例外を決めない。

## Sources

- [desc] Initial description: [依頼原文](../../project-description.json)。
- [scope] Workflow-selected scope: `selfhost-stage1`、[状態記録](../../aidlc-state.md)。
- [Q1] [確認事項](practices-discovery-questions.md): 通常の作業単位で進める。内容確認は `Looks correct`。
- 独立担当の調査根拠は上記3寄稿を参照。
- 調査日: 2026-09-07。ローカル全テスト・実地スモーク・バイナリ自己診断・最新CI状態の検証は未実施。

## Assumptions & Open Questions

独立支援レビューと本工程の質問・内容確認は完了した。ゴールデン版等の後続裁定事項は未解決のまま記録する。本成果物は工程承認、memoryへの反映、stage-1到達を示さない。
