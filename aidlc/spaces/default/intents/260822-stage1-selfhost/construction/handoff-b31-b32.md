# park ハンドオフ — b31 途中・b32 計画済み(2026-09-01)

オーナー指示 `park` による一時停止の記録。再開(unpark)時はこのファイルから読み始める。

## 現在地

- **main**: `1ce3e389`(b30 マージ済み — 定義リポジトリの ES 全面転換・dto 語彙統一)。オープン中の PR なし
- **作業ブランチ**: `bolt/b31-receiver-names`(push 済み・PR 未作成)。WIP は全ゲート緑
  (fmt / clippy 0 / cargo lint 0 / **1449 tests**)で park 時点コミットに保全済み

## b31(受け手変数名の省略全廃)— 途中

**裁定**(オーナー 2026-09-01): ポート/リポジトリ/DAO/クライアントの受け手名(フィールド・
引数・ローカル)は型のドメイン名の**完全な snake_case**。先頭語の切り詰め
(`IntentExecutionRepository` → `execution_repository` 等)は全廃。

### 完了済み(park 時点コミットに含まれる)

- プロダクション全面の是正(use-case 構造体・DaoImpl・runtime 配線 — 17ファイル +519/-211)。
  例: `execution_repository` → `intent_execution_repository`、`state_dao` → `execution_state_dao`、
  `definition_dao` → `workflow_definition_dao`
- canon 同期: `use-case-rules.md` §2 の `CommitVerdictUseCase` 例示を実物と逐語一致へ
- 全数棚卸し(子エージェント実測): **残る違反はテストコード限局の6パターン・約90行**

### 未着手(再開後の残作業)

1. **A 群 約90行の修正**(すべてテストコード):
   - `reader` → `journal_reader`(JournalReader 束縛、~60行 — RMU 自クレートテストと app 統合テストに集中)
   - `repository` → `intent_execution_repository` / `intent_repository` / `workflow_definition_repository`(具象型直受けの箇所)
   - `executions` / `intents` / `definitions` → 各完全形(ストア同居系テスト3ファイル)
   - `client` → `definition_artifacts_client`、`dao` → `workflow_definition_dao` / `execution_state_dao`
   - 注意: Reader 側に `type Reader =` エイリアスを**新設して字面を合わせる迂回は禁止**(裁定済み — 素直に改名)
2. **B 群は存置で確定**(判定理由ごと最終報告・PR 記述へ転載): B-1 ジェネリック単一型束縛の
   総称 `repository` / B-2 associated type が文字どおり `Repository` の契約基盤 /
   B-3 ローカル `type Repository =` エイリアス経由 / B-4 値・DTO 型の束縛
   (集約値の束縛 `definition: &WorkflowDefinition` 等は**スコープ外・別裁定** — 件数報告のみ)
3. 受け入れグレップ(word-boundary・B 群除外判定つき)+ 4ゲート + coverage 相対 PASS
4. 統合(コミット→PR→収束→merge queue)は team-lead セッションの手順どおり

担当エージェント `b31-names` は park 指示済み(停止・ツリー残置)。

## b32(1ファイル1公開型)— 計画済み・未着手

**裁定**(オーナー 2026-09-01、3点):

1. **1ファイル = 1公開型**(`pub struct` / `pub enum` / `pub trait`)— Java 形式。
   ファイル名は型名の snake_case。同居可: private 補助型・`pub type` エイリアス・
   主題型に仕える自由関数・`pub(crate)` 以下(package-private 相当)
2. **非公開型は従属する公開型のファイルに納める**(private 型だけの孤立ファイルを作らない)。
   複数公開型に共有される private 型は主従属先へ寄せるか公開昇格を個別判定。
   公開型ゼロの自由関数モジュール(`codec.rs` 等 — 裁定済みの free function 化)は正当
3. **リンターで機械強制**: `cargo lint` 新ルール(仮名 `one-public-type`)—
   トップレベル公開型2つ以上を検出、抑制は共通規約(理由必須)、**赤例テスト必須**。
   canon 着手条件③により**検出と74ファイルの是正を同一 Bolt で着地**

**実測**: 違反は **74ファイル**(最大: イベント族 enum+変種ペイロードで12公開型/1ファイル、
`stage_node` 9、query `directive` 8)。イベント族はディレクトリ化(enum 1+変種各1ファイル)。
canon 記録先: abstract-data-type(「1ファイル1型」を domain ADT 限定から全層公開型へ拡張)・
module-visibility・README 規則表/機械化ロードマップ。

## 積み残し Issue(変わらず)

#70 intents.json 投影 / #71 実走査(遡及不能論点コメント済み) / #72 set-autonomy 一括 /
#73 report ガード完全化 / #74 park 本体(Quint モデル照合含む) / **#76 誕生時乖離(要オーナー裁定)** /
**#77 intent-create 原子性(要オーナー裁定)** / 小粒: rustdoc 宙づり・`.coderabbit.yaml`・
StateFileWrite の Debug 表示(PR #78 コメントに記録)

## 再開手順

1. `bolt/b31-receiver-names` を checkout し本ファイルと park 時点コミットを確認
2. b31 残作業(上記 A 群)を実施 → ゲート → PR → 収束 → merge
3. b32 に着手(上記裁定3点+74ファイル棚卸しから)
