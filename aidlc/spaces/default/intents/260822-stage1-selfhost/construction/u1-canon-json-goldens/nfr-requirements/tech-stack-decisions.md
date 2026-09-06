# tech-stack-decisions — U1 正準JSONとゴールデン

> 出典: `../functional-design/functional-spec.md`、`../functional-design/rules.md`、
> `../../../inception/requirements-analysis/requirements.md`、`../../../inception/contract-design/contract-summary.md`（C7）、
> `nfr-requirements-questions.md` の2026-09-06確認済み要約。
> 現行の `Cargo.toml`、`modules/core/infrastructure/Cargo.toml`、`clippy.toml` と実装を根拠にする。
> ADR 0001の初期クレート配置・採取予定の記述は履歴として扱い、ここでは現行の機能設計と確定済みC7を適用する。

## 1. 選定と境界

| 領域 | 選定 | 理由・制約 |
|---|---|---|
| 配置 | `core-infrastructure::canon_json`（`modules/core/infrastructure/src/canon_json/`） | ドメインに依存しない変換部品。旧shared/canon-jsonへ戻す別名や互換ラッパーを作らない |
| 読取 | serde + serde_json、`preserve_order` と `float_roundtrip` をworkspaceで有効化 | 挿入順と浮動小数の読取精度を保持する。深さ・UTF-8の境界はsecurity-requirements NFR4.3に従う |
| 型付き値の変換 | `to_value` → `Result<JsonValue, ToValueError>` | JSONキーにできない型等の変換失敗を伝播する。型付きDTOのデシリアライズはアダプタで行える |
| 書出し | 既存の専用ライタ、3プロファイル | すべてのオブジェクトで整数形式キーが数値順で先頭。残りはhash-canonicalでUTF-16順、それ以外は宣言・挿入順。汎用フォーマッタへの置換は受入バイトを変える |
| 数値 | 非負整数u64、負整数i64、浮動小数f64 | 保持型と出力表記を分ける。2^53超の整数はf64へ丸めてJS互換出力。非有限数はnull、負ゼロは0。任意値の完全往復を約束しない |
| ハッシュ | sha2のSHA-256、canonical-prefixed / compact-raw | W2の用途表どおり使う。暗号ライブラリの安全性を知名度で断言せず、固定依存と検査結果で評価する。認証用の署名ではない |
| 性質検証 | 既存proptest、固定シード20260823 | 決定性と対象値域での出力安定性を検証する。正準化前後の値の完全一致や再ハッシュの冪等性とは区別する |
| 採取 | `scripts/goldens/` の既存スクリプト、bun | ピン3c3146cfから採取する。bunは開発時ツール。採取時のバージョンは来歴へ記録する |
| 保存先 | `tests/golden/upstream-3c3146cf/` | C7の確定配置。hash-canonical/cases.jsonのexpectedに3プロファイルの出力とハッシュを保持し、各familyのprovenance.json・cases-missing.jsonで来歴と欠落を管理する |
| 機械強制 | clippy disallowed-methods | 契約JSONの直列化関数群・型付き値のto_valueをcanon_jsonへ集約する。変換内部や契約外の永続化DTO等の例外は理由付きで局所化し、クレート全体を免除しない |
| その他のJSON方式 | 汎用JSON/JCSライブラリへの置換は行わない | upstreamの受入出力・数値・キー順が検収基準であり、別方式の標準準拠だけでは代替にならない |

## 2. 現在の依存と品質設定

canon_jsonが使うランタイム依存はserde・serde_json・sha2で、追加は不要。所属クレートには他機能のためlibc・hmac・base64等もあるが、このモジュールの責務とは区別する。proptestとゴールデン比較用regexは開発依存である。採用バージョンの正本はCargo.lockで、過去のCodeKBの番号を現行版として転載しない。

workspaceのunsafe_code forbid、rust-toolchain.toml、CIの最小権限とcargo auditを維持する。外部クレート内部までunsafe不使用とは主張しない。カバレッジは90%床・相対差0.01ポイント、固定シードはscripts/coverage.shとCIで揃える。依存更新時は両ロックファイルを検査し、その実行結果を根拠として扱う。

## 3. 検証と今後の扱い

preserve_orderとfloat_roundtripは既に有効。正準JSON87試験とgolden16試験の成功を確認済みだが、この文書更新で最新のcargo auditや全CLI経路検査を実行したとはしない。採取済みhash-canonicalの来歴はbun 1.3.13を記録する。再採取時は実際に使ったバージョンとコマンドを記録する。

入力の実測範囲・拒否境界・巨大入力に残る制約はsecurity-requirements §4に示す。性能の数値目標は追加せず、観測された劣化を測定してintent記録へ残す。既存のピンや期待値の更新、依存・値モデルの拡張は、それぞれの変更理由と検証を必要とする。
