# U1 旧レビュー保存記録

2026-09-06の再レビュー前に旧Review節をそのまま保存した。旧ID 1・2・3は再レビュー時の形式上R-01・R-02・R-03へ対応付け、指摘内容と履歴を引き継ぐ。このファイルは現在の承認を表さない。

## Review

**Verdict:** NOT-READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-05T07:00:29Z
**Iteration:** 1
**Request Challenge:** review:8763a6305d40c2cc847be8ae1e5d58c5

### Findings

旧レビューの数値番号 1〜3 はそのまま保持する。旧 context の「No findings」は解消証拠として用いず、退避された旧レビューと現物を照合した。新規所見は R-04 以降とする。

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| 1 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u1-canon-json-goldens/functional-design/rules.md > BR2.3、および entities.md > GoldenCase.expected | 旧所見は C7 の受入表が入力とハッシュだけを宣言していた点。現在の contract-summary.md > C7 は cases.json の expected に canonical_output / canonical_digest 等を明記し、2026-08-22 の訂正理由も記録している。実コーパスの 32 行と比較テストもこの形を使っている。 | 追加対応なし。現行 C7 とコーパスの対応を維持する。 | Resolved |
| 2 | Minor | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u1-canon-json-goldens/functional-design/functional-spec.md > W2 | 用途とダイジェスト族の対応表は依然ない。ADR 0001 と現行 canon_json/mod.rs は contract_sha256・approval fingerprint を canonical-prefixed、bundle hash・directiveHash・route hash・配送冪等 digest を compact-raw と区別しているが、W2 は用途を一括列挙する。実装側の説明は改善されているものの、本書だけでは選択が曖昧なままである。 | W2 に用途と族の対応を明記し、根拠となる ADR の区分を参照する。 | Unresolved |
| 3 | Minor | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u1-canon-json-goldens/functional-design/entities.md > JsonValue.integer_value | i64/u64 の判別規則は設計に未記載。現行 value/number.rs は非負を PosInt(u64)、負を NegInt(i64)、小数・非有限を Float と説明し、numbers_prefer_unsigned_then_signed_then_float テストも成功する。実装では決着しているが、論理モデルへの反映がない。 | 非負・負・浮動小数の判別規則を entities の制約へ反映する。大整数の出力丸めは R-05 と区別して記述する。 | Unresolved |
| R-04 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u1-canon-json-goldens/functional-design/rules.md > BR1.1・BR1.2、および functional-spec.md > W1 手順 2 | BR1.1 は hash-canonical の全キーを UTF-16 順に整列し、BR1.2 の整数形式キー優先を contract-pretty / contract-compact に限定する。しかしコーパス hash-canonical/integer-like/numeric-vs-string-order の canonical_output はキー順 1,9,10,x であり、文字列順の 1,10,9,x ではない。現行 canonical.rs は全プロファイルで整数形式キーを数値昇順で先頭に置き、残りだけを再帰ソートする。設計を文字どおり再実装すると、U1 自身のゴールデンとハッシュ互換が壊れる。 | BR1.2 の適用を全プロファイルへ広げ、BR1.1 と W1 に「整数形式キーを数値昇順で先頭、残りを UTF-16 順」の二段階を明記する。実コーパスの上記例を境界条件として参照する。 | New |
| R-05 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u1-canon-json-goldens/functional-design/rules.md > BR1.3.logic | integer なら常に十進表記する規則には、JS の正確整数範囲を超えた値の丸めがない。コーパス hash-canonical/large-int/around-2p53 は入力 9007199254740993 の出力を 9007199254740992 に固定し、u64-range では u64 最大値の出力が 18446744073709552000 になる。現行 writer の整数範囲テストと全行比較は成功しており、仕様どおりの正確な整数出力へ戻すと受入値に一致しなくなる。 | BR1.3 に整数の保持型と出力時の JS 互換丸めを分けて定義し、2^53 周辺・u64 上限付近のゴールデンを根拠として明示する。 | New |
| R-06 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u1-canon-json-goldens/functional-design/entities.md > JsonValue.string_value、および rules.md > BR1.4 | string_value は UTF-8 と定義する一方、BR1.4 は孤立サロゲートをエスケープして出力することを要求する。孤立サロゲートはこの値モデルで保持できず、両方を同時に実装できない。現行 mod.rs はこの非対称を明示し、lone_surrogate_escapes_are_rejected_as_syntax_errors テストは読取拒否を固定する。現行実装が対応していない入力まで、設計は互換保証している。 | 対象契約に孤立サロゲートが現れないという根拠を明記したうえで、UTF-8 入力の拒否境界と互換保証範囲を BR1.4・W3 に反映する。対応を要するなら値モデルの変更が必要であり、実装の存在だけで要求を縮小しない。 | New |
| R-07 | Minor | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u1-canon-json-goldens/functional-design/functional-spec.md > 第 2 節 to_value・第 5 節エラー一覧 | to_value を失敗しない JsonValue 返却として宣言しているが、現行 value/json_value.rs は Result<JsonValue, ToValueError> を返す。タプルをキーにしたマップの変換拒否は maps_with_non_string_keys_are_rejected テストで確認できる。唯一の型付き変換境界の失敗経路が設計から落ちている。 | インターフェイス例とエラー一覧に変換失敗を追加し、呼出側へ返す材料とリトライの扱いを明記する。 | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| aidlc-sensor-required-sections.ts（--stage functional-design、各 --output-path） | PASS: entities / rules / functional-spec、所見 0 | 追記前の H2 数は 2 / 2 / 8。文面と実測値の一致までは検査しない。 |
| aidlc-sensor-upstream-coverage.ts（consumes 5 件・deliverables 3 件を明示） | PASS: unreferenced 0 | Unit 定義・要求割当・要求・構成・共有契約への参照がある。 |
| aidlc-sensor-traceability.ts | FAIL: missing_from_upstream_ids 34 件。gaps / orphans / invalid_entries / invalid_targets / missing_from_table は空 | 欠落一覧は FR1〜FR6・FR8・FR9 系で、共有 story-map 上の U1 担当外。U1 の FR7・FR7.1〜7.3 と 13 BR は対応し、対象 Unit の要求欠落としては計上しない。 |
| linter / type-check の適用判定 | 対象外・未実行 | 成果物に TS/JS/TSX のコード出力や該当スニペットがない。Rust 全体の lint 成功は主張しない。 |
| cargo test --locked -p core-infrastructure canon_json | PASS: 87 件、失敗・無視 0 | キー順・整数範囲・符号・孤立サロゲート拒否・変換エラーを含む現行実装の単体・性質テスト。統合テストはこのフィルタでは実行されないため次行で別途実行した。 |
| cargo test --locked -p core-infrastructure --test golden_hash_canonical --test golden_corpus_read | PASS: 7 + 9 = 16 件、失敗・無視 0 | 32 行の正準化コーパスについて 3 プロファイルと 2 ハッシュ族を比較し、CLI/フックコーパスの読取・範囲・正規化も確認。CLI 実装との全経路比較や upstream の再採取は今回行っていない。 |
| C7 と cases.json / provenance.json の現物照合 | 一致 | 旧所見 1 は解消。ピンと採取手順の記録があり、出力文字列とハッシュの両方を保持する。 |
| ER 図・状態遷移の机上確認 | 軽微な補足余地あり | Digest と値の関係を方向付きで読める。failing から再比較成功への遷移は W5 の再比較指示にはあるが状態表に明示されない。Mermaid パーサ検査は未実行。 |

### Summary

未解消の Critical 0・Major 3・Minor 3 のため ADVISORY 判定は NOT-READY。現行実装とゴールデン検証は成功しているが、保存済みの振る舞い仕様には、それを再実装すると互換性を失うキー順・大整数・文字列表現の契約差が残る。実装を古い設計へ戻さず、実測と契約の根拠に沿って設計を同期する必要がある。
