# code-summary — U1 正準JSONのコメントと実装記録の是正

> Unit: u1-canon-json-goldens。実施日: 2026-09-06。
> 承認済み `code-generation-plan.md` のStep 1〜6と `unit-test-instructions.md` に従う。
> Testing Contract: `sha256:303d9bb7b5d777d54a6761be9ed154d85d5bb3f2d6b9cce02f71f4ed1b3a4ff3`。
> 設計入力: `../functional-design/{functional-spec,rules,entities}.md`、
> `../nfr-requirements/{security-requirements,tech-stack-decisions}.md`、
> `../nfr-design/{security-design,logical-components}.md`。
> 上流の割当と契約は同intentのinceptionにあるrequirements、unit-of-work、
> unit-of-work-story-map、components、contract-summaryのU1・FR7・C7を参照した。

## 1. 今回の変更

既存の `core_infrastructure::canon_json` と固定コーパスを再利用し、説明コメントと実装記録を是正した。新規の実行時コード、API、テスト、依存、固定データは作成していない。実行時のエラーメッセージも維持した。

| 変更ファイル | 内容 |
|---|---|
| modules/core/infrastructure/src/canon_json/mod.rs | 契約JSONの直列化・型付き値の変換を本モジュールへ集約する説明へ修正。型付きDTOの読取境界を区別し、依存3種、入力経路ごとの検証と互換範囲を明示 |
| modules/core/infrastructure/src/canon_json/parse.rs | テキスト・バイト列の読取境界、127段受理・128段目で拒否、Encodingが生じる経路を明示。旧message-catalogの説明を呼出側の文言組立へ修正 |
| 本ディレクトリのcode-summary.md | 現行配置、再利用、今回の検証、未検証範囲を記録 |
| 本ディレクトリのcode-summary-history-2026-08-22.md | 更新前のcode-summary全文を履歴として保存。過去のReviewも保持 |
| 本ディレクトリのtraceability.json | FR7親子4件、BR13件、詳細NFR11件の計28件を列挙し、旧配置を現行ファイルへ対応付け |
| 本ディレクトリのsource-manifest.json | 実際に変更したアプリケーション側2ファイルだけを列挙 |
| 本ディレクトリのcode-generation-plan.md | 作業完了チェックのみ更新 |

影響は上記2ソースの説明コメントと記録に限る。既存の17項目の公開面、rustdoc例の実行部分、値モデル、直列化・ハッシュ・読取の動作は変えていない。別名や後方互換ラッパーも追加していない。

## 2. 再利用した実装と要求の対応

以下のソース相対パスは `modules/core/infrastructure/src/canon_json/` を基準とする。これらを今回新規作成したものとして扱わない。

| 対応 | 現行ファイルと確認事項 |
|---|---|
| BR1.1・BR1.2 | canonical.rsのmember_order。整数形式キー0〜2^32-2を全プロファイルで数値順に先頭へ置き、残りは契約用で挿入順、正準用でUTF-16コード単位順。01・-1・4294967295は整数形式キーから除外 |
| BR1.3 | value/number.rsとwriter.rs。保持型はPosInt(u64)・NegInt(i64)・Float(f64)。出力時は絶対値2^53超の整数をf64へ丸め、負ゼロは0、非有限数はnull、指数閾値は1e21・1e-6 |
| BR1.4 | writer.rsの最小エスケープとparse.rsの拒否試験。UTF-8文字列を扱い、孤立サロゲートはSyntax。任意のJS UTF-16文字列への完全互換とはしない |
| BR1.5 | writer.rsとprofile/。prettyは2スペースと末尾改行、compact・canonicalは空白なし。空コンテナは1行 |
| BR1.6・NFR1.2 | digest.rsとdigest_family.rs。hash_canonicalは正準出力のSHA-256にsha256:接頭辞、hash_compactはcompact出力の生hex。contract_sha256・approval fingerprintは前者、bundle・directiveHash・route・配送冪等digestは後者 |
| BR1.7 | clippy.tomlがserde_jsonのto_string・to_vec・to_writer系列とto_valueを禁止。value/json_value.rsの変換点は理由付き局所allow。型付きDTOのデシリアライズや契約外の永続化表現は別の境界 |
| BR1.8・NFR4.1 | Cargo.tomlのpreserve_order・float_roundtripを保持。canon_jsonのランタイム依存はserde・serde_json・sha2で、所属クレートの別機能の依存とは区別 |
| NFR4.3 | parse.rsのparse / parse_bytes、value/json_value.rsのto_value。構文・UTF-8・深さ・型付き変換失敗を経路別に確認 |
| NFR2.3 | writer.rs・parse.rs・digest.rsの既存PBT11件。固定シードで決定性と対象値域の出力安定性を検証 |
| FR7.1・FR7.3・BR2.3・NFR1.1 | modules/core/infrastructure/tests/golden_hash_canonical.rs。採取済み32行の3プロファイル出力・2族ハッシュを全行比較 |
| FR7.2・BR2.2・BR2.4・NFR1.3 | 同クレートtests/golden_corpus_read.rsとtests/support/mod.rs。コーパス読取・正規化・差分表示・代表ケース・欠落理由を確認 |
| BR2.1・BR2.5・NFR4.4 | scripts/goldens/とtests/golden/upstream-3c3146cf/の既存採取手順・来歴・変更方針。今回再採取や期待値更新はしていない |

重複キーについて、既存試験 `duplicate_keys_are_last_wins_at_the_first_position` は最後の値と最初の位置を固定している。機能設計の記載不足R-08は既知のMinorとしてそのまま引き継ぎ、凍結済みの設計・Reviewへ変更を加えていない。

## 3. 入力経路と保証の限界

- `parse(&str)` は深さを事前走査してからserde_jsonで読む。文字列外のオブジェクト・配列を1段と数え、127段まで受理し、128段目でTooDeepを返す。不正構文・孤立サロゲートはSyntaxで、offsetはバイト位置。
- `parse_bytes(&[u8])` は不正UTF-8をEncodingで拒否し、成功後にparseへ委譲する。UTF-8の&strを受けるparseからEncodingは返さない。
- `to_value(&T)` は変換不能な複合型キー等をToValueErrorとして返す。parseの深さ検査は通らず、JsonValueの直接構築にもこの検査は適用されない。
- 入力バイト数の上限はなく、巨大な平坦入力や深い構築値のメモリ・スタック枯渇まで保証しない。
- 大整数丸め・非有限数のnull化・キー整列があるため、任意値の完全往復やハッシュへの再ハッシュの冪等性は要求しない。DigestFamilyの識別も、呼出側の関数選択ミスまで防ぐ保証ではない。

## 4. 固定コーパスと入力測定

固定ピンは `3c3146cfd7cef33020d48e8d48d4e80d0f8c2820`。保存先はC7の `tests/golden/upstream-3c3146cf/` を維持した。

- hash-canonicalは32行。provenance.jsonは2026-08-22T13:07:22Z・bun 1.3.13・採取コマンド・ソースと抽出スニペットのSHA-256を記録し、missing_casesは空。
- 現行CLIは28ケース、フックは14ケース。両族の来歴は2026-08-29T08:12:42Z・bun 1.3.13。旧記録のCLI22件を現在の件数として転載していない。
- CLIの未採取はset-autonomy/gatedとcontinue/multi-part、フックはstop-forwarding-loop/transcript-carve-out。それぞれcases-missing.jsonに理由・証拠・後続対応がある。
- 正規化は固定normalization.jsonの4プレースホルダ（時刻・clone・root・session）を使用する。期待値と実測値に同じ規則を適用する設計と、既存比較器の実装を確認した。

2026-09-06、Pythonで同コーパス配下のJSON88ファイルを再計数し、`../nfr-input-measurements.json` の各パス・バイト数・深さと全件一致した。最大深さ7、最大ファイルstage-graph.jsonは81,850バイト・深さ4、scope-grid.jsonは13,509バイト・深さ3。32行中のJSON文字列入力29件も最大深さ7で、JavaScript構築3件は別扱いである。この測定は将来の全入力の上限を定めない。

NFR4.4の限定確認として、入力・出力・監査・来歴を含む全320ファイルへ、秘密鍵ヘッダ、AWSアクセスキー形式、GitHubトークン形式、利用者ホームパスのパターン照合を行い、一致は0件だった。既存の正規化試験も成功したが、任意の秘密情報・個人情報が含まれない証明にはしない。新たな採取は実施していない。

## 5. 今回の試験結果

承認済みUnit限定コマンドを、コメント修正後に同じ固定シード20260823で実行した。全コマンドexit 0、合計104件成功、失敗・ignoredとも0。単体・PBTには同クレートの別機能37件を除くcanon_jsonフィルタを適用した。

| コマンド | 成功数 | 完了日時（UTC、ログ更新時刻） | ログ |
|---|---:|---|---|
| `PROPTEST_RNG_SEED=20260823 cargo test --locked -p core-infrastructure --lib canon_json` | 87 | 2026-09-06T02:14:02Z | /tmp/u1-code-unit-after.log |
| `PROPTEST_RNG_SEED=20260823 cargo test --locked -p core-infrastructure --test golden_hash_canonical --test golden_corpus_read` | 7 + 9 | 2026-09-06T02:14:21Z | /tmp/u1-code-golden-after.log |
| `PROPTEST_RNG_SEED=20260823 cargo test --locked -p core-infrastructure --doc canon_json` | 1 | 2026-09-06T02:14:32Z | /tmp/u1-code-doc-after.log |

比較基準として準備時ログを再利用した。/tmp/u1-plan-unit-baseline.logは2026-09-06T01:31:21Zに87件、/tmp/u1-plan-golden-baseline.logは01:31:25Zに9+7件、/tmp/u1-plan-doc-baseline.logは01:31:31Zに1件成功。対象・件数は修正後と一致する。一時ログの保存場所と、履歴として残すこの要約を区別する。

## 6. Testing Contract・品質設定・過去の証跡

新規プロダクションコードがないため、今回は人工的なRedや形式的な新規テストを作っていない（NFR2.1）。今後の振る舞いの変更では承認済み契約どおり、対象レイヤーの失敗試験の出力を記録してから最小実装し、成功中に整理する。

更新前の実装要約は `code-summary-history-2026-08-22.md` に全文保存した。旧shared/canon-json配置、過去のRed/Green記録、旧Reviewと所見は歴史であり、今回の実施や最新の検証結果として扱わない。過去のRedを今回の成功ログから推定していない。旧developer-brief-1.md・developer-brief-2.mdも変更していない。

以下は設定を確認した結果であり、今回それぞれの全検査が成功したという意味ではない。

| 要求 | 維持した設定 | 今回の確認範囲 |
|---|---|---|
| NFR2.2 | scripts/coverage.shの絶対90%床・相対差0.01ポイント・シード20260823、既存除外 | 設定と差分を確認。全体カバレッジ・必須CIは今回未実行 |
| NFR4.1 | Cargo.lock、モジュール依存3種、CIのworkspace / tools/lint両ロックファイルへのcargo audit | 依存用途と実行設定を確認。最新の依存脆弱性検査は未実行 |
| NFR4.2 | workspaceのunsafe_code forbidとクレートの継承、Rust 1.95.0固定、CIのcontents: read | 設定を確認。外部依存内部までunsafe不使用とは保証しない |

## 7. 性能と未検証範囲

NFR5.1は性能の数値目標を追加しない方針として維持する。今回ベンチマークは実施しておらず、試験時間を性能結果に読み替えない。劣化が観測された場合は、対象入力とサイズ・深さ、実行環境とツールの版、比較対象のピンとプロファイル、計測方法・反復条件を記録し、同条件で比較する。結果と影響をintent記録へ残し、必要な変更計画を親セッションへ返す。

全CLI/フック実行経路の比較、新規採取、最新依存検査、全体カバレッジ、性能測定は今回の104件の成功には含まれない。コーパス読取試験は後続Unitの実行出力一致を証明しない。旧Reviewの欠落一覧非空アサート・広いCLONE正規化・フック区分の可読性も、履歴から解消済みと推定しない。本計画はそれらのコード・コーパス変更を含まない。

## 8. 完了と引継ぎ

Step 1〜6を順に実行し、コメント以外の実行コード・エラーメッセージ・公開API・固定期待値・依存・品質閾値に変更がないことを差分で確認した。source-manifestのwritesはmod.rsとparse.rsの2件のみで、再利用ファイルを変更済みとして列挙していない。

Step 1チェック直後、承認ガードが読取用sedとアプリ編集を拒否した。親の指示で完了チェックだけを承認時へ戻すと、同じアプリ編集が成功した。実行順は維持し、全チェックは本体作業完了後にまとめた。ガードの無効化や受領証の作成は行っていない。

機能設計R-08は変更せず引き継ぐ。新たな機能欠陥は今回の限定確認・試験では判明していない。独立レビュー・Unit完了・次工程への移行、commit・push・外部投稿は親セッションが処理する。
