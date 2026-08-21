# ADR 0001: 正準 JSON シリアライザの一本化

- **ステータス**: **Accepted**（2026-08-22 オーナーレビューで確定: バイト一致互換を維持する — 本家と同じバイト列を出す。敵対的レビュー 1 巡反映済み）
- **日付**: 2026-08-22
- **対応**: `00-policy.md` §3 A2。リスク R1（正準 JSON の仕様間分裂）・R7（ダイジェスト不一致）の手当て
- **関連**: ADR 0002（文言カタログ）、`01-domain-model.md` §7-2（Published Language クレート層）

## コンテキスト

upstream では、ハッシュ、drift guard のバイトパリティ、`runtime-graph.json` の決定性（「同一監査ログ → バイト同値」）、`stage-graph.json` の固定 28 フィールド順、冪等判定と freshness 検査が、すべて JSON 直列化の決定性の上に乗っている。方針分析の段階で既に 5 通りの直列化推奨が仕様間に併存しており（R1）、一本化せずに実装を始めると全ゴールデンの再生成と全ハッシュの再計算を強いられる。

**ハッシュ入力には 2 族ある**（凍結コーパスで確定済み）:

- **正準族（hashObject）**: `canonicalize` が全オブジェクトキーを**再帰的にソート**してから直列化し、`sha256:` プレフィックス付きでハッシュする（`09-cli-tools.md:619`）。`contract_sha256`・approval fingerprint がこの族。
- **非正準族（`sha256(JSON.stringify(x))`）**: 挿入順・コンパクトのまま直列化してハッシュする。bundle hash・`directiveHash`・route hash（`02-orchestration-engine.md:179-181`）、ルール配送の冪等 digest（`07-hooks.md:357`）がこの族。

JS と Rust の間には放置すると静かに乖離する差が 3 つある。

1. **キー順**: JS のオブジェクトは基本挿入順だが、**integer-like キー（"0", "2", "10" 等）は挿入順に関係なく数値昇順で先頭に並ぶ**（ECMAScript の規定。node 実測で確認済み）。serde_json の `Value` は既定でソート順（BTreeMap）であり、そのまま使うと挿入順と割れる。型付き struct はフィールド宣言順で直列化される。
2. **数値表記**: `JSON.stringify(1.0)` は `"1"`、serde_json の f64 `1.0` は `"1.0"`。さらに指数表記の書式も異なる（JS は `1e+21`・`-0` は `"0"`、serde_json/ryu は `1e21`・`-0.0`）。
3. **体裁**: 代表的なディスク成果物は 2 スペースインデント＋末尾改行（`JSON.stringify(parsed, null, 2)` — `10-distribution-harnesses.md:380`、`03-state-audit-runtime.md:345`）。成果物ごとの網羅確認は残る。

## 決定

1. **単一クレート**（仮称 `canon-json`。内部クレート名は D6 の互換対象外）を新設し、契約 JSON の読み書きはすべてこのクレートを経由する。
2. **3 つの直列化プロファイル**を定義し、各プロファイルを upstream 実出力・実ハッシュとのバイト一致ゴールデンで固定する。
   - `contract-pretty`: ディスク成果物（`stage-graph.json` / `scope-grid.json` / `harness.json` / `runtime-graph.json` 等）と、**Markdown に埋め込まれる契約 JSON**（code-generation-plan.md の Testing Contract フェンス付きブロック、センサー詳細ファイルの fenced JSON）。体裁は upstream 実測値（代表: 2 スペース＋末尾改行）。
   - `contract-compact`: stdout の 1 行 JSON（Directive）と、**非正準ハッシュ族の入力形**（bundle / directiveHash / route hash / ルール配送冪等 digest）。キー順は型・構築順（= upstream の挿入順）。
   - `hash-canonical`: **hashObject 互換のハッシュ入力形に限定**。全オブジェクトキー（struct のフィールド名も対象）を再帰的にソートして直列化する。正準化は serde 出力の再正準化か専用 Serializer で行う。ソートの照合順序は A5 の契約表で固定する（JS `Array#sort` は UTF-16 コード単位順で、キーが ASCII に限る場合は Rust のバイト順と一致 — 契約キーが ASCII のみであることをゴールデン整備時に棚卸しする）。
3. **キー順の正はプロファイルが決める**。`contract-pretty` / `contract-compact` は型付き struct のフィールド宣言順（stage-graph の 28 フィールド順は struct 宣言で符号化）、動的マップは挿入順、`hash-canonical` のみ再帰ソート。serde_json の `preserve_order` フィーチャは Cargo のフィーチャ統合によりビルド全体に効くため、**ワークスペース全体で常時有効化**し、ソート順が必要な箇所は `BTreeMap` か hash-canonical の直列化時ソートを明示的に使う。integer-like キーが契約 JSON に存在しないことを棚卸し、存在する場合はその箇所の写像を個別定義する。
4. **整数は整数型で持つ**。契約型の数値フィールドは i64/u64 に固定し、f64 経由の `"1.0"` 化を型で防ぐ（E1）。浮動小数が本質のフィールドが契約に現れる場合は、ECMA-262 `Number::toString` 互換の数値ライタ（指数閾値 1e21 / 1e-6、`e+` 書式、`-0` の消去を含む）に差し替えて吸収する。該当フィールドの棚卸しを計測タスクに含める。
5. **直接呼び出しの禁止**。canon-json 以外のクレートからの `serde_json::to_string` / `to_string_pretty` / `to_vec` / `to_vec_pretty` / `to_writer` / `to_writer_pretty`、および契約経路での `to_value` は clippy の `disallowed-methods` で拒否する。`Value` の `Display` / `format!` 経由は lint では塞げない**残余ホール**であり、レビュー規約とゴールデンで補完する（upstream が biome の import 制限で同種の限界を明文化したのと同じ扱い — `00-overview.md:327-329`）。
6. **検証はゴールデン先行**。upstream dist の実成果物と、upstream ツールの実行採取＋**ピン留めコミットのソース読解**で仕様を写し取り、実出力・実ハッシュ値とのバイト一致テストを実装より先に整備する（方針書 R9 と同じ原則）。

## 帰結

- ドリフトガード・freshness・冪等判定・ハッシュの全系が単一実装に乗り、R1 は構造的に解消される。
- 浮動小数フィールドが契約に存在する場合、serde_json 既定フォーマッタの差し替えという実装作業が確定する（決定 4）。
- derive だけでは済まない箇所（プロファイル切替・再正準化・カスタムライタ）に少量のボイラープレートが生じるが、契約の所在を明示するコストとして受け入れる。

## 決定順序についての注記

`00-policy.md` §3 は A8 → A1 → A3 → A2 の順を基本とするが、本 ADR の規範内容（プロファイル・キー順・数値の規則）は D6 で凍結済みの upstream 資産に対する契約であり、A8（リポジトリ構成）・A1（配布モデル）と独立に確定できる。クレート名・配置・clippy 設定の適用単位は非規範とし、A8 確定時に追認する。

## 検討した代替案

- **RFC 8785（JCS）**: 不採用。upstream が使っておらず、D6（aidlc 互換）が優先。標準準拠より実測互換。
- **BTreeMap（ソート順）全面採用**: 不採用。JS の挿入順と乖離し、既存成果物とのバイト一致が原理的に取れない。
- **成果物ごとの個別ライタ容認**: 不採用。R1 の再演になる。

## hash-canonical の受入条件（2026-08-22 追記 — PR #2 レビュー反映）

バイト一致互換は、次の仕様固定とゴールデン表が揃うまで**受入れない**。

1. **仕様の固定項目**（ピン留めコミット `3c3146cf` のソース読解で確定し、本 ADR に追記する）: (a) `canonicalize` の再帰キーソートの照合順序（JS `Array#sort` 既定 = UTF-16 コード単位順。契約キーが ASCII のみであることの棚卸し込み）、(b) `JSON.stringify` の数値表記（ECMA-262 `Number::toString` — 指数閾値 1e21 / 1e-6、`e+` 書式）、(c) **非有限数（NaN / ±Infinity）の `null` 化**、(d) **`-0` の `"0"` 表記**、(e) 文字列の最小エスケープ集合。
2. **入力クラス別ゴールデン表**: 上記 (a)〜(e) の各入力クラス（ネスト・integer-like キー・非有限・負ゼロ・非 ASCII 文字列等）について upstream の実出力文字列と実ハッシュ値を表として固定し、契約テストに載せる。
3. **成果物ごとの体裁**（インデント・末尾改行・コンパクト形）も同じゴールデンで固定する。

## 未確定事項

- 上記受入条件 1 の (a)〜(e) の具体値と、条件 2〜3 のゴールデン表の採取（stage-0 環境での実行採取＋ソース読解）。完了後に本 ADR の該当節を「確定」へ更新する。
