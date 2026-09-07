# pending-revision — U3 code-generation（2026-09-07 再走の所見と折り戻し先）

> 本ステージの成果物（`code-generation-plan.md` / `unit-test-instructions.md` / `code-summary.md` / `traceability.json`）は独立レビュー
> （advisory、iteration 1、**READY** — Critical 0 / Major 0 / Minor 4、2026-09-07T07:56:32Z）の受領で凍結された。ここには、開発担当の照合で
> 見つかった不一致 M-1〜M-6 と、レビュー所見 R-01〜R-04 について、コンダクタが現行コードで再実測して確定した扱いと折り戻し先を書く。
> 適用はステージゲート（unit-major では全 Unit 決着後の code-generation ゲート）の Request Changes 経路、または各設計段のゲートで行う。
> 実装（`formal/orchestration/journal_protocol.qnt` の凡例 5 行）は本 Bolt b52 で変更済みで、ここに実装側の残件は無い。

## 1. 行範囲の数え方の統一（レビュー R-01 ← 開発 M-1 / M-2 / M-3）

**裁定**: 関数・ブロックの行範囲は「開始行（`fn` / `enum` / `if` の行）〜閉じ括弧の行」で数える。`security-design.md` の引用の大半
（`write_error :277-303`、`read_error :245-265`、`find_by_id :331-439`）が既にこの数え方であり、変更が最小になる。この基準で再判定した結果:

| 対象 | 設計の記述 | 実測（閉じ括弧基準） | 扱い |
|---|---|---|---|
| `IntentExecution::new`（`security-design.md` §2 層 (2)） | `intent_execution.rs:290-345` | `:290-337`（`:338-345` は `replay` の doc コメント） | **不一致（M-1）**。ND pending-revision R-04 と同一。適用文面: `:290-337` |
| `IntentExecutionDto::to_domain`（同 §2 層 (2)） | `dto/intent_execution_dto.rs:208-293` | `:208-294`（`:293` は最終文、`:294` が閉じ括弧） | **不一致（M-2）**は基準上も維持する。適用文面: `:208-294` |
| `stored_version`（同 §3） | `:313-321` | `:313-320`（`:321` は `impl` の閉じ括弧） | **不一致（M-3）**。適用文面: `:313-320` |
| 書込前ガード（同 §2 層 (0)） | `store:447-452` | `:447-453`（`:452` は `});`、`:453` がガード節の閉じ括弧） | 基準を揃えると**不一致**（レビュー R-01 の指摘で新規）。適用文面: `:447-453` |
| `CorruptDetail`（同 §2） | `:101-113` | `:101-114`（`:113` は最後の変種、`:114` が `enum` の閉じ括弧） | 同上（新規）。適用文面: `:101-114` |

折り戻し先: `../nfr-design/pending-revision.md`（R-04 に上の 5 行を統合）→ nfr-design ゲート。`code-summary.md` §3 冒頭には凍結解除時に
「行範囲は開始行〜閉じ括弧」の 1 文を加える。

## 2. `unit-test-instructions.md` §2 の `dto` 期待件数（開発 M-4）

`#[test]` + `#[tokio::test]` の合計を `dto/tests.rs` の 1 ファイルで数えて **29** と書いたが、`--lib orchestration::dto` フィルタは同ディレクトリ
7 ファイルに散る **47** を実行する（`tests.rs` 29 / `dto_vocabulary.rs` 6 / `workflow_definition_dto.rs` 6 / `dto_decode_error.rs` 2 /
`intent_execution_aggregate_key_dto.rs` 2 / `intent_aggregate_key_dto.rs` 1 / `workflow_definition_aggregate_key_dto.rs` 1。コンダクタ再実測
2026-09-07 でも 47）。`security-design.md` §2 / `logical-components.md` §4 の「`dto/tests.rs` 29 件」は正しく、直すのはテスト手順だけ。

適用文面（§2 の実測段落）: 「`dto` 47（`dto/tests.rs` 29 + 同ディレクトリ 6 ファイル 18）」。折り戻し先: 本ステージのゲート（Request Changes
で `unit-test-instructions.md` を改訂し、承認指紋を取り直す）。教訓は §7。

## 3. 計画本文の引用精度（開発 M-5）とブリーフの節番号（開発 M-6）

- M-5: 計画 §3 Step 2 (d) の「ファサード `orchestration/mod.rs:38-66`」は、最後の `pub use workflow_definition_repository_impl::{ … };` が `:66`
  に始まり `:68` で閉じるので `:38-68` が正（起点 5 か所 `:38-41` / `:46` / `:52` / `:62` / `:65-66` は一致）。折り戻し先: 本ステージのゲート
  （計画は承認指紋の対象。文面だけの訂正なので Request Changes 時にまとめる）。`logical-components.md` §1 は起点しか書いていないため無関係。
- M-6: `developer-brief-9.md` の「functional-spec §5 の `:137`」は §6（`## 6.` は `:133`）が正。ブリーフは記録であり成果物ではないので
  訂正しない（本ファイルで訂正を記録する）。

## 4. `traceability.json` の target 3 件（レビュー R-02 / R-04、コンダクタ判断の記録）

- **BR5.2**（R-02）: 開発担当は記録側 `code-summary.md` を target にしたが、ステージ定義「OK target は実装・テスト側の実在ファイル 1 本」に
  合わせてコンダクタが `.github/workflows/ci.yml` へ差し替えた。BR5.2 の前半（契約 / 差分再生 / ITF / Quint / lint を機械実行する）は `ci.yml` で
  検収できるが、後半（実測の証拠がある範囲だけ報告する）は記録規律であり、証跡は `code-summary.md` §1.3 / §6 にしかない。
  **裁定案**: target は `ci.yml` のまま、functional-design ゲートで `rules.md` の BR5.2 を (i) 機械実行の義務と (ii) 報告規律に分割する提案を
  載せる（分割すれば (ii) は traceability の対象外＝story-map 備考へ）。折り戻し先: `../functional-design/pending-revision.md`（新規項目 16）。
- **NFR1.2**（R-04）: target `formal/orchestration/journal_protocol.qnt` は「ロック協定を置き換えた側」であり、合格基準（退役語彙の grep 0 件、
  `ls formal/orchestration/` = 3 モデル）という**不在**の検収手段ではない。**裁定案**: target を、3 モデルだけを typecheck / 検査対象として
  列挙する `scripts/quint-gate.sh` に寄せる（不在の検収を機械的に担う実在ファイル）。折り戻し先: 本ステージのゲート（traceability は凍結）。
- **BR1.4**: target は U4 所有の `modules/core/read-model-updater/src/orchestration/journal_reader.rs`。BR1.4 自身が「U4 所有の現行 trait に従う」と
  定めるため妥当（レビューも異議なし）。変更なし。

## 5. ステージ定義の MUST の免除を明示する（レビュー R-03）

ステージ定義 `code-generation.md:143-147` は計画に「テストファイル」「テスト構成」のステップを無条件で要求する。本計画 §4 は「新規プロダクション
コード・新規テストが無く、変更は振る舞いを持たないコメント 5 行のため、Red / Green を架空に実行しない」と宣言し、Plan Approval（質問票 P5、
`Approve Plan` 2026-09-07T07:14:34Z）で人間がそれを承認した。レビューはこの読み替え自体を妥当としつつ、免除を暗黙にしないよう求めている。

**ゲートでの提示文**: 「U3 code-generation（b52）はワークスペース変更が Quint 凡例コメント 5 行のみで新規プロダクションコード 0 のため、ステージ
定義 `:143` のテストファイル・テスト構成ステップを免除した（既存スイート 11 本 + quint-gate + coverage で受入）。この免除を承認する」。
承認されたら本ファイルに裁定日時を追記する。質問票・計画は承認指紋の対象のため事後に書き換えない。

## 6. 記録の是正（コンダクタ自身の誤り）

- 計画 §3 Step 6 の `source-manifest.json` の例示 `"writes":["formal/…"]`（文字列要素）は strict schema と合わず、レビュー要求が
  「writes[0] must be an object」で拒否された。`{ "path": "formal/orchestration/journal_protocol.qnt" }` に是正して要求し直した
  （REVIEW_REQUESTED 2026-09-07T07:46:29Z）。計画本文は凍結のため未訂正。適用文面: Step 6 の例示をオブジェクト形式へ。
- レビュー要求の直後（07:46:34Z）に `code-summary.md` §8 へ 1 行足してしまい、凍結基準（要求時のバイト）と食い違ったため、同じ Edit を逆適用して
  要求時のバイトへ戻した（`sha256:042d5bc6…` 一致を確認）。判定記録（REVIEW_COMPLETED）は機械検証を通過している。

## 7. 教訓（§13 の学習候補）

- **n 件の主張は実行フィルタと同じ範囲で数える**: 属性を 1 ファイルで数えて「29」と書き、`--lib orchestration::dto` の実行 47 と食い違った。
  件数はコマンドを 1 回流して書く。
- **行範囲は「開始行〜閉じ括弧」で統一して書く**: 同じ表の中で基準が揺れると、不一致の判定自体が信用できなくなる（R-01）。
- **REVIEW_REQUESTED の後は記録ディレクトリ内でも成果物を触らない**: 要求時のバイトが凍結基準。直したいことは pending-revision に書く。
- **source-manifest の `writes` はオブジェクト要素**: `{ "path": "…" }`。計画の例示から正しい形にしておく。
