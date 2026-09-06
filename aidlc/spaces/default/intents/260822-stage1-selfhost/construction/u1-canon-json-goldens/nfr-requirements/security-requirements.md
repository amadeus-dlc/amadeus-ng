# security-requirements — U1 正準JSONとゴールデン

> NFR Requirements（Unit: u1-canon-json-goldens、kind: library）。出典:
> `../functional-design/functional-spec.md`、`../functional-design/rules.md`、
> `../../../inception/requirements-analysis/requirements.md`、`../../../inception/contract-design/contract-summary.md`（C1/C7）。
> 2026-09-06の確認済み要約 `nfr-requirements-questions.md` に基づく。
> 配置・依存の根拠は `tech-stack-decisions.md`、入力測定は `../nfr-input-measurements.json`。

## 1. 範囲と信頼境界

U1の変換処理は `core-infrastructure::canon_json` に置く。ドメインには依存せず、ネットワーク・認証・認可・永続化を持たない。ゴールデンの採取・保存は開発時の別処理である。「依存なし」は内部ドメインへの依存を指し、外部クレート不使用を意味しない。

入力JSONはリポジトリ内にあっても無条件には信頼せず、構文・文字列表現・深さを検証する。型付きDTOの読取はアダプタ側に残り、契約JSONの直列化と型付き値からの変換はcanon_jsonを通す（BR1.7）。

秘密情報・個人情報をゴールデンへ保存しない。使い捨て環境と正規化は漏洩を減らす手段であり、混入がない証明ではない。入力・出力・監査・来歴を含めて採取物を点検する。JSON文字列が任意の内容を保持できることと、保存してよい内容の規則を区別する。

## 2. 要求と合格基準

| ID | 要求 | 合格基準 | 出典 |
|---|---|---|---|
| NFR1.1 | 確認済みの値域・深さ境界で3プロファイルの出力をupstream受入表に一致させる | 32行の出力文字列・2族のダイジェストが一致する。全プロファイルで整数形式キーを数値順で先頭に配置し、大整数の出力丸めを含める。孤立サロゲート・深さ128以上は読取拒否。CLI/フックの実行結果比較は後続Unitが担う | NFR1、FR7.3、BR1.1〜BR1.6、W3、C7 |
| NFR1.2 | ハッシュの用途と族を固定する | W2の対応表どおり、contract_sha256・approval fingerprintはcanonical-prefixed、bundle・directiveHash・route・配送冪等digestはcompact-raw。族別の比較テストが成功する | NFR1、BR1.6、C1 |
| NFR1.3 | 正規化で機能差を隠さず、期待値と実測値へ同じ規則を適用する | コーパスの規則に限定して時刻・clone・root・sessionを置換し、未加工値と正規化後の差分を確認できる。未採取ケースは理由付きで明示する | NFR1、BR2.1〜BR2.5、C7 |
| NFR2.1 | 新規プロダクションコードをレイヤーごとのred-green-refactorで実装する | 失敗テスト→実装→成功の証跡を残す。採取済み受入表を維持する。今回の文書更新は新規コード実装ではなく、過去のredの実施を今回の成功ログから推定しない | NFR2、team.md Testing Posture |
| NFR2.2 | 必須CIとカバレッジ基準を維持する | 必須チェックが成功し、絶対90%床・相対差0.01ポイントの許容範囲を満たす。許可済みmain.rs配線部以外へ除外を追加しない | NFR2、scripts/coverage.sh |
| NFR2.3 | 固定シードのPBTと受入表で決定性・出力の安定性を検証する | 同一値・同一プロファイルなら同一出力、同一値・同一族なら同一ハッシュ。有限数等の対象値域で直列化→読取→再直列化の出力が安定する。大整数の丸め・非有限数のnull化・順序整列を伴うため、任意値の完全往復やハッシュへの再ハッシュの冪等性は要求しない | NFR2、BR1.1〜BR1.6 |
| NFR4.1 | canon_jsonのランタイム依存は既存のserde・serde_json・sha2とし、依存変更を審査する | モジュールの依存用途とCargo.lockを確認し、CIのworkspaceおよびtools/lintのcargo auditが成功する。クレート全体の他機能の依存と区別する。検査日時・対象がない「既知脆弱性なし」は記載しない | NFR4、確認済み要約 |
| NFR4.2 | workspaceのunsafe_code forbid、ツールチェーン固定、CI最小権限を維持する | Cargo.tomlのworkspace lintを継承し、rust-toolchain.tomlとCI設定に従って検証する。自クレートへのforbidは外部依存内部のunsafe不使用まで保証しない | NFR4 |
| NFR4.3 | JSON読取の不正構文・孤立サロゲート・深すぎるネストを拒否する | オブジェクト・配列を1段として127段を受理、128段以上はTooDeep。不正構文と孤立サロゲートはSyntax。非有限数の出力・制御文字のエスケープ・to_valueの変換失敗も機能設計どおり。深さ制限を巨大入力全般のメモリ上限とみなさない | NFR4、BR1.3/BR1.4、W3 |
| NFR4.4 | ゴールデンへ秘密情報・個人情報・採取者の環境固有値を混入させない | 使い捨て環境で採取し、正規化の適用に加えて入力・出力・監査・来歴を点検する。漏れがあれば記録・是正し、正規化だけを不在証明にしない | NFR4、BR2.1/BR2.2 |
| NFR5.1 | 性能の数値目標は追加せず、観測された劣化を実測で調べる | upstreamと比較する場合は入力・環境・計測方法を記録する。今回のテスト実行時間をベンチマーク扱いしない。明確な劣化はintent記録へ残す | NFR5 |

## 3. 脅威と残る制約

| 区分 | 対象 | 対策・限界 |
|---|---|---|
| Spoofing / Elevation of Privilege | 純粋な変換APIは認証・認可を持たない | 認証機能をU1へ追加しない。ハッシュ一致を送信者の真正性や認可の証明に使わない |
| Tampering | 正解データを実装に合わせて改変する危険 | C7のピン・来歴・変更方針を保持し、実装を修正する。ハッシュだけで悪意ある書換えを防げるとはしない |
| Repudiation | ゴールデン採取の出所を追えなくなる危険 | commit・採取コマンド・日時を保存する。来歴は再現の手掛かりであり、暗号的な否認防止ではない |
| Information Disclosure | 入力や採取環境の情報が保存される危険 | NFR4.4の点検。正規化されない任意文字列や来歴も対象とする |
| Denial of Service | 深いネストと巨大入力 | 読取の深さを制限する。バイト数上限は現在U1にないため、大きな平坦入力やプログラム構築値の無制限な深さまで安全とは保証しない。受入範囲を広げる際は呼出境界も含めて再評価する |

## 4. 入力範囲の実測と検証の限界

2026-09-06、固定ピンの `tests/golden/upstream-3c3146cf/` 配下のJSON88ファイルを測定した。オブジェクト・配列を1段、スカラーを0段として数え、最大深さは7。最大ファイルはstage-graph.jsonの81,850バイト・深さ4、scope-grid.jsonは13,509バイト・深さ3だった。

正準化32ケースのうちJSON文字列をinputに持つ29ケースも最大深さ7。JavaScriptで構築する3ケースは、このJSON文字列の測定対象外である。方法と全対象は `../nfr-input-measurements.json` に記録した。これらの観測値は将来の全入力の上限ではない。読取上限127は採取済みの入力を排除しないが、それを超える任意のupstream入力との同値性は保証しない。

同セッションのcanon_json試験87件とgolden試験16件は成功した。後者は32行の比較とCLI/フックコーパスの読取・正規化を含む。新たな採取、全CLI実行経路、最新依存脆弱性検査、性能ベンチマークをこの103件で検証済みとはしない。

## 5. 適用範囲

NFR1・NFR2・NFR4は上記の枝番要求で検証する。NFR3のジャーナル再構成・投影はU1の変換処理の責務外。NFR5は数値目標を設けない方針と劣化時の測定をNFR5.1で継承する。常駐サービスの可用性・水平スケール・クラウド監視をこのライブラリの要求として追加しない。


## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-06T01:22:50Z
**Iteration:** 1
**Request Challenge:** review:7f6f633649a245248646fee3ae7673de

### Findings

1回のADVISORYレビューとして既存IDを引き継いだ。既存2件は解消済みで、新規所見はない。

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Minor | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u1-canon-json-goldens/nfr-requirements/security-requirements.md > NFR1.1・NFR4.3・第4節 | 深さ制限の互換影響が明記された。固定コーパスのJSON88ファイルとJSON文字列入力29件は最大深さ7であり、独立した再計算でも全ファイルの記録と一致した。採取済み入力を排除しないことと、将来の任意入力の同値性を保証しないことを区別し、確認済みQ&A・functional-spec W3と同じ127段受理／128段以上拒否を定めている。 | 追加対応なし。受入範囲を拡張するときは、深さ境界と入力測定を再評価する。 | Resolved |
| R-02 | Minor | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u1-canon-json-goldens/nfr-requirements/security-requirements.md > 第3節 Repudiation行 | 「該当なし」と緩和策の併記が解消された。対象をゴールデン採取の出所の追跡とし、commit・コマンド・日時を保持する用途と、暗号的な否認防止ではない限界を明記している。BR2.1およびC7の来歴要件と一致する。 | 追加対応なし。来歴による再現支援と暗号的な真正性の保証を区別する。 | Resolved |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| aidlc-sensor-required-sections（stage=nfr-requirements、security-requirements.md / tech-stack-decisions.md） | PASS、追記前H2数5 / 3、findings_count=0 | 指定された2文書の構造検査を実行した。 |
| aidlc-sensor-upstream-coverage（consumes=functional-spec,rules,requirements,contract-summary、deliverables=security-requirements,tech-stack-decisions） | PASS、unreferenced=[]、findings_count=0 | 出力のscanned_filesも指定2文書だけであることを確認した。 |
| aidlc-sensor-traceability（U1 nfr-requirements/traceability.json） | PASS、findings_count=0 | NFR1〜NFR5の列挙・対応表・対象IDが解決する。NFR3の対象外理由は永続化・投影を持たないU1の境界と一致する。 |
| JSON入力の独立再計算（Python、固定コーパス内だけ） | 記録と全88ファイル一致 | 最大深さ7、最大81,850バイト（stage-graph.json、深さ4）。scope-grid.jsonは13,509バイト・深さ3。32ケース中JSON文字列入力29件も最大深さ7。残り3件をこの測定から除外する説明も一致する。 |
| parse.rsの読取経路・境界テストの照合 | 要求と一致 | check_depthが128段目でTooDeepを返す。127段受理、文字列内の括弧除外、孤立サロゲート拒否の既存テストがある。巨大な平坦入力やプログラム構築値の深さまで保護するとは評価しない。 |
| 既存実行ログ（/tmp/u1-resume-unit-tests.log、/tmp/u1-resume-golden-tests.log） | 87 + 16 passed、0 failed | 同セッションのログを確認し、コード未変更のため再実行していない。golden16件は読取9件と正準化比較7件であり、3プロファイル・2族の全行比較を含む。全CLI/フック経路の実行検証とは区別する。 |
| Cargo.toml・クレート依存・clippy.toml・rust-toolchain.toml・CI・coverage設定 | 本文の設定記述と一致 | serde_jsonのpreserve_order / float_roundtrip、workspace unsafe_code forbidの継承、Rust 1.95.0固定、contents: read、2ロックファイルのcargo audit実行設定、90%床・相対差0.01・シード20260823を確認した。cargo auditの最新成功や現在のCI成功は今回検証していない。 |
| C7・hash-canonical/provenance.json | 一致 | 配置・ピン・32ケース・bun 1.3.13・採取日時・コマンドが記録されている。来歴の存在を秘密情報不在や暗号的真正性の証明とはしない。 |
| linter / type-check | 対象外 | 対象2文書にTypeScript/JavaScriptのコード片はない。 |

### Summary

互換範囲、拒否境界、依存・品質要件、ゴールデン採取時の情報保護が、確認済みQ&Aと共有契約に沿って定義されている。既存2件は解消され、未実行の脆弱性検査・性能測定・全CLI経路検証を成功扱いしていないため、非機能要求としてREADYとする。
