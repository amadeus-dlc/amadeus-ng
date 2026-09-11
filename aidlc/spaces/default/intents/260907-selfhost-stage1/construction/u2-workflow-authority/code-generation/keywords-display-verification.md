# keywords ブロック列の読取と `PipelineLinkError` の `Display` 修正（U2、担当 `u2_keywords_display`）

2026-09-11。裁定 [coverage-round2-questions.md](coverage-round2-questions.md) Q1 = A（ブロック列の読取を TDD で追加）、Q2 = A（`Display` の混入を直す）に対する実装記録。ログは [keywords-display-logs/](keywords-display-logs/)。

## 作業 1（Q1）: 配布 scope ファイルの `keywords:` ブロック列

### 本家の受理範囲（根拠）

本家 `vendor/aidlc-workflows` 固定コミット `a277af21`（`a277af218f0df7f325d3b8be7b6d90fce2c5bd40`）の `dist/claude/.claude/tools/aidlc-lib.ts` で、scope frontmatter の `keywords` は `listField(fm, "keywords")`（`loadScopeMetadataAll`、20800 行）で読まれる。`listField`（21206 行）の受理範囲は次の 3 つで、それ以外は空の列を返す。

| 形 | 本家 `listField` の扱い | 本 build（変更後） |
| --- | --- | --- |
| ブロック列: `keywords:` の直後にインデントされた `- item` 行が続く（ダッシュの後に空白 1 つ以上。`-foo` は要素にならない）。インデントされた `- ` 行以外に当たった時点で列を閉じ、後続の `description: >` の折返し行を列へ漏らさない。各要素の前後の引用符を剥がす | 受理 | `parse_block_sequence` で受理（同じ終端条件、`unquote` で引用符を剥がす） |
| フロー列: 同じ行に `[a, b]`（`[` で始まる） | 受理（`parseInlineDepsList`） | `parse_flow_sequence` で受理（従来どおり） |
| 空: `keywords:` の後にブロック要素もフロー列も無い、`keywords: []`、キー自体が無い | 空 | 空 |
| 裸のスカラー: `keywords: api` | **空**（フロー列の正規表現は `[` を要求する） | **空**へ変更（従来は `["api"]` として受理していた。配布 12 ファイルに裸スカラーは無く、`store` 側の書出しはフロー列なので実運用の影響は無い） |

配布 `.claude/scopes/aidlc-*.md` 12 ファイルの実測: ブロック列 8（bugfix / express / infra / mvp / poc / refactor / security-patch / workshop）、`keywords: []` 4（classic / enterprise / feature / selfhost-stage1）。従来の読取ではブロック列 8 ファイルのキーワードがすべて空になり、`next fix the crash` が「compose 提案」へ落ちていた（Red ログ 05 の panic 文言がそれを示す）。

差分の限界（本家との細部の違い、観測差として記録）:
- 本家の引用符剥がしは先頭と末尾を独立に剥がす（`"api'` も `api`）。本 build の `unquote` は同種の対になった引用符だけを剥がす。配布ファイルに片側だけの引用符は無い。
- 本家のフロー列は引用符内のカンマや `]` を字句解析する。本 build は従来どおりカンマ分割（この作業では変更していない）。

### 変更ファイル

- `modules/core/command/interface-adapter/src/orchestration/compiled_definition_repository_impl.rs`
  - `parse_scope_metadata`: 行の走査を `Peekable` にし、`keywords:` の値が空ならブロック列 `parse_block_sequence` を読む。ブロック列は要素行だけを消費し、閉じた行（次のトップレベルキー）は従来どおりキーとして読み直す。
  - `parse_flow_sequence`: `[` で始まらない値は空を返す（本家に揃える）。
  - `parse_block_sequence`（新規）: 上表のブロック列の受理条件。
  - doc コメント（受理する形の記述）を更新。
  - 単体テスト: `keywords_read_the_flow_sequence_and_tolerate_a_bare_scalar` を `keywords_read_the_flow_sequence`（裸スカラーの期待を除去）に改め、`keywords_read_the_block_sequence_as_distributed` / `keywords_block_sequence_stops_at_the_next_top_level_key` / `keywords_are_empty_when_neither_a_block_nor_a_flow_sequence_follows` / `keywords_block_sequence_does_not_swallow_following_scalar_keys` を追加（4 件）。
- `modules/app/aidlc/tests/refusal_paths_contract.rs`
  - `a_brownfield_workspace_prices_the_inferred_scope_with_reverse_engineering`: 前担当がフロー列へ書き換えていた処理を外し、配布ファイルどおりのブロック列（`keywords:\n  - fix\n  - bug\n  - broken\n`）のまま推論経路を踏む形に戻した。期待する質問文 `This looks like "bugfix" work` は本家 `aidlc-orchestrate.ts`（a277af21、4394 行）の文言に基づく既存の期待を維持。`tests/golden/upstream-a277af21/` にキーワード推論の採取は無かった（`looks like` で grep して 0 件）ので、期待値の変更や追加はしていない。

### Red / Green の根拠

| ログ | 内容 | 結果 |
| --- | --- | --- |
| `03-keywords-unit-red.log` | 新規単体テスト 4 件（変更前の実装） | 4 件 FAILED（`left: []`） |
| `04-keywords-unit-green.log` | 実装後の単体テスト `keywords_` 5 件 | 5 passed |
| `05-app-inference-red.log` | app テストをブロック列へ戻した状態で、keywords 読取だけを一時的に旧挙動（フロー列のみ）へ戻して採取 | FAILED（`None of the ready-made plans is an obvious fit for: "fix the crash"` = compose 提案へ落ちる） |
| `06-app-inference-green.log` | 実装を戻して同テスト | 1 passed |

## 作業 2（Q2）: `pipeline_link_error.rs` の `Display` 混入

### 意図した文言の根拠

`modules/core/command/domain/src/orchestration/pipeline_link_error.rs` は git 未追跡（`??`）で、`git log -p` にも `main` にも履歴が無い（`git log -S"指定されたlink"` も 0 件）。意図した文言は次の 2 点から `"pipeline link: {self:?}"` と判断した。

1. 兄弟の `continuation_error.rs` が `write!(f, "continuation: {self:?}")` と同じ形（材料のみの 1 行）で書かれている（`coding-rules/error-handling.md` の「`Display` は材料のみ」）。
2. 混入した `/// 指定されたlink。` は同ファイルのフィールド `link` に付いている doc コメント（15 / 31 / 42 行）と同一で、`link` という語の直前に doc 行を機械的に挿入した結果が書式文字列の `pipeline link:` にも当たったと読める。

### 変更ファイル

- `modules/core/command/domain/src/orchestration/pipeline_link_error.rs`: `Display` を `write!(f, "pipeline link: {self:?}")` の 1 行に修正。
- `modules/core/command/domain/tests/rejection_material_contract.rs`: `the_pipeline_link_rejection_renders_its_debug_material` を、先頭 `pipeline` と末尾 `Debug` 材料だけの緩い固定から、`"pipeline link: ArtifactRequired"` / `"pipeline link: Command(IntentMismatch)"` の完全一致と、`UnknownLink` 変種に改行・`///` が混入しないことの検証へ強化。

### Red / Green の根拠

| ログ | 内容 | 結果 |
| --- | --- | --- |
| `01-display-red.log` | 強化したテスト（修正前） | FAILED（`left: "pipeline \n/// 指定されたlink。\nlink: ArtifactRequired"`） |
| `02-display-green.log` | 修正後 | 1 passed |

## 完了条件の検査

| ログ | コマンド | 結果 |
| --- | --- | --- |
| `07-cargo-test-three-crates.log` | `cargo test -p core-command-interface-adapter -p core-command-domain -p aidlc` | exit=0（テスト結果ブロック 68 件すべて ok、FAILED 0） |
| `08-fmt.log` / `09-fmt-recheck.log` | `cargo fmt --all --check` | 08 はテストファイルの整形差で exit=1 → `rustfmt` で整形 → 09 で exit=0 |
| `10-clippy.log` | `cargo clippy --workspace --all-targets -- -D warnings` | exit=0（警告・エラーなし） |
| `11-lint.log` | `cargo lint` | exit=0 |

## 触れていないもの

- 承認済み `code-generation-plan.md` / `unit-test-instructions.md` / `code-generation-questions.md`、`.claude/` 配下、配布 scope ファイル。
- `tests/golden/upstream-a277af21/`（採取の追加・修正なし）。
- コミット・push・stash は行っていない。
