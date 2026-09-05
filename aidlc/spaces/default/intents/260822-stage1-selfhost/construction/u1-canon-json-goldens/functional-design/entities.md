# entities — U1 canon-json とゴールデン（`u1-canon-json-goldens`）

> Functional Design（Construction 3.1）成果物（Unit: U1）。出典: `../../../inception/units-generation/unit-of-work.md`
> （U1 の責務）、`../../../inception/units-generation/unit-of-work-story-map.md`（FR7.1〜7.3）、
> `../../../inception/requirements-analysis/requirements.md`（FR7）、`../../../inception/domain-design/components.md`
> （CanonJson: 依存ゼロの純粋部品）、`../../../inception/contract-design/contract-summary.md`（C7 ゴールデン、C1 の
> continue_token 正準化）、`docs/adr/0001-canonical-json-serializer.md`、確認質問 `functional-design-questions.md`。
>
> 実装言語に依存しない論理モデル。下の fenced `yaml` が正本。

## 1. エンティティ（正本）

```yaml
entities:
  - name: JsonValue
    description: "メモリ上の JSON 値。オブジェクトのキー順を保持する（JS の挿入順に対応）。canon-json が読み書きの唯一の経路"
    attributes:
      - { name: kind, type: enum, required: true, allowed_values: ["null", boolean, number, string, array, object] }
      - { name: number_repr, type: enum, required: false, allowed_values: [integer, float], constraints: "kind = number のとき必須。契約型の数値は integer に固定（ADR 0001 決定 4）" }
      - { name: integer_value, type: integer(i64/u64), required: false, constraints: "非負の整数は u64、負の整数は i64 で保持する。小数・非有限は float。保持型と出力表記は別で、絶対値が 2^53 を超える整数の出力は BR1.3 の JS 互換丸めを適用" }
      - { name: float_value, type: float(f64), required: false, constraints: "非有限（NaN / ±Infinity）は直列化時に null" }
      - { name: string_value, type: string(UTF-8), required: false, constraints: "Unicode scalar value の列。対をなすサロゲートのJSONエスケープは単一の文字へ復号する。孤立サロゲートは保持できず、読取時に ParseError::Syntax で拒否（BR1.4）" }
      - { name: items, type: list<JsonValue>, required: false, constraints: "kind = array" }
      - { name: members, type: "ordered_list<(key: string, value: JsonValue)>", required: false, unique: key, constraints: "kind = object。順序 = 構築順（挿入順）" }
    constraints:
      - "members のキーは一意"
      - "構築後は不変（値オブジェクト）"

  - name: SerializationProfile
    description: "直列化プロファイル（ADR 0001 決定 2）。用途ごとに体裁とキー順が決まる閉集合"
    attributes:
      - { name: name, type: enum, required: true, unique: true, allowed_values: [contract-pretty, contract-compact, hash-canonical] }
      - { name: indent, type: enum, required: true, allowed_values: [two-spaces, none], defaults: "pretty = two-spaces、他 = none" }
      - { name: trailing_newline, type: boolean, required: true, defaults: "pretty = true、他 = false" }
      - { name: key_order, type: enum, required: true, allowed_values: [declared-or-insertion, recursive-sorted], constraints: "全プロファイルで integer-like キーが数値昇順で先頭。残りのキーのみ hash-canonical では再帰的に UTF-16 順、それ以外は宣言順/挿入順" }
      - { name: purpose, type: string, required: true }
    constraints:
      - "3 値の閉集合。追加はプロファイル仕様（ADR 0001）の改訂を伴う"

  - name: Digest
    description: "sha256 ダイジェスト。正準族（hashObject 互換、`sha256:` 接頭辞付き）と非正準族（生 hex）の 2 形"
    attributes:
      - { name: family, type: enum, required: true, allowed_values: [canonical-prefixed, compact-raw] }
      - { name: hex, type: string, required: true, constraints: "64 桁の小文字 16 進" }
      - { name: rendered, type: string, required: true, constraints: "canonical-prefixed = 'sha256:' + hex、compact-raw = hex" }
    relationships:
      - { to: SerializationProfile, cardinality: "many-to-one", direction: "Digest → SerializationProfile", description: "canonical-prefixed は hash-canonical、compact-raw は contract-compact の出力バイト列から計算" }

  - name: GoldenCase
    description: "upstream ピン `3c3146cf` から採取した正解データ 1 件（C7）"
    attributes:
      - { name: id, type: string, required: true, unique: true, constraints: "family/<group>/<case> の形" }
      - { name: family, type: enum, required: true, allowed_values: [hash-canonical, cli, hook] }
      - { name: input, type: record, required: true, constraints: "hash-canonical: 入力 JSON 文字列。cli: argv + stdin + 初期ワークスペース。hook: stdin JSON + 初期状態" }
      - { name: expected, type: record, required: true, constraints: "hash-canonical: 出力文字列 + Digest。cli: stdout + 状態ファイル差分 + 監査行。hook: exit code + stderr + 監査行" }
      - { name: normalization, type: list<NormalizationRule>, required: true, constraints: "期待値・実測値の双方に同じ規則を適用してから比較（Q1 = A）" }
      - { name: provenance, type: record, required: true, constraints: "upstream commit, captured_at, capture command" }
    relationships:
      - { to: GoldenCorpus, cardinality: "many-to-one", direction: "GoldenCase → GoldenCorpus" }

  - name: NormalizationRule
    description: "非決定値をプレースホルダへ置換する規則（Q1 = A）"
    attributes:
      - { name: placeholder, type: enum, required: true, allowed_values: ["<TS>", "<CLONE>", "<ROOT>", "<SESSION>"] }
      - { name: pattern, type: string, required: true, constraints: "置換対象の形（ISO 8601 UTC、<host>-<clone> 形のシャード名、リポジトリ絶対パス、セッション ID）" }
      - { name: applies_to, type: list<enum>, required: true, allowed_values: [stdout, state-diff, audit, stderr] }

  - name: GoldenCorpus
    description: "ゴールデン全体。再採取スクリプトと正規化規則の正本を含む"
    attributes:
      - { name: upstream_commit, type: string, required: true, constraints: "3c3146cf（v2.6.40）。更新は別 intent（C7）" }
      - { name: recapture_command, type: string, required: true, constraints: "bun で upstream ツールを実行する再現可能な手順（前提 A3）" }
      - { name: rules, type: list<NormalizationRule>, required: true }
      - { name: cases, type: list<GoldenCase>, required: true }
    constraints:
      - "cases は family ごとに最低 1 件。hash-canonical は入力クラス別（ADR 0001 受入条件 2）に全クラスを網羅"

relationships:
  - { from: Digest, to: JsonValue, cardinality: "one-to-one", description: "Digest は 1 つの JsonValue の直列化バイト列から決定的に計算される" }
  - { from: GoldenCase, to: SerializationProfile, cardinality: "many-to-one", description: "hash-canonical 族のケースは hash-canonical プロファイルで検証、cli/hook 族は contract-compact（stdout）と contract-pretty（ディスク成果物）で検証" }
```

## 2. 要約

- **JsonValue** は挿入順を保持する不変の JSON 値で、読み書きの唯一の入口。数値は整数優先（契約型は整数に固定）。
- **SerializationProfile** は 3 値の閉集合。体裁（インデント・末尾改行）とキー順（宣言/挿入順 vs 再帰ソート）を決める。
- **Digest** は 2 族（正準 = `sha256:` 接頭辞、非正準 = 生 hex）で、どのプロファイルのバイト列から計算するかが固定。
- **GoldenCase / GoldenCorpus / NormalizationRule** は upstream から採取した正解データとその比較規則。正規化規則は
  コーパスの一部として固定され、期待値と実測値の両方に同じ規則を適用する。
- 状態を持つエンティティは無い（ライブラリは純粋関数群。GoldenCase は採取 → 検証の静的データ）。
