# ドメインは永続化知識から中立 — serde・ストア trait・ジャーナル語彙を書かない

**裁定日**: 2026-08-30（オーナー「ドメインに永続化知識を含めるな。集約はどんな永続化知識からも
中立なのです。全部撤回せよ」— B12 で `IntentMaterial`（serde 復号の中間表現）が domain に
追加されたことを契機に、従来 domain に居た serde 設定一式ごと撤去を裁定）
**関連**: [infrastructure-layer.md](infrastructure-layer.md)（相手方契約を知る層はアダプタ）、
[upstream-contracts.md](upstream-contracts.md)（境界で変換）、
[cqrs-boundaries.md](cqrs-boundaries.md)（共有部品は側の独立を DRY に優先 — 側ごと専用化）
**機械強制**: **クレート依存**（`core-command-domain` の `Cargo.toml` に serde・
event-store-adapter-rs が現れないこと — 違反はビルドで落ちる）+ レビュー基準
**上書き**: domain-design decisions.md の「ドメイン層に serde と chrono が入る」トレードオフ
受容（2026-08-2x）のうち serde の受容を撤回（chrono = 時刻の値は永続化知識ではなく残る）

## 原則

ドメイン層（集約・エンティティ・値オブジェクト・ドメインイベント）には**いかなる永続化知識も
書かない**:

- `#[derive(Serialize, Deserialize)]`・`#[serde(...)]` 属性・serde 依存そのもの
- ストアライブラリの trait 実装（`AggregateId` 等 — aid 列の組み方はストアの語彙）
- ジャーナルの型判別子・列名・直列化形式の定数（`EVENT_MANIFEST` 等）
- 復号用の中間表現（`IntentMaterial` のような serde の双子）

## どこに書くか

**書く側の永続化モデル（DTO）はインターフェイスアダプタ層が所有する。**

- アダプタが domain の**公開アクセサ**で読み、自前の serde DTO へ写して書く。
- 復号は DTO で受け、domain の**検査付き再構成コンストラクタ**（`from_*` — Always Valid）へ
  渡す。検査を迂回する構築口は存在しない（担保の場所が domain の serde 属性から
  アダプタの変換関数へ移るだけで、担保自体は落ちない）。
- ストア trait（`AggregateId` 等）はアダプタが**自前のラッパ型**で実装する（境界で変換 —
  upstream-contracts.md）。
- **読む側（RMU）は自前の復号 DTO を持つ**（cqrs-boundaries「共有部品は側の独立を DRY に
  優先 — 側ごと専用化」）。書き手と読み手のワイヤ形式の一致は横断適合テストで固定する。

## なぜか

直列化属性は「このフィールドはこの名前・この形でバイトになる」という**ストアとの契約**であり、
ドメインの語彙ではない。domain に置くと、ワイヤ形式の変更理由がドメイン型の変更理由になり、
層の変更理由が混線する。また serde の derive/try_from は「検査を迂回する復号口を塞ぐ」ための
中間表現（memento の双子）を次々に domain へ呼び込み、ドメインが永続化の都合で肥える。

## 対象外

- `chrono`（時刻の値 — 永続化知識ではない）
- **`[dev-dependencies]` の `serde_json`（`Value` 読取のみ）** — ITF 準拠テストが Quint トレース
  JSON という**外部フィクスチャを読む手段**であり、ドメイン自身の永続化知識ではない（テストの
  `unwrap` 許容と同じ類の対象外）。derive を使う dev 依存は不可。機械強制の対象は
  **`[dependencies]`**（プロダクション面）であり、そこに serde / event-store-adapter-rs が
  現れたら違反。event-store-adapter-rs は dev にも残さない — 改訂 9 以降、domain 型は
  `AggregateId` を実装しないため正当なテスト用途が存在しない（2026-08-30 裁定）
- ドメインイベント・スナップショット**という概念**（decide / apply / snapshot は ES の
  ドメイン語彙。禁止されるのはその**直列化の記述**）
- テストコードがアダプタの DTO を使って固定するワイヤ形式の検証
