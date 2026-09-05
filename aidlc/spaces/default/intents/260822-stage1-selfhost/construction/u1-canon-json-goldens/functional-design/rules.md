# rules — U1 canon-json とゴールデン（`u1-canon-json-goldens`）

> Functional Design（Construction 3.1）成果物（Unit: U1）。出典: `../../../inception/units-generation/unit-of-work.md`、
> `../../../inception/units-generation/unit-of-work-story-map.md`（FR7.1〜7.3）、`../../../inception/requirements-analysis/requirements.md`
> （FR7、NFR1）、`../../../inception/domain-design/components.md`（CanonJson）、`../../../inception/contract-design/contract-summary.md`
> （C7）、`docs/adr/0001-canonical-json-serializer.md`（決定 1〜6、受入条件 (a)〜(e)）、確認質問 `functional-design-questions.md`。
>
> 下の fenced `yaml` が正本。BR1.x = 直列化・ハッシュの規則、BR2.x = ゴールデンの規則。

## 1. 規則（正本）

```yaml
rules:
  - id: BR1.1
    statement: "全プロファイルで integer-like キーを数値昇順で先頭に置く。残りは contract-pretty / contract-compact では宣言順または挿入順、hash-canonical では再帰的に UTF-16 順にする"
    category: constraint
    applies_to: [JsonValue, SerializationProfile]
    trigger: "serialize / hash の呼出時"
    logic: "各オブジェクトで BR1.2 の整数形式キーを先頭に置く。IF profile = hash-canonical THEN 残りのキーを UTF-16 コード単位順で再帰的に整列 ELSE 残りの宣言順/挿入順を保持。受入例 numeric-vs-string-order の 1,9,10,x は 1,10,9,x にしない"
    violation: "該当なし（内部規則）。キー順が規則と異なる出力はゴールデン不一致として検出"
    source: "FR7.3, ADR 0001 決定 2・3"
  - id: BR1.2
    statement: "ECMAScript の所有プロパティ順序を全プロファイルに適用する — integer-like キーは数値昇順で先頭、残りは BR1.1 の順序"
    category: constraint
    applies_to: [JsonValue]
    trigger: "全プロファイルの直列化"
    logic: "IF キーが integer-like（0〜2^32-2 の正準十進表記）THEN 数値昇順で先頭に並べる ELSE BR1.1 のプロファイル別順序。01、-1、4294967295 は整数形式キーではない"
    violation: "契約 JSON に integer-like キーが現れる箇所は棚卸しし、写像を個別定義する（ADR 0001 決定 3）。未定義のまま現れたらゴールデン不一致で検出"
    source: "FR7.3, ADR 0001 決定 3（JS 実測）"
  - id: BR1.3
    statement: "整数の保持型と出力表記を区別する。出力は JS の数値表記に合わせ、絶対値が 2^53 を超える整数は f64 に丸める。指数閾値 1e21 / 1e-6、'e+' 書式、-0 は '0'、NaN / ±Infinity は null"
    category: calculation
    applies_to: [JsonValue]
    trigger: "kind = number の直列化"
    logic: "IF integer AND 絶対値 ≤ 2^53 THEN 十進表記 ELSE integer は f64 へ変換して浮動小数と同じ JS 互換最短表記を使う。非有限は null、負ゼロは 0。around-2p53 の 9007199254740993 → 9007199254740992、u64-range の最大値 → 18446744073709552000 を受入値とする"
    violation: "該当なし。不一致はゴールデン（負ゼロ・非有限・指数クラス）で検出"
    source: "FR7.3, ADR 0001 決定 4・受入条件 (b)(c)(d)"
  - id: BR1.4
    statement: "UTF-8 で表せる文字列について JSON.stringify の最小エスケープを使う。二重引用符・バックスラッシュ・U+0000〜U+001F のみエスケープし、非 ASCII・斜線・U+2028/U+2029 はそのまま出力する"
    category: constraint
    applies_to: [JsonValue]
    trigger: "文字列の読取・直列化"
    logic: "有効な Unicode scalar value は最小集合だけをエスケープする。読取時の孤立サロゲートは ParseError::Syntax として拒否する。任意の JS UTF-16 文字列との完全互換は主張しない"
    violation: "非 ASCII・エスケープクラスはゴールデンで検証し、孤立サロゲートは拒否テストで境界を固定する。互換範囲と根拠は functional-spec W3 を参照"
    source: "FR7.3, ADR 0001 受入条件 (e)"
  - id: BR1.5
    statement: "体裁はプロファイルで固定 — contract-pretty は 2 スペースインデント + メンバごとの改行 + ファイル末尾改行、contract-compact / hash-canonical は空白なし。空の配列/オブジェクトは '[]' / '{}'"
    category: constraint
    applies_to: [SerializationProfile]
    trigger: "serialize の呼出時"
    logic: "IF profile = contract-pretty THEN JSON.stringify(x, null, 2) + '\\n' 相当 ELSE JSON.stringify(x) 相当"
    violation: "成果物ごとの体裁はゴールデンで固定（ADR 0001 受入条件 3）"
    source: "FR7.3, ADR 0001 決定 2"
  - id: BR1.6
    statement: "ダイジェストは 2 族 — 正準族は hash-canonical 出力の UTF-8 バイト列の sha256 に 'sha256:' を付ける（hashObject 互換）、非正準族は contract-compact 出力の sha256 を生 hex で返す"
    category: calculation
    applies_to: [Digest]
    trigger: "hash の呼出時"
    logic: "IF family = canonical-prefixed THEN 'sha256:' + hex(sha256(utf8(serialize(hash-canonical)))) ELSE hex(sha256(utf8(serialize(contract-compact))))"
    violation: "hash-canonical 受入表（FR7.1）の行不一致"
    source: "FR7.1, FR7.3, ADR 0001 コンテキスト（2 族）"
  - id: BR1.7
    statement: "契約JSONの直列化と型付き値の変換は core-infrastructure::canon_json を通す。呼出側は同じクレート内でも serde_json の直列化関数（to_string / to_string_pretty / to_vec / to_writer 系）と to_value を直接呼ばない"
    category: policy
    applies_to: [workspace]
    trigger: "コンパイル（clippy disallowed-methods）"
    logic: "禁止関数の直接呼出は clippy で拒否する。実装内部で必要な呼出は canon_json の変換・読取境界へ局所化する。契約外の永続化DTO等の例外は clippy 設定と該当箇所の理由付き許可で限定し、クレート全体を除外しない"
    violation: "CI の clippy（-D warnings）で拒否。Value の Display / format! 経由は残余ホール — レビューとゴールデンで補完"
    source: "ADR 0001 決定 5"
  - id: BR1.8
    statement: "serde_json の preserve_order をワークスペース全体で常時有効にし、ソート順が必要な箇所は BTreeMap か hash-canonical の直列化時ソートを明示的に使う"
    category: policy
    applies_to: [workspace]
    trigger: "Cargo フィーチャ解決"
    logic: "preserve_order 有効 → 動的マップは挿入順を保持"
    violation: "フィーチャが落ちるとキー順がソート順に化け、ゴールデン不一致で検出"
    source: "ADR 0001 決定 3"

  - id: BR2.1
    statement: "ゴールデンは upstream ピン 3c3146cf のツールを実行して採取した実出力・実ハッシュでなければならず、採取手順（再採取スクリプト）と来歴をコーパスに同梱する"
    category: policy
    applies_to: [GoldenCorpus, GoldenCase]
    trigger: "ゴールデンの追加・更新"
    logic: "IF ケースに provenance（commit, captured_at, command）が無い THEN 受け入れない"
    violation: "レビューで差し戻し"
    source: "FR7.1, FR7.2, ADR 0001 決定 6, 前提 A3"
  - id: BR2.2
    statement: "非決定値はプレースホルダに正規化してから比較する — タイムスタンプ <TS>、clone id <CLONE>、絶対パス <ROOT>、セッション ID <SESSION>。規則はコーパスの一部として固定し、期待値と実測値の双方に同じ規則を適用する"
    category: constraint
    applies_to: [NormalizationRule, GoldenCase]
    trigger: "ゴールデン比較テスト"
    logic: "normalize(expected) == normalize(actual) をバイト比較"
    violation: "不一致 = テスト失敗（正規化漏れも同様に失敗として現れる）"
    source: "FR7.2, 確認質問 Q1 = A"
  - id: BR2.3
    statement: "hash-canonical 受入表は入力クラス別に全クラスを持ち、各行で出力文字列とダイジェストの両方が一致しなければならない — クラス: ネスト、integer-like キー、非有限数、負ゼロ、指数表記、非 ASCII 文字列、エスケープ、空の配列/オブジェクト、型付き struct のフィールド順"
    category: validation
    applies_to: [GoldenCorpus]
    trigger: "FR7.3 の受入"
    logic: "IF いずれかの行が不一致 THEN FR7.3 不合格"
    violation: "canon-json の実装を直す（ゴールデンは直さない）"
    source: "FR7.1, FR7.3, ADR 0001 受入条件 1・2"
  - id: BR2.4
    statement: "CLI ゴールデンの範囲は next / report / continue / park の主要遷移（開始・awaiting-approval・approve・reject・revise・skip・jump・park/unpark・recompose・set-autonomy）とフック 4 本の代表ケース（許可 / 拒否 / 無視 を 2〜3 件ずつ）。後続 Bolt で必要な経路は追加採取する"
    category: policy
    applies_to: [GoldenCorpus]
    trigger: "FR7.2 の採取計画"
    logic: "上記の遷移・ケースを最小集合とし、不足は追加（削除はしない）"
    violation: "該当なし（計画規則）"
    source: "FR7.2, 確認質問 Q2 = A"
  - id: BR2.5
    statement: "ゴールデンの更新は upstream ピン更新の intent でのみ行い、差分は逸脱台帳と突き合わせてレビューする"
    category: policy
    applies_to: [GoldenCorpus]
    trigger: "upstream ピン更新"
    logic: "IF ピンが変わらない THEN ゴールデンは不変"
    violation: "レビューで差し戻し"
    source: "contract-summary C7, NFR1"
```

## 2. 規則の要約

| ID | 区分 | 一言 | 出典 |
|---|---|---|---|
| BR1.1 | constraint | キー順はプロファイルが決める（hash-canonical のみ再帰ソート） | FR7.3 / ADR 0001 |
| BR1.2 | constraint | 動的マップは ECMAScript のプロパティ順（integer-like 先頭） | FR7.3 / ADR 0001 |
| BR1.3 | calculation | 整数は整数型、浮動小数は JS 互換、非有限は null、-0 は 0 | FR7.3 / ADR 0001 (b)(c)(d) |
| BR1.4 | constraint | エスケープは JSON.stringify の最小集合 | FR7.3 / ADR 0001 (e) |
| BR1.5 | constraint | 体裁はプロファイル固定（pretty = 2 スペース + 末尾改行） | FR7.3 / ADR 0001 |
| BR1.6 | calculation | ダイジェスト 2 族（`sha256:` 接頭辞 / 生 hex） | FR7.1 / FR7.3 |
| BR1.7 | policy | serde_json 直接呼び出し禁止（clippy） | ADR 0001 決定 5 |
| BR1.8 | policy | preserve_order 常時有効 | ADR 0001 決定 3 |
| BR2.1 | policy | ゴールデンは upstream 実行採取 + 来歴必須 | FR7.1 / FR7.2 |
| BR2.2 | constraint | 非決定値はプレースホルダ正規化して比較 | FR7.2 / Q1 |
| BR2.3 | validation | hash-canonical 受入表は入力クラス網羅・全行一致 | FR7.1 / FR7.3 |
| BR2.4 | policy | CLI ゴールデンは主要遷移 + フック代表ケース | FR7.2 / Q2 |
| BR2.5 | policy | ゴールデン更新はピン更新 intent のみ | C7 / NFR1 |
