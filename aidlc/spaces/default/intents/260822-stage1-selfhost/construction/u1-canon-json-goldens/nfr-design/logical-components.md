# logical-components — U1 正準JSONとゴールデン

> 出典: `../nfr-requirements/security-requirements.md`、`../nfr-requirements/tech-stack-decisions.md`、
> `../functional-design/functional-spec.md`、`../../../inception/contract-design/contract-summary.md`（C7）、
> `../../../inception/domain-design/components.md`（CanonJson）、
> 2026-09-06確認済み `nfr-design-questions.md`。
> この設計のコンポーネントはプロセス内のモジュールと開発時のテスト支援である。

## 1. 論理構成と現行配置

以下のsrc相対パスは `modules/core/infrastructure/src/canon_json/` を基準とする。モジュールを旧shared/canon-jsonへ戻さない。

| 部品 | 配置 | 責務・境界 |
|---|---|---|
| value | value/ | JsonValue・Number・ObjectMembers、型付き値変換to_valueとToValueError。構築値の表現を所有する |
| profile | profile/ | SerializationProfile・Indent・KeyOrder。体裁とキー順の選択を表す |
| writer | writer.rs | serialize。体裁・数値・文字列を書き、各オブジェクトの並びをcanonicalへ委譲する |
| canonical | canonical.rs | 全プロファイルの整数形式キー優先と、正準プロファイルの残りのUTF-16順を計算する。公開APIにしない |
| digest | digest.rs | Digestとhash_canonical / hash_compact。対応する出力バイトからダイジェストを計算する |
| digest_family | digest_family.rs | DigestFamily。正準・非正準の族を識別する |
| parse | parse.rs | parse / parse_bytes、ParseError、MAX_DEPTH。バイト検証とJSON読取の入口を所有する |
| ファサード | mod.rs | 上記の公開型・関数・定数をpub useで列挙する。内部実装はprivate modに置く |
| コーパス | リポジトリのtests/golden/upstream-3c3146cf/ | C7に従う正解データ・正規化規則・来歴・未採取記録 |
| 比較器 | modules/core/infrastructure/tests/support/mod.rs | コーパス読取・正規化・差分。プロダクトへ含めず、既存の結合テストから利用する |
| 採取スクリプト | scripts/goldens/ | 固定ピンの実行結果を採取する開発時処理 |

公開面は現行mod.rsの列挙を正本とする。Digest / DigestFamily、hash_canonical / hash_compact、MAX_DEPTH / ParseError / parse / parse_bytes、Indent / KeyOrder / SerializationProfile、JsonValue / Number / ObjectMembers / ToValueError / to_value、serializeを公開する。型ごとの定義は現行の個別ファイルに置き、利便目的の別名や互換ラッパーを追加しない。

## 2. 依存と隔離の範囲

canon_jsonはドメイン・アダプタ・外部システムのプロトコルを知らない純粋な変換部品である。serde・serde_json・sha2への依存は持つ。所属クレートの別機能の依存を、このモジュールの依存と混同しない。

利用者はファサードを通じて呼び出す。契約JSONの直列化・型付き値変換の制限はsecurity-design §4のとおり。同じクレート内でも包括免除にしない。ゴールデン比較器はtests配下に分離し、正解データと採取スクリプトは本体のランタイムへ含めない。

状態・ネットワーク接続・ファイルハンドルを変換処理が保持する設計ではない。ただし同一プロセスのメモリやスタックは共有するため、独立した障害隔離境界ではない。

## 3. 失敗の伝播と影響

| 事象 | 検出と伝播 | 限界 |
|---|---|---|
| 不正JSON・孤立サロゲート・深さ超過 | parseがSyntax / TooDeepを返す | 呼出側が処理を中断するかを決める。自動再試行しない |
| 不正UTF-8 | parse_bytesがEncodingを返す | UTF-8の&strを受けるparseとは別の入口 |
| 型付き値の変換不能 | to_valueがToValueErrorを返す | parseの構文検査・深さ検査では代替できない |
| 出力・ダイジェストの不一致 | 受入表・族別テストで検出 | テスト範囲外の入力まで完全互換を証明するものではない |
| 巨大入力・深い直接構築値 | 入力サイズと利用条件を見て評価する | Resultで回収できるとは限らず、プロセス全体に影響し得る |
| 採取物への情報混入・欠落 | 採取物全体の点検と理由付き欠落記録 | 正規化だけを不在証明にしない |

## 4. 品質検証と配置

| 種別 | 配置 | 検証する性質 |
|---|---|---|
| 単体試験 | canon_jsonの各モジュール | 数値・キー順・エスケープ・体裁、各入力経路のエラー、127/128段の境界 |
| PBT | canon_jsonのテストコード | 同一入力・同一プロファイルの決定性、対象値域での再直列化の安定性 |
| 正準化受入表 | modules/core/infrastructure/tests/golden_hash_canonical.rs | 採取済み32行の3プロファイル・2族 |
| コーパス読取 | modules/core/infrastructure/tests/golden_corpus_read.rs | CLI/フックも含む読取・正規化・範囲。全実行経路比較とは区別する |
| 品質・依存検査 | CI、scripts/coverage.sh | 必須CI、90%床・相対差0.01ポイント・固定シード、両ロックファイルの依存検査 |
| 性能劣化の調査 | 劣化が観測された対象経路 | 入力・環境・比較条件を揃えた測定。数値目標は追加しない |

この設計更新ではコードを変更していない。既存103試験の成功ログと入力88ファイルの測定を根拠にし、最新依存検査や全CLI経路の実行を済ませたとは扱わない。

## 5. 他の作業への引継ぎ

新たなクラウド資源・AWS Bedrock・常駐監視は不要。CI側では所属クレートに既存の品質・依存検査を適用する。後続のCLI・フック実装はC7の同じコーパスを使用し、未採取経路を理由付き記録に照らして扱う。成果物の差分はsecurity-designと全11件の要求対応表で確認する。
