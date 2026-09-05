# amadeus-lint

`cargo lint` は `modules/` の Rust ソースを検査し、所見または読み取り・構文エラーがあれば終了コード 1 を返す。

## use-case-domain-getter

`modules/` 配下の `/use-case/src/` にある実装から、`/domain/src/` の型が保持データを公開する getter を呼ぶことを禁止する。command/query の両方が対象。getter 定義自体と interface-adapter、RMU、アプリの呼出しは対象外。

getter は名前の禁止リストではなく実装から分類する。直接フィールド、参照・複製、標準の借用変換、単純なローカル束縛、下位 getter への委譲、コレクションの取得と getter への射影、enum 各変種の保持値を返す match を検出する。比較・分類・優先順位解決などの判断を加えるクエリや、状態変更のコマンドは含めない。`is_` で始まっていても bool フィールドを直接返すなら getter である。

ファイル間の use/import、再公開、import の別名、型エイリアス、型注釈、ジェネリックの単一ポート境界、フィールド、メソッド契約から受信者の型を復元する。Repository の戻り値の `await`・`?`、Result 内の tuple 分配、ローカル別名、参照、clone、直接・UFCS 呼出し、既知の Vec/Option/Result の操作を追跡する。既知のコレクションの iterator closure 内も検査する。無関係な型の同名メソッドを、名前だけでドメインの getter とみなさない。

通常の関数、impl、trait の default メソッドを検査する。テストパス、`#[cfg(test)]` の項目、テスト用として宣言された外部モジュールファイルは対象外。`#[cfg(not(test))]` は対象。診断直前行の理由付き `// amadeus-lint: allow(use-case-domain-getter) — 理由` で個別に抑制できる。理由のない allow は認めない。

### 静的解析の限界

これは rustc の型検査器ではない。マクロ展開・関数ポインタ・動的 dispatch・関連型・外部 crate の契約・複雑な制御フローや型パターンは復元しない。trait を明示した `<T as Trait>::method`、glob による再公開、Cargo で別名を付けた依存や独自の lib 名、`#[path]` によるモジュール配置も解決しない。crate 名は `modules/` から `src/` までの配置（区切りとハイフンをアンダースコアへ置換）に対応する本リポジトリの規約を使用する。

保持データの射影は許可した構文の範囲で分類する。任意の処理を挟んだ getter、名前付き enum フィールドの分配などは検出できない場合がある。型を確定できない呼出しは推測で所見にせず、レビューの対象として残す。したがって所見がないことは、全 getter 呼出しの不存在を証明しない。

## 検査

```sh
cargo test --manifest-path tools/lint/Cargo.toml
cargo fmt --manifest-path tools/lint/Cargo.toml -- --check
cargo clippy --manifest-path tools/lint/Cargo.toml --all-targets -- -D warnings
cargo lint
```

ドメイン getter の索引、名前解決、式の型復元、getter 本体分類、呼出し検査を `src/domain_getter/` の別モジュールに分けている。既存の単一ファイル検査 `check_source` とそのテストは維持する。複数ファイル検査の結果は実 CLI で合流する。

## 導入時の検出結果（2026-09-05）

リンター自身の93テスト、fmt、clippyは成功。同じgetter呼出しを含む一時フィクスチャを実CLIへ渡し、
use-case配置では終了コード1、interface-adapter配置では終了コード0を確認した。

導入時は539ファイルを走査し、既存違反24件を検出して終了コード1になった。
以下は是正前の内訳であり、後続の修正で全24件を解消した。現在は終了コード0になる。
以下のファイルはすべて `modules/core/command/use-case/src/orchestration/` 配下。

| ファイル | 件数 |
|---|---:|
| commit_verdict_use_case.rs | 11 |
| create_intent_use_case.rs | 1 |
| park_use_case.rs | 1 |
| promote_practices_use_case.rs | 1 |
| record_review_use_case.rs | 4 |
| record_single_stage_run_use_case.rs | 2 |
| record_skeleton_stance_use_case.rs | 3 |
| switch_autonomy_use_case.rs | 1 |

getterの呼出しをユースケース内の別関数に移す、取得用メソッドへ改名する、理由付きallowを一括追加する、といった回避は是正と扱わない。

是正では、再試行対象の固定を `ReportRequest::for_retry_at`、レビュー方針を
`Intent::resolve_review_policy`、報告の適用を `IntentExecution::apply_report`、
名指しの隔離実行を `record_single_stage_run_named` へ任せた。報告判断・コマンド拒否は、
応答に必要な文脈もドメイン側で生成する。関連参照の取得はRepositoryへ依頼し、
IDを読む処理はinterface-adapter側に置いた。検出ルールと例外設定は緩めていない。
