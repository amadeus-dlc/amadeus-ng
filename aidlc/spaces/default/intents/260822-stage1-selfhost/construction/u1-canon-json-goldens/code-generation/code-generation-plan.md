# code-generation-plan — U1 canon-json とゴールデン（`u1-canon-json-goldens`）

> Code Generation（Construction 3.5）の計画（Unit: U1、kind: library、Bolt: B1、規模 M）。出典:
> `../functional-design/functional-spec.md`（W1〜W5、インターフェイス）、`../functional-design/rules.md`（BR1.1〜BR1.8 /
> BR2.1〜BR2.5）、`../functional-design/entities.md`、`../nfr-requirements/security-requirements.md`（NFR1.x / NFR2.x /
> NFR4.x）、`../nfr-requirements/tech-stack-decisions.md`、`../nfr-design/security-design.md`、
> `../nfr-design/logical-components.md`、`../../../inception/contract-design/contract-summary.md`（C7 — Q2 = A で layout
> 改訂済み）、`../../../inception/units-generation/unit-of-work.md`（U1）、`../../../inception/requirements-analysis/
> requirements.md`（FR7.1〜7.3、NFR1/NFR2/NFR4）、`docs/adr/0001-canonical-json-serializer.md`、
> `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/`（全規則）、`code-generation-questions.md`（Q1 = A、Q2 = A）。
>
> 実装はワークスペースルート（`modules/shared/canon-json/`、`tests/golden/upstream-3c3146cf/`、`scripts/goldens/`）に書く。
> 記録ディレクトリにはコードを置かない。brownfield: 既存ファイルはその場で変更し、複製ファイルを作らない。

## 1. 前提と範囲

- **作るもの**: (1) `canon-json` クレートの実体化（既存スタブ `modules/shared/canon-json/`）。(2) upstream ピン
  `3c3146cf`（v2.6.40）から採取したゴールデン（hash-canonical 受入表 = FR7.1、CLI / フック実行出力 = FR7.2）と
  再採取スクリプト。(3) ゴールデン比較器（テスト側）。
- **作らないもの**: 他 Unit のコード（U3 の WorkflowDefinitionRepositoryImpl の `serde_json` → canon-json 置換、
  U6 の continue_token、U7 の CLI）。ADR 0001「未確定事項」の確定記述は U9（canon-docs）へ引き渡す（採取値は
  ゴールデン README に記録して渡す）。`cargo audit` / `rust-toolchain.toml` / `unsafe_code` の workspace lint 昇格は U10。
- **ブランチ（Q1 = A）**: `main-sync` から `bolt/b1-u1-canon-json-goldens` を切る。最初のコミットは aidlc 記録
  （`aidlc/` 配下）のみ、以降はコードのコミット。PR は Bolt ゲート承認後にコンダクタが 1 本だけ開く（直列運用）。
  開発エージェントは push / PR を行わない。
- **ゴールデン配置（Q2 = A）**: `tests/golden/upstream-3c3146cf/{hash-canonical,cli,hooks}/`。既存の
  `stage-graph.json` / `scope-grid.json` と README は同ディレクトリ直下のまま**バイト不変**（README の規定）。
  README には節を追記する（既存文のバイトは変えない）。
- **採取環境**: bun 1.3.13、ネットワーク有り（`raw.githubusercontent.com/awslabs/aidlc-workflows/3c3146cf…` は
  HTTP 200 を実測）。upstream ピンのコードを**実行して**採取する（BR2.1）。採取に失敗したケースは欠落として
  記録し、捏造しない（W4 のエラー経路）。
- **コーディング規則**（正本 `coding-rules/`）: フィールド既定 private（アクセサ公開）、モジュール既定 private
  （公開は `lib.rs` の `pub use` 列挙のみ、利便再エクスポート禁止）、ドメイン同値は `PartialEq`/`Eq`、`unwrap` /
  `expect` はプロダクトコード禁止、`missing_docs` deny（全公開要素に rustdoc）、thiserror / anyhow は使わず手実装
  エラー enum + `fmt::Display`。Tell-Don't-Ask。Repository 語彙の造語禁止（本 Unit に Repository は無い）。

## 2. 公開 API（設計の写し — 実装の契約）

`modules/shared/canon-json/src/lib.rs` が公開するのは次だけ（`pub use` 列挙。モジュール `value` / `profile` / `writer` /
`canonical` / `digest` / `parse` はすべて private）:

```text
JsonValue { Null, Bool(bool), Number(Number), String(String), Array(Vec<JsonValue>), Object(ObjectMembers) }
Number { PosInt(u64), NegInt(i64), Float(f64) }            // 非負は u64 優先、負は i64、小数・非有限は f64
ObjectMembers                                               // 挿入順保持・キー一意（同名挿入は値を置換し位置は維持 = JS）
SerializationProfile { ContractPretty, ContractCompact, HashCanonical }  // indent() / trailing_newline() / key_order() / purpose()
Digest { family(), hex(), rendered() }   DigestFamily { CanonicalPrefixed, CompactRaw }
ParseError { Syntax { offset, detail }, TooDeep { limit }, Encoding }   ToValueError
serialize(&JsonValue, SerializationProfile) -> String
hash_canonical(&JsonValue) -> Digest            // "sha256:" + hex（hash-canonical 出力の UTF-8 バイト列）
hash_compact(&JsonValue) -> Digest              // 生 hex（contract-compact 出力の UTF-8 バイト列）
parse(&str) -> Result<JsonValue, ParseError>    // 挿入順保持、深さ上限 128（TooDeep）
parse_bytes(&[u8]) -> Result<JsonValue, ParseError>   // 不正 UTF-8 → Encoding
to_value<T: Serialize>(&T) -> Result<JsonValue, ToValueError>   // 型付き struct → 宣言順（契約経路の唯一の変換点）
```

設計からの差分（記録）: functional-spec §2 は `to_value -> JsonValue` だが、`serde_json::to_value` は失敗し得る
（非文字列キーのマップ等）ため `Result` で返す（`unwrap` 禁止の帰結）。`parse_bytes` は `ParseError::Encoding` を
到達可能にするために追加する。

## 3. 規則の実装方針（BR → コード）

| 規則 | 実装 |
|---|---|
| BR1.1 / BR1.2 キー順 | contract-*: integer-like キー（0〜2^32-2 の正準十進表記）を数値昇順で先頭、残りは挿入/宣言順。hash-canonical: integer-like を数値昇順で先頭、残りを **UTF-16 コード単位順**（`encode_utf16` で比較）。upstream の `canonicalize` は `Object.keys().sort()` → `Object.fromEntries` なので、integer-like 先頭は JS のプロパティ順により hash-canonical でも成立する — ゴールデンで固定 |
| BR1.3 数値 | `PosInt` / `NegInt` は十進。ただし **|v| > 2^53** の整数は JS が f64 として扱うため f64 経路（`v as f64`）で書く（JS の丸めと一致）。`Float`: 非有限 → `null`、`0.0` / `-0.0` → `0`、それ以外は `format!("{:e}")` の最短桁 + 指数から ECMA-262 `Number::toString`（k, n 規則: 1e-6 ≤ \|x\| < 1e21 は非指数表記、それ以外は `d.ddde±N`、`e+` 書式）を組み立てる |
| BR1.4 エスケープ | `"`, `\\`, U+0000〜U+001F（`\b \f \n \r \t` 短縮、他は `\u00xx` 小文字 hex）。非 ASCII と `/` と U+2028/2029 は生出力。孤立サロゲートは Rust 文字列に存在しないため該当なし（入力側: serde_json は `"\ud800"` を拒否 → `ParseError::Syntax`。既知の非対称として README に記録、契約 JSON には現れない） |
| BR1.5 体裁 | pretty = 2 スペース・`"key": value`・メンバごと改行・末尾 `\n`・空は `[]` / `{}`。compact / canonical = 空白なし |
| BR1.6 ダイジェスト | `sha2::Sha256` で直列化文字列の UTF-8 バイト列をハッシュ。`Digest::rendered()` が族ごとの表記 |
| BR1.7 直接呼び出し禁止 | ルート `clippy.toml` の `disallowed-methods` に `serde_json::to_string` / `to_string_pretty` / `to_vec` / `to_vec_pretty` / `to_writer` / `to_writer_pretty` / `to_value` を登録（reason に canon-json 経由を明記）。canon-json 内の唯一の呼出点（`to_value`、`from_str` は禁止対象外）は `#[allow(clippy::disallowed_methods)]` + 理由コメント。既存コードに禁止関数の呼出があれば棚卸しして code-summary に列挙し、lint が赤になる箇所だけ最小修正（他 Unit の設計には踏み込まない） |
| BR1.8 preserve_order | `[workspace.dependencies]` に `serde_json = { version = "1", features = ["preserve_order"] }`・`serde = { version = "1", features = ["derive"] }`・`sha2`・`proptest` を置き、既存クレート（core-domain dev-dep、core-interface-adapter）も `serde_json.workspace = true` に揃える（フィーチャ統合をクレート単独ビルドでも保証） |
| NFR4.3 深さ上限 | `parse` 前に文字列リテラル外の `{` `[` を数える軽量スキャンで深さ > 128 を `TooDeep { limit: 128 }` として決定的に拒否（serde_json 既定の再帰上限と同値。`disable_recursion_limit` は使わない）。上限は `const` で 1 か所 |
| BR2.1 来歴 | 各コーパスに `provenance.json`（upstream commit、取得 URL、抽出スニペットの sha256、captured_at、command、bun version）。ケース単位でも `provenance` を持つ |
| BR2.2 正規化 | 比較器 `normalize()`: `<TS>`（ISO 8601 UTC）、`<CLONE>`（`<host>-<clone>` シャード名）、`<ROOT>`（作業ツリー絶対パス）、`<SESSION>`（セッション ID）。規則は `tests/golden/upstream-3c3146cf/normalization.json` に固定し、期待値・実測値の双方に適用 |
| BR2.3 受入表 | 入力クラス: ネスト / integer-like キー（混在・ソート） / 非有限数 / 負ゼロ / 指数表記（1e21, 1e-7, 123e-20 など閾値の両側） / 2^53 超の整数 / 非 ASCII 文字列 / エスケープ（制御文字・`"`・`\\`・`/`・U+2028） / 空の配列・オブジェクト / 型付き struct のフィールド順 / 浮動小数の整数値（1.0） |
| BR2.4 CLI 範囲 | next（開始）・report awaiting-approval / approved / rejected / revised・continue（load-steering）・skip・jump・park / unpark・recompose・set-autonomy、フック 4 本 × 許可 / 拒否 / 無視 2〜3 件 |
| BR2.5 更新方針 | README に「ピン更新 intent でのみ更新」を明記。更新手順 = 再採取スクリプト |

## 4. 棚卸し（code-generation で確定し code-summary に記録する事項）

- [ ] I1. 契約 JSON の実測最大ネスト深さ（`tests/golden/upstream-3c3146cf/*.json`、`.claude/tools/data/*.json`、
      `.claude/tools/data/scopes/`、ゴールデン入力）が 128 を十分下回ること（想定 10 段未満）。
- [ ] I2. 契約 JSON に integer-like キーが現れないこと（現れる場合は箇所と写像を記録）。
- [ ] I3. 契約 JSON のキーが ASCII のみであること（UTF-16 順 = バイト順の前提）。
- [ ] I4. 契約 JSON に浮動小数フィールドが現れないこと（現れる場合は一覧）。
- [ ] I5. ワークスペース内の `serde_json::to_*` / `to_value` 直接呼出の棚卸し（lint 導入の影響範囲）。
- [ ] I6. `preserve_order` 有効化で既存テスト（ITF 準拠 2 本の `serde_json::Value` 利用）が緑のままであること。
- [ ] I7. `components.md` の CanonJson `external_dependencies: []` を実依存（sha2 / serde / serde_json）へ更新
      （記録側、コンダクタが実施 — nfr-design レビュー Minor 1）。

## 5. 実装ステップ（TDD、レイヤーごとに Red → Green → Refactor）

Testing Contract の `plan_profile.steps` を基線とし、ライブラリに存在しない層（Repository / Frontend）は省く。
「Data model」= 値・プロファイル・ダイジェストの型、「Business logic」= writer / canonical / digest / parse、
「API」= ファサードと `to_value`。各 Red ステップでは失敗するコマンド出力（失敗テスト名と要約行）を
`code-summary.md` に記録してから Green に進む。

### 5.0 コンダクタ（承認後・委任前）

- [ ] Step 0. Bolt 開始と ブランチ: `bun .claude/tools/aidlc-bolt.ts start --name B1 --batch 1` →
      `git switch -c bolt/b1-u1-canon-json-goldens`（`main-sync` から）→ aidlc 記録を 1 コミット
      （`chore(aidlc): record inception and U1 design for stage-1 self-host`）。

### 5.1 骨格（開発エージェント — 委任 1）

- [x] Step 1. プロジェクト構造と設定: `Cargo.toml` の `[workspace.dependencies]` に serde / serde_json(preserve_order) /
      sha2 / proptest を追加し、`core-domain`（dev-dep）・`core-interface-adapter` を `.workspace = true` に揃える。
      `modules/shared/canon-json/Cargo.toml` に `serde` / `serde_json` / `sha2`（runtime）、`proptest`（dev）を追加。
      `clippy.toml` に `disallowed-methods`（BR1.7）。`lib.rs` に private モジュール 6 本の空殻と `pub use` 列挙の枠。
      `cargo build -p canon-json` と `cargo clippy --workspace --all-targets -- -D warnings` が緑（I5 / I6 の棚卸しを
      ここで実施）。
- [x] Step 2. テストランナー確認: `cargo test -p canon-json`（brownfield — 実測済み: 0 tests, exit 0）。
      統合テストの置き場 `modules/shared/canon-json/tests/` と、ゴールデンを `env!("CARGO_MANIFEST_DIR")/../../../
      tests/golden/upstream-3c3146cf/` で読む経路を決め、`unit-test-instructions.md` のコマンドで走ることを確認。

### 5.2 ゴールデン採取 — hash-canonical 受入表（FR7.1 / BR2.1 / BR2.3）

- [x] Step 3. 再採取スクリプト: `scripts/goldens/recapture-hash-canonical.sh`（bash, `set -euo pipefail`）と
      `scripts/goldens/capture-hash-canonical.ts`（bun）。手順: 使い捨てディレクトリに upstream ピン
      `3c3146cfd7cef33020d48e8d48d4e80d0f8c2820` の `dist/claude/.claude/tools/aidlc-testing-posture.ts` を取得 →
      `canonicalize` / `sha256` / `hashObject`（upstream 仕様 09-cli-tools.md §8.4 が指す `:104-123`）をスニペットとして
      抽出し sha256 で照合（期待値はスクリプトに固定） → `export` を付けた一時モジュールとして bun から import →
      入力クラス（§3 BR2.3 行）ごとに JS 値を評価し、`JSON.stringify(canonicalize(v))` / `hashObject(v)`（正準族）、
      `JSON.stringify(v)` と `sha256(JSON.stringify(v))` 生 hex（非正準族）、`JSON.stringify(v, null, 2) + "\n"`
      （pretty）を採る → `tests/golden/upstream-3c3146cf/hash-canonical/cases.json` と `provenance.json` に書く。
      入力は JSON テキスト（`input`）で表し、JSON で表せない NaN / ±Infinity のクラスだけ `input_js`（JS 式文字列）
      + Rust 側の構築手順（`construct`）を持つ。ケース ID は `hash-canonical/<class>/<case>`。
- [x] Step 4. 採取の実行とレビュー: スクリプトを実行してコーパスを生成し、`git diff` で内容を目視（秘密情報・
      絶対パス無し）。`README.md` に「採取ゴールデン」節を追記（採取手順・来歴・正規化規則・更新方針 BR2.5・
      既知の非対称: 孤立サロゲート）。

### 5.3 canon-json — Data model 層（value / profile / digest の型）

- [x] Step 5. Red: `JsonValue` / `Number` / `ObjectMembers`（挿入順・同名置換・アクセサ）、`SerializationProfile`
      3 値の属性（indent / trailing_newline / key_order / purpose）、`Digest` / `DigestFamily` の `rendered()`、
      `ParseError` / `ToValueError` の `Display` を対象に失敗テスト（各コンポーネント 5〜8 本）を書き、失敗出力を記録。
- [x] Step 6. Green: 最小実装（フィールド private + アクセサ、`PartialEq` 導出、手実装 `Display`）。
- [x] Step 7. Refactor: 命名・rustdoc（`missing_docs`）・`must_use` 整理。テスト緑のまま。

### 5.4 canon-json — Business logic 層（writer / canonical / digest / parse）

- [x] Step 8. Red: (a) ゴールデン受入表テスト `tests/golden_hash_canonical.rs` — 全行で hash-canonical 出力と
      `sha256:` ダイジェスト、compact 出力と生 hex、pretty 出力を比較（失敗 = 行ごとの diff を表示）。
      (b) ユニット: 数値表記クラス（整数・2^53 超・1.0・1e21 / 1e-7 境界・負ゼロ・非有限）、エスケープクラス、
      キー順（integer-like 混在・UTF-16 順）、体裁（pretty の入れ子・空）、parse（不正 JSON の offset、深さ 128 超
      → TooDeep、`parse_bytes` の不正 UTF-8 → Encoding、重複キーは後勝ち・位置維持）。失敗出力を記録。
- [x] Step 9. Green: writer（プロファイル分岐・数値ライタ・最小エスケープ・体裁）、canonical（再帰ソート）、
      digest（sha2）、parse（深さスキャン → `serde_json::from_str` preserve_order → `JsonValue` 変換）。
- [x] Step 10. Refactor: 数値ライタの分離、重複排除、rustdoc。ゴールデン全行一致・ユニット緑のまま。
- [x] Step 11. PBT（proptest、`src/` 同居）: 決定性（同入力 → 同出力）、`parse(serialize(v, compact)) == v`
      （NaN を含まない生成器）、`hash_canonical` の冪等性、canonical ソートの冪等性。ケース数は既定（シード固定は U10）。

### 5.5 canon-json — API 層（ファサードと to_value）

- [ ] Step 12. Red: `#[derive(Serialize)]` の struct が宣言順の `JsonValue` になること（ネスト・`Option` の `None`
      スキップ有無は serde の既定どおり — テストで固定）、`to_value` の失敗経路（非文字列キーのマップ → `ToValueError`）、
      ファサードが設計の列挙どおりの項目だけを公開していること（`lib.rs` を読む軽量テスト or doc test）。
- [ ] Step 13. Green: `to_value`（`serde_json::to_value` → `JsonValue` 変換、`#[allow(clippy::disallowed_methods)]`
      + 理由コメント）、`lib.rs` の `pub use` 列挙。
- [ ] Step 14. Refactor: クレート rustdoc（`//!`）に 3 プロファイル・2 族・禁止規則・深さ上限を記す。

### 5.6 棚卸しと品質ゲート（委任 1 の締め）

- [ ] Step 15. 棚卸し I1〜I6 を実施し結果を `code-summary.md` 用に報告（小さな検査テスト or bun/シェルのワンライナー。
      数値はすべて実測）。
- [ ] Step 16. 品質ゲート: `cargo fmt --all --check` → `cargo clippy --workspace --all-targets -- -D warnings` →
      `cargo lint` → `cargo test --workspace` → `cargo llvm-cov -p canon-json --summary-only`（導入済みなら。
      canon-json は 100% 近傍を目標、床 90%）。コミットは意味単位（`feat(canon-json): …` / `test(goldens): …`）。

### 5.7 ゴールデン採取 — CLI / フック実行出力（FR7.2 / BR2.4 — 委任 2）

- [ ] Step 17. 再採取スクリプト `scripts/goldens/recapture-cli.sh` + `capture-cli.ts`: 使い捨てディレクトリに upstream
      ピンを取得（`git init && git fetch --depth 1 origin 3c3146cf… && git checkout FETCH_HEAD`、失敗時は raw 取得に
      フォールバック）→ `dist/claude/` を使い捨てワークスペースへ配置 → BR2.4 の主要遷移を bun で順に実行
      （`AIDLC_SKIP_HUMAN_PRESENCE_GUARD` 等、非対話化に必要な env を記録）→ 各ステップの stdout JSON・
      `aidlc-state.md` 差分・監査シャード差分を採り、`normalization.json` の規則で正規化 →
      `tests/golden/upstream-3c3146cf/cli/<verb>/<case>/{argv,stdin,stdout.json,state.diff,audit.md}`。
      フック 4 本（stop-forwarding-loop / record-human-turn / state-transition-guard / write-audit-log に対応する
      upstream のフックファイル）に代表 stdin JSON を与え `hooks/<hook>/<case>/{stdin.json,exit,stderr}`。
      非対話で再現できない遷移は**欠落として**`cases-missing.json` に理由つきで記録（捏造しない。後続 Bolt U6 / U7 で追加）。
- [ ] Step 18. 比較器の整備: `modules/shared/canon-json/tests/support/`（または dev-dependency のテスト支援モジュール）に
      `normalize()`・コーパス読取・行ごと diff を置き、cli / hooks 族は「読めて正規化できる」ことまでをテストで固定
      （実装突合せは U6 / U7）。README に cli / hooks 節を追記。
- [ ] Step 19. 品質ゲート再実行（Step 16 と同じ）とコミット。

## 6. トレーサビリティ（ストーリー → ステップ）

| 要求 / 規則 | ステップ | 主な成果物 |
|---|---|---|
| FR7.1 受入表採取 | 3, 4 | `scripts/goldens/recapture-hash-canonical.sh`, `capture-hash-canonical.ts`, `tests/golden/upstream-3c3146cf/hash-canonical/cases.json` |
| FR7.2 CLI / フックゴールデン | 17, 18 | `scripts/goldens/recapture-cli.sh`, `capture-cli.ts`, `tests/golden/upstream-3c3146cf/{cli,hooks}/` |
| FR7.3 canon-json 実装（受入表全行一致） | 5〜14 | `modules/shared/canon-json/src/*.rs`, `tests/golden_hash_canonical.rs` |
| BR1.1〜BR1.6 | 8〜10 | `writer.rs`, `canonical.rs`, `digest.rs` |
| BR1.7 | 1, 13 | `clippy.toml`, `value.rs`（allow 箇所） |
| BR1.8 | 1 | `Cargo.toml`（workspace.dependencies） |
| BR2.1 / BR2.2 / BR2.5 | 3, 4, 17, 18 | `provenance.json`, `normalization.json`, README |
| BR2.3 | 3, 8 | `cases.json`, `tests/golden_hash_canonical.rs` |
| BR2.4 | 17 | `tests/golden/upstream-3c3146cf/cli/`, `hooks/` |
| NFR1.1〜1.3 | 3, 8〜10, 18 | ゴールデン + 比較器 |
| NFR2.1〜2.3 | 5, 8, 11, 16 | Red 記録、カバレッジ、PBT |
| NFR4.1 / NFR4.2 | 1 | `Cargo.toml`（依存 3 つ）、`#![forbid(unsafe_code)]` 維持 |
| NFR4.3 | 8, 9 | `parse.rs`（深さスキャン・ParseError） |
| NFR4.4 | 4, 17 | 正規化・README・目視 |

## 7. 委任の形

- 委任 1（Step 1〜16）と委任 2（Step 17〜19）を同じ承認済み計画・同じ指紋の下で**直列に** aidlc-developer-agent へ
  委任する（1 回の委任に詰め込むと文脈が長すぎるため）。各委任の冒頭行は `AIDLC-UNIT: u1-canon-json-goldens` と
  `AIDLC-TESTING-CONTRACT: <contract_sha256>`。
- 失敗時はコンダクタが halt-and-ask（retry / skip / abort）を出す。

## Testing Contract

```json
{
  "version": 1,
  "methodology": "tdd",
  "source": "team",
  "ordering": "新規プロダクションコードはレイヤーごとに red-green-refactor",
  "scope": "classic",
  "test_strategy": "standard",
  "project_type": "brownfield",
  "applicable_notes": [
    {
      "layer": "org",
      "text": "We treat tests as a first-class deliverable in every Bolt. The specific\nmethodology (TDD, BDD, ATDD, or classic test-after) is affirmed at\npractices-discovery and recorded in `team.md` under this heading with explicit\n`Methodology` and `Ordering` fields; Code Generation resolves those fields\nindependently from coverage, tooling, and scope notes.\n\nWhen no posture has been affirmed, our default per scope is:\n- **Methodology**: test-after\n- **Ordering**: implement each applicable testable layer, then write and run\n  that layer's tests.\n- `mvp`, `enterprise`, `feature`, `infra`, `classic` add an 80% line-coverage\n  floor and CI execution before merge.\n- `bugfix`, `security-patch` add a targeted regression for the specific\n  bug/vulnerability and require the existing suite to remain green.\n- `express` uses the Minimal strategy: requirement-driven unit tests (one per\n  requirement, with a happy-path floor per component); existing tests remain\n  green.\n- `poc`, `refactor`, `workshop` add no extra new-test floor and require the\n  existing suite to remain green.\n\nThe active `Test Strategy` still applies in every scope and determines test\nvolume/types. Scope floors are additive; they never reduce or replace the\nselected strategy.\n\nAffirm a stricter posture in `team.md` if the team commits to one."
    },
    {
      "layer": "team",
      "text": "- **Methodology**: tdd\n- **Ordering**: 新規プロダクションコードはレイヤーごとに red-green-refactor\n  （失敗するテストを先に書く）で実装する。Quint モデル検査・ITF 準拠テスト・\n  ゴールデンパリティは TDD サイクルの外側の受け入れゲートとして維持し、\n  TDD の red を代替しない。（インタビュー Q2、選択肢 A で確定——品質レビュー\n  の自己完結化置換案どおり）\n\nテストピラミッド（ユニット層を厚く、結合・E2E層を薄く）を意識した配分とする\n（オーナー明言）。比率は**定性のみ**とし、数値目標は定めない（インタビュー\nQ3、選択肢 A）: 単体テスト優位・統合テストは境界ごと・E2E は最小、という\n配置規則で充足する。\n\nこのプロジェクトは TDD の上に **3層の品質保証** を重ねている点が特徴的で、\nそれぞれ役割が異なる（`code-quality-assessment.md` §品質保証の全体像より）:\n\n1. **Quint 形式検証**（毎 PR）— 決定論コアの状態機械契約そのものを検証。\n   不変条件 run 27本・到達性 witness 12本の反転判定・決定的シナリオ。\n   モデルの検査力自体も mutation テストで証明済み（engine_loop 3/3、\n   audit_lock 10/10 + witness 7/7、stop_hook 7/7）。\n2. **ITF 準拠テスト**（`modules/core/domain/tests/`、engine_loop / audit_lock\n   の2モデル・2ファイル）— Quint モデルのトレースを集約に再生し状態射影を\n   突き合わせることで、モデルと実装の乖離を検出。TDD の「テストを先に書く」\n   対象は実装コードだが、契約の正本は Quint 側にあるため、ITF 準拠テストは\n   実装後に契約適合を機械確認する位置づけ（TDD サイクルの red-green-refactor\n   そのものではなく、その外側のゲート）。なお stop_hook は ITF 準拠テストが\n   未整備（既知の穴、`evidence.md` インタビュー未確定事項 (e) 参照）。\n3. **PBT（proptest）+ ゴールデンパリティ**— upstream 配布実バイト33ノードの\n   全数 load パリティを固定し、upstream 互換の逸脱を検出。\n\nしたがって TDD サイクルは主にユニットテスト層（インライン `#[cfg(test)]`、\n実測**40ファイル**——集計方法: `modules/` 配下・`tests/` ディレクトリを除いた\nインライン `#[cfg(test)]` 数。`tests/` 配下6本（ITF準拠2 + 統合4）を含めると\n46、`tools/lint/src/check.rs` を含めても47であり、いずれの集計でも48には\nならない。開発者レビュー指摘どおり40へ訂正した）に適用し、ITF 準拠テスト・\nゴールデンパリティはレイヤー横断の受け入れ確認として TDD サイクルの外側に\n位置づける。\n\n- **カバレッジ**: 絶対ゲート90%床 + PR 相対ゲート（head が base を下回ったら\n  fail、許容誤差 0.5pp。PBT のシード非固定に起因するノイズ較正値であり、\n  stage-1 スコープで**シード固定により 0.01 へ引き締める**——インタビュー\n  Q7、選択肢 A/B。除外設定は現状無いが、**composition root（`main.rs` の\n  配線部分）のみカバレッジ除外を許可**し、それ以外は床を維持する\n  （インタビュー Q5、選択肢 B。除外設定は `scripts/coverage.sh` への確定\n  アクション、`evidence.md` 参照）。実測 94.87〜95.29%（`scripts/coverage.sh`）。\n- **ツーリング**: `cargo test --workspace`（234テスト全緑、実測）、\n  `cargo-llvm-cov`、Quint 0.32.0（Node 22 経由）。\n- **テスト種別**: ユニット（インライン `#[cfg(test)]`）、PBT（proptest、集約\n  本体同居）、ITF 準拠（`modules/core/domain/tests/` 2本）、統合（\n  `modules/core/interface-adapter/tests/` 4本 — ゴールデンパリティ・FS ロック・\n  Repository 実装・シンボリックリンク防御）。\n- **CI ゲート**（`main` へのマージ条件、実測）: `check` ジョブ（`cargo fmt\n  --all --check` → `cargo clippy --workspace --all-targets -- -D warnings` →\n  `cargo lint` → `cargo test --workspace`）、`quint` ジョブ\n  （`scripts/quint-gate.sh`）、`coverage` ジョブ（`scripts/coverage.sh`、\n  絶対90%床 + PR 相対ゲート）の3ジョブすべてを緑にする。この3ジョブは\n  **stage-1 スコープで branch protection の required status checks として\n  機械強制する**（インタビュー Q4、選択肢 A——現状は運用規律のみで機械強制が\n  無いという品質レビューの重大指摘を受けての裁定。設定作業は\n  `evidence.md` の確定アクションに記載）。\n- **スコープ注記**: `tools/lint`（`cargo lint` の実装クレート）は workspace\n  非メンバーの detached クレートであり、CI の fmt/clippy/test がまだ届いて\n  いない（設計監査 C27）。**stage-1 スコープに含める**: `tools/lint` への\n  CI 3ステップ（fmt/clippy/自己テスト）追加（インタビュー Q7、選択肢 A）。\n  macOS CI ジョブ追加・`main` への push トリガー追加は本 intent には\n  含めず、後続 intent へ繰り延べる（インタビュー Q7、選択肢 E 相当の一部\n  不採択）。"
    }
  ],
  "obligations": {
    "strategy": "standard",
    "strategy_volume": [
      "Five to eight tests per component.",
      "Unit tests plus integration tests for key boundaries.",
      "Add E2E, performance, or security tests when requirements demand them."
    ],
    "scope_floor": [
      "Keep the existing test suite green.",
      "This scope adds no extra new-test floor beyond the selected test strategy."
    ],
    "combination_rule": "Apply every selected-strategy obligation and every scope-floor obligation; neither replaces the other, and a targeted scope regression may add the narrowest necessary test type beyond the strategy default."
  },
  "plan_profile": {
    "methodology": "tdd",
    "runner_step": "Verify the existing test runner/configuration and record the exact unit-scoped command.",
    "runner_ready_before_first_test": true,
    "testable_layers": [
      "Data model / database behavior",
      "Repository / data access",
      "Business logic",
      "API / endpoint",
      "Frontend behavior"
    ],
    "steps": [
      "Project structure and production configuration skeleton.",
      "Verify the existing test runner/configuration and record the exact unit-scoped command.",
      "Data model / database behavior - Red: write the failing tests and record the failing command output.",
      "Data model / database behavior - Green: implement only enough behavior to pass.",
      "Data model / database behavior - Refactor: improve the implementation while tests stay green.",
      "Repository / data access - Red: write the failing tests and record the failing command output.",
      "Repository / data access - Green: implement only enough behavior to pass.",
      "Repository / data access - Refactor: improve the implementation while tests stay green.",
      "Business logic - Red: write the failing tests and record the failing command output.",
      "Business logic - Green: implement only enough behavior to pass.",
      "Business logic - Refactor: improve the implementation while tests stay green.",
      "API / endpoint - Red: write the failing tests and record the failing command output.",
      "API / endpoint - Green: implement only enough behavior to pass.",
      "API / endpoint - Refactor: improve the implementation while tests stay green.",
      "Frontend behavior - Red: write the failing tests and record the failing command output.",
      "Frontend behavior - Green: implement only enough behavior to pass.",
      "Frontend behavior - Refactor: improve the implementation while tests stay green.",
      "Environment/build configuration.",
      "Documentation and traceability."
    ]
  },
  "input_sha256": "sha256:e4f36aa113753d3604df570f5ec3a0cb465d4b29d82a17a16efbb2ea8b993111",
  "contract_sha256": "sha256:303d9bb7b5d777d54a6761be9ed154d85d5bb3f2d6b9cce02f71f4ed1b3a4ff3"
}
```

