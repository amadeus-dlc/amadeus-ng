# ADR 0005: リポジトリ構成と Cargo ワークスペース

- **ステータス**: **Accepted**（2026-08-22 オーナー決定: モノレポ・`modules/{core,shared,infra-io,app,harness}` 構成・層＝クレート）
- **日付**: 2026-08-22
- **対応**: `00-policy.md` §3 A8（他のすべての ADR の前提）
- **関連**: `01-domain-model.md` §7（レイヤ規約・層＝Cargo クレート）、ADR 0003 決定 3（`formal/` の配置）、D4 / D10

## コンテキスト

A8 は「本リポジトリ（docs）と Rust コードの置き場所」を決める、実装開始の唯一のブロッカーだった。決めることは 3 つ: (1) モノレポか分離か、(2) ディレクトリ構成、(3) クレート粒度（01 §7 が A8 送りにした「コンテキスト × 層のマトリクスか、層単位クレートか」）。

前提となる確定事項: 層の逆依存はビルドエラーで強制する（D4 — Cargo 依存宣言そのものが強制手段）。インフラは純粋部品群と infra-io に分割し、infra-io に依存できるのはアダプタ層と composition root のみ。upstream の検証済み不変条件「決定論エンジンは全ハーネスでバイト同一、違うのはシェルだけ」により、use-case と interface-adapter は**ハーネス中立（core 側）**であり、harness 側に残るのはデータ（マニフェスト）と薄いフックアダプタシムだけ（upstream の「A manifest is DATA」哲学の Rust 版）。

## 決定

1. **モノレポ**。本リポジトリ（amadeus-dlc/amadeus-ng）の `main` ブランチに仕様（`docs/`）とコードを同居させる。`docs` ブランチは役目を終えて廃止する。
2. **ディレクトリ構成**:

   ```text
   Cargo.toml            # workspace
   modules/
     core/               # ハーネス中立 = バイト同一エンジンの全部
       domain/           #   集約・Domain Primitive・純関数ドメインサービス
       use-case/         #   CLI 動詞＝ユースケース、ポート (trait) 定義
       interface-adapter/ #  Controllers / Presenters / Gateways
     shared/             # 純粋部品クレート群（全層から依存可）
       canon-json/       #   正準 JSON (A2)
       message-catalog/  #   文言カタログ (A3)
       audit-events/     #   監査イベントスキーマ (Published Language)
       directive-schema/ #   directive プロトコル (Published Language)
     infra-io/           # アトミック書込・spawn 基盤 (A4)・テレメトリ配線 (A10)
     app/
       aidlc/            # composition root: マルチコールバイナリ (A1)
     harness/
       claude/           # マニフェストデータ＋フックアダプタシム（フェーズ A はここのみ）
   formal/               # Quint モデル（ADR 0003 決定 3 のとおりルートへ — docs/specs/formal から移設）
   docs/                 # 仕様セット（既存のまま）
   ```

3. **クレート粒度: 層＝クレート、コンテキスト＝当面はクレート内モジュール**。ビルドエラーで強制したいのは層の逆依存であり、それは層クレートの分離だけで達成できる。7 コンテキスト × 3 層のマトリクスはフェーズ A では空クレートだらけになるため採らない。コンテキストの切り出しは所有権が要求したときに行い、**最初の切り出し候補は verification のレシート述語クレート**（B10: orchestration が依存する単一実装）と確定しておく。
4. **依存辺**（`Cargo.toml` の dependencies がそのまま規範。逆辺を書くとビルドエラー）:

   | クレート | 依存してよいもの |
   | --- | --- |
   | `core-domain` | `audit-events` / `directive-schema`（PL の型が Domain Primitive の置き場のため） |
   | `core-use-case` | `core-domain` ＋ PL 2 つ |
   | `core-interface-adapter` | `core-use-case` / `core-domain` / PL 2 つ / `canon-json` / `message-catalog` / `infra-io` |
   | `infra-io`・shared 4 つ | （相互含め）依存なしを維持 |
   | `aidlc`（bin） | `core-interface-adapter` / `infra-io`（composition root として全層を配線） |
   | `harness-claude` | `core-interface-adapter` |

5. **ライセンスは `MIT OR Apache-2.0` のデュアル**（Rust エコシステム慣習）を workspace 既定として設定する。LICENSE ファイル一式と NOTICE（upstream 出自の明記 — 00-policy §7）は初回公開（A1 の配布着手）時に整備する。
6. **`formal/` をリポジトリルートへ移設**し、docs 内の相対リンクを更新する（ADR 0003 決定 3 の履行）。

## 帰結

- 実装のブロッカーが解消され、フェーズ A（D10 の domain-model-first TDD）に着手できる。stage-0 セットアップ（オーナー担当）と並行可能。
- クレート名（`canon-json` 等）は workspace 内部名であり D6 の互換対象外。crates.io 公開時の一意名（プレフィクス付与）は A1 の公開判断と同時に決める。
- コンテキスト切り出しの遅延により、当面はクレート内モジュール境界がレビュー基準になる（コンパイラは層のみ守る）。切り出し時は 01 の正準用語から機械的にクレート名を導出する。

## 検討した代替案

- **リポジトリ分離（docs と code）**: 不採用。仕様と実装の相互参照（E4 トレーサビリティ、実験記録）が頻繁で、同一 PR で仕様と実装を直せるモノレポが合う。
- **コンテキスト × 層のクレートマトリクス**: 不採用（前述）。将来の切り出しは妨げない。
- **`crates/` 命名**: Rust 慣習だが、オーナーの `modules/` 案を採用（実害なし）。
