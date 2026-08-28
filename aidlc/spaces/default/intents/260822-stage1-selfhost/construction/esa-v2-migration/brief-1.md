# brief-1 — 委任 1: ドメインの Conformist 化（event-store-adapter-rs v2.0.0）

Conversation language: 日本語。**本作業は AI-DLC のステージ外**（オーナー裁定 — 乗り換えは
AIDLC を使わないモードで直接進める）。ただし規律は同じ: TDD、検査全通過、報告ファイル。

## 目的

ADR-010（`inception/domain-design/decisions.md`）の乗り換え第 1 段。**ドメイン型が本家
event-store-adapter-rs v2.0.0 の trait（`Event` / `Aggregate` / `AggregateId`）を直接実装する**
状態にする。腐敗防止層は置かない（オーナー裁定）。ストアの差し替えは委任 2（本委任ではやらない）。

## 先に読むもの

1. `inception/domain-design/decisions.md` の **ADR-010**（乗り換えの全裁定）と ADR-006
2. `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/` — 特に
   `upstream-contracts.md`（借り物の契約を曲げない）/ `ubiquitous-language.md`
   （Published Language）/ `abstract-data-type.md` / `factory-naming.md` / README の衝突優先順
3. 本家 v2.0.0 の docs.rs / ソース（`lib/src/types.rs` の trait 定義が正）

## やること

1. **依存追加**: `event-store-adapter-rs = "=2.0.0"`（**完全固定** — 本家スキーマに結合するため。
   feature は本委任では不要 — memory バックエンドは常時コンパイルされる）、`serde`（derive）、
   `chrono`。`Cargo.toml` の workspace 管理に従う。
2. **`WorkflowExecutionEvent`** を本家 `Event` trait に適合させる:
   - イベント ID の新設（`type ID` の境界は本家定義を実測して従う。採番はイベント生成時）
   - `aggregate_id()`（既存 `intent_id()` との関係は本家契約が正）
   - `seq_nr() -> usize`（u64 から戻す）
   - `occurred_at() -> &DateTime<Utc>`（文字列から変更。ISO 8601 文字列が要る境界は
     変換で得る）
   - `is_created()`（genesis = `Started` のみ true）
   - `Serialize` / `Deserialize` / `Clone` ほか本家の境界すべて
3. **`WorkflowExecution`** を本家 `Aggregate` trait に適合させる:
   - `id()` / `seq_nr() -> usize` / `version() -> usize` / `last_updated_at() -> &DateTime<Utc>`
     （最終更新は apply で維持する新フィールド）
   - `Serialize` / `Deserialize`。**FSM の規律（ADR-002）は崩さない** — serde は表現の
     写しであってコマンド迂回の口ではない（フィールドを pub にしない。serde は
     private フィールドのまま derive できる）
4. **`IntentId`** を本家 `AggregateId` に適合させる（`type_name()` / `value()`）。
   `value()` は我々の規則（tell-dont-ask）が禁じる名前だが、**外部 trait の実装は
   Published Language への準拠**であり例外。doc コメントに一行理由を書くこと
   （`ubiquitous-language.md` の例外作法）。
5. **usize 化の波及**を全部直す: ワイヤ（event_wire / state_wire）、ITF 準拠テスト、
   契約テスト、`GlobalSeqNr` との接点。`to_u64` アクセサの扱いは実測で判断し報告に書く。
6. **時刻の波及**: `FakeClock`（epoch ms）→ chrono ベースへ。upstream 互換の ISO 8601
   文字列面（監査シャード等の Published Language）は**逐語維持** — DateTime からの変換で作る。
7. **証明**: 本家の **memory バックエンド**に対して我々の集約を実際に
   persist / 再構成するテストを書く（trait 適合のコンパイル通過だけでは不十分。
   genesis → イベント数件 → スナップショット + replay での復元一致まで）。

## 触ってはいけないもの

`EventStoreImpl` / `schema.rs` / ローカル `EventStore` trait / `JournalReader`（委任 2 で
差し替え・削除する。**本委任では現状のまま共存させ、全テストを緑に保つ**。u64/usize の
境界が生じたら変換で繋ぎ、箇所を報告に列挙）。`docs/**` と設計文書の同期はコンダクタ。
`.claude/**` / `scripts/**` / `formal/**`。`git add/commit/push` はしない。

## 検査（全部通す）

cargo fmt --all --check / clippy --workspace --all-targets -- -D warnings / cargo lint /
PROPTEST_RNG_SEED=20260823 cargo test --workspace（674 全緑を維持 + 新規テスト）/
bash scripts/quint-gate.sh

## 報告

`construction/esa-v2-migration/developer-report-1.md`。before/after、本家 trait の実測定義、
判断（イベント ID の採番方式・last_updated_at の維持規則・to_u64 の扱い）、
記述が実態と食い違う箇所の列挙（C3 の u64 → usize 再改訂を含む）、検査結果、設計質問、未了。
最終応答は 10 行以内（内容はファイルが正）。
