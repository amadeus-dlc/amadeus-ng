# security-design — U1 正準JSONとゴールデン

> NFR Design（Unit: u1-canon-json-goldens、kind: library）。出典:
> `../nfr-requirements/security-requirements.md`、`../nfr-requirements/tech-stack-decisions.md`、
> `../functional-design/functional-spec.md`、`../../../inception/contract-design/contract-summary.md`（C1/C7）。
> `nfr-design-questions.md` の2026-09-06確認済み要約に基づく。
> 論理配置は `logical-components.md`、入力実測は `../nfr-input-measurements.json`。

## 1. 設計方針

純粋な変換処理を `core-infrastructure::canon_json` に置く。検証を入力経路ごとに明示し、決定的な出力を受入表で確認する。ネットワーク・認証・永続化・クラウド資源は導入しない。失敗を返すこととプロセス障害を隔離することを区別する。

## 2. 入力経路と検証（NFR4.3）

| 経路 | 処理 | 通常の失敗 |
|---|---|---|
| parse(&str) | 文字列リテラル内とエスケープを区別して深さを事前検査し、JSONを読んで挿入順を保持する | 不正構文・孤立サロゲートはSyntax、128段目に達した入力はTooDeep |
| parse_bytes(&[u8]) | UTF-8として検査し、成功後にparseへ委譲する | 不正UTF-8はEncoding、以降はparseと同じ |
| to_value(&T) | 型付き値をJSON値へ変換し、変換不能な型を拒否する | ToValueError |
| JsonValueの直接構築 | 型の値域に従って構築する | parseの構文・深さ検査を通ったとは扱わない |

ParseErrorはSyntax { offset, detail }、TooDeep { limit }、Encodingを持つ。Syntaxのoffsetはバイト位置。parseの引数は既にUTF-8の&strなので、その経路からEncodingが生じるとは記載しない。to_valueは深さ制限を代替せず、プログラム構築値の任意の深さまで保護するとはしない。

MAX_DEPTHは128、受入最大段数は127。文字列外のオブジェクト・配列を1段として数え、128段目でTooDeep { limit: 128 }を返す。上限を入力に応じて自動で引き上げたり、serde_jsonの再帰制限を無効化したりしない。

採取済みJSON88ファイルとJSON文字列入力29ケースは最大深さ7だった（測定対象・方法は `../nfr-input-measurements.json`）。JavaScript構築の3ケースをこの測定へ含めない。これは採取済み入力が上限内である証拠であり、将来の全入力や巨大な平坦入力のメモリ安全性を保証するものではない。

## 3. 出力とハッシュの選択（NFR1.1〜NFR1.3）

同じJsonValue・同じプロファイルから同じバイト列を生成する。全プロファイルで整数形式キーを数値昇順で先頭に置き、残りだけをhash-canonicalでUTF-16順にする。大整数のJS互換丸め、負ゼロ、非有限数、最小エスケープは機能設計BR1.1〜BR1.6に従う。

DigestFamilyはダイジェスト族の識別情報を運ぶ。利用者が誤った関数を選ぶことまで型だけで防げるとはしない。W2の用途表と族別テストを併用する。

| 用途 | 呼出と出力 |
|---|---|
| contract_sha256・approval fingerprint | hash_canonical、sha256:接頭辞付き |
| bundle hash・directiveHash・route hash・配送冪等digest | hash_compact、生hex |

これらのハッシュは内容比較の材料であり、送信者の認証や改変防止の署名ではない。採取済み32行の3プロファイル・2族を比較する。CLI/フックの実行結果比較は後続Unitが担い、コーパス読取の成功を全経路検証に読み替えない。

## 4. 依存と直列化境界（NFR4.1・NFR4.2）

canon_jsonが使用するランタイム依存はserde・serde_json・sha2。serde_jsonはpreserve_orderとfloat_roundtripを有効にする。所属クレート内の別機能の依存と、canon_json自身の依存を区別する。共有components.mdのCanonJson.external_dependenciesも現在この3依存を記載している。

契約JSONの直列化関数群とto_valueの直接呼出しをclippy disallowed-methodsで制限する。変換内部・契約外の永続化DTO等に必要な例外は理由付きの局所許可とし、クレート全体を免除しない。型付きDTOのデシリアライズはアダプタの責務として残す。

Cargo.lock、workspace unsafe_code forbid、rust-toolchain.toml、CI最小権限を維持し、workspaceとtools/lintの依存検査を実行する。外部クレート内部までunsafe不使用とは保証しない。最新の検査成功は、実行結果がある場合だけ記録する。bunは採取用の開発時ツールである。

## 5. 採取物と情報の扱い（NFR4.4・NFR1.3）

使い捨て環境で採取し、コーパスの規則に従って時刻・clone・root・sessionを期待値と実測値の双方で正規化する。未加工値と正規化後の差分を確認し、機能差を隠す置換を追加しない。

入力・出力・監査・来歴を含めて秘密情報・個人情報・環境固有値の残存を点検する。正規化だけを情報不在の証明にしない。来歴にはピン・日時・実行コマンド・ツールバージョンを残す。採取失敗や未採取は理由付きで記録し、期待値を捏造しない。ライブラリ自身はログを書かず、診断情報の表示・保存は呼出側が扱う。

## 6. 失敗・品質・性能（NFR2.x・NFR5.1）

読取と型付き変換の通常エラーはResultで呼出側へ返し、同じ入力を自動で再試行しない。serializeとhashは構築済み値に対する計算であり、非有限数のnull化は仕様上の変換である。割当失敗や深い直接構築値によるプロセス障害までResultで回収・隔離できるとはしない。

新規コードはレイヤーごとのTDD、固定シードのPBT、採取済み受入表で検証する。カバレッジ90%床・相対差0.01ポイントと必須CIを維持する。任意値の完全往復や再ハッシュの冪等性ではなく、対象値域での出力安定性と同一入力の決定性を検証する。

性能目標を新設せず、劣化が観測された場合は入力・実行環境・比較対象・計測方法を記録して測定する。試験の実行時間を性能ベンチマークの代用にしない。

## 7. 要求への対応

| 要求 | 設計上の手当て |
|---|---|
| NFR1.1 | §2の入力境界、§3の3プロファイルと32行比較 |
| NFR1.2 | §3の族の識別・用途表・族別テスト |
| NFR1.3 | §3の決定性、§5の規則限定の正規化と欠落記録 |
| NFR2.1 | §6のレイヤーごとのTDD、logical-components §4の検証配置 |
| NFR2.2 | §6のカバレッジとCI基準 |
| NFR2.3 | §6の対象値域を限定した性質検証 |
| NFR4.1 | §4の依存・ロックファイル・依存検査 |
| NFR4.2 | §4のunsafe forbid・ツールチェーン・CI権限 |
| NFR4.3 | §2の入力経路別検証と127/128段の境界 |
| NFR4.4 | §5の採取物全体の点検・来歴 |
| NFR5.1 | §6の劣化時の測定と記録 |


## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-06T01:29:31Z
**Iteration:** 1
**Request Challenge:** review:40f7ea78ddb26fa8228a31d8ab87abb8

### Findings

1回のADVISORYレビューとして既存IDを引き継いだ。既存2件は解消済みで、新規所見はない。

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Minor | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u1-canon-json-goldens/nfr-design/security-design.md > 第4節、および aidlc/spaces/default/intents/260822-stage1-selfhost/inception/domain-design/components.md > CanonJson.external_dependencies | 共有コンポーネント表はserde・serde_json（preserve_order、float_roundtrip）・sha2を列挙済み。設計第4節と現行Cargo.tomlの依存用途に一致し、内部依存depends_on=[]との区別も維持されている。更新対象の記載漏れという旧所見は、上流の更新自体が完了したため解消している。 | 追加対応なし。依存変更時は共有表と実装の対応を維持する。 | Resolved |
| R-02 | Minor | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u1-canon-json-goldens/nfr-design/security-design.md > 第4節「契約JSONの直列化関数群とto_value」 | to_valueの直接呼出しも制限対象として明記された。functional-spec第2節・tech-stack-decisionsの機械強制欄と一致し、clippy.tomlにはserde_json::to_valueの禁止、value/json_value.rsには変換点の理由付き局所許可が実在する。 | 追加対応なし。契約外の例外も理由付きの局所許可に限定する。 | Resolved |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| aidlc-sensor-required-sections（stage=nfr-design、security-design.md / logical-components.md） | PASS、追記前H2数7 / 5、findings_count=0 | 指定された2設計文書の構造検査を実行した。 |
| aidlc-sensor-upstream-coverage（consumes=security-requirements,tech-stack-decisions,functional-spec,contract-summary、deliverables=security-design,logical-components） | PASS、unreferenced=[]、findings_count=0 | scanned_filesも指定2文書に限定されている。 |
| aidlc-sensor-traceability（U1 nfr-design/traceability.json） | PASS、findings_count=0 | 全11件の要求が列挙され、対応する設計節に解決する。 |
| C1・C7・CanonJsonの共有契約との照合 | 一致 | ハッシュの用途は機能設計W2、コーパスの配置・ピン・所有・更新方針はC7に対応する。後続UnitのCLI実行比較とU1の比較器を区別している。 |
| mod.rs・parse.rs・canonical.rs・digest.rs・value/json_value.rsの限定照合 | 一致 | 公開面、UTF-8検査からparseへの委譲、127段受理・128段拒否、to_valueの変換失敗、全プロファイルの整数形式キー優先、2族の計算経路が設計と一致する。プログラム構築値やプロセス障害まで深さ検査・Resultで保護するとはしていない。 |
| JSON入力の独立再計算（Python、固定コーパス内のみ） | 記録と全88ファイル一致 | 各ファイルのパス・バイト数・深さがnfr-input-measurements.jsonと一致し、最大深さ7。受入表32行のうちJSON文字列入力29件も最大深さ7で、構築入力3件は別扱いになる。 |
| 既存実行ログ（/tmp/u1-resume-unit-tests.log、/tmp/u1-resume-golden-tests.log） | 87 + 16 passed、0 failed | 同セッションのログを確認。golden_hash_canonical.rsは3プロファイル・2族の全行比較を実装している。コード未変更のため再試験は行っていない。 |
| Cargo.toml・クレート依存・clippy.toml・rust-toolchain.toml・CI・coverage設定 | 本文の設定記述と一致 | serde_jsonの2機能、unsafe forbid継承、Rust 1.95.0固定、contents: read、両ロックファイルのcargo audit実行設定、90%床・相対差0.01・シード20260823を確認。現在のCI成功・最新依存検査成功を証明するものではない。 |
| linter / type-check | 対象外 | 対象2文書にはTypeScript/JavaScriptのコード片がない。 |

### Summary

確認済み要件に沿って、入力経路ごとの検証、純粋な変換処理の配置、失敗伝播と隔離の限界、品質検証を具体化している。既存2件は解消され、未実行の最新脆弱性検査・性能測定・全CLI経路検証を成功扱いしていないためREADYとする。
