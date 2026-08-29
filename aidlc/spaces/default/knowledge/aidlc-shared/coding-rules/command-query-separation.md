# CQS — Query は `&self` + 戻り値、Command は `&mut self` + 戻り値なし

**裁定日**: 2026-08-23（オーナー）
**参考**: [fraktor-rs `cqs-principle.md`](https://github.com/j5ik2o/fraktor-rs/blob/main/.agents/rules/rust/cqs-principle.md)（オーナー指示により本プロジェクト向けに翻案）
**適用例**: U3（Bolt B5）— `EventStore::persist_event(&mut self) -> Result<(), E>` / `JournalReader::advance_checkpoint(&mut self) -> Result<(), E>` は適合。`WorkflowExecutionRepository::store(&self)` は違反で是正
**機械強制**: `cargo lint` ルール化予定
**関連**: [interior-mutability.md](interior-mutability.md)（CQS 違反を消す目的で `&self` + 内部可変性へ逃げてはならない）

## 原則

**CQS（Command-Query Separation）をできるだけ守る。**

> **集約のコマンドは本規則の対象外**（2026-08-29 オーナー裁定 — [aggregate-commands.md](aggregate-commands.md)）: イベントソーシングの集約は状態遷移で**必ず**ドメインイベントを戻り値で返す（1 コマンド 1 イベント）。イベントは書込の産物であり観測クエリではないため、CQS 違反ではなく個別許可も不要。ユースケース層以上の Command には本規則がそのまま適用される。

- **Query**: 状態を読み取る — `&self`、戻り値あり
- **Command**: 状態を変更する — `&mut self`、戻り値なし または `Result<(), E>`

## 判定フロー

```
1. このメソッドは状態を変更するか？
   ├─ No → &self + 戻り値（Query）
   └─ Yes → 次へ

2. 戻り値が必要か？
   ├─ No → &mut self + () または Result<(), E>（Command）
   └─ Yes → 次へ

3. CQS 違反なしでロジックが書けるか？
   ├─ Yes → 2 つのメソッドに分離する
   └─ No → オーナーの許可を得て違反を許容し、理由をコメントに書く
```

## 許容される違反（オーナー許可が前提）

| ケース | 理由 |
| --- | --- |
| `Vec::pop` 相当 | 読み取りだが状態前進が不可避 |
| `Iterator::next` 相当 | プロトコル上 `&mut self` + `Option<T>` が必要 |
| Builder のメソッドチェーン | チェーンのため自身を返す |
| `with_write` 相当のクロージャ実行 | ロック区間を閉じたまま結果を返す（[interior-mutability.md](interior-mutability.md) の `*Shared` パターン） |

> **読み取り意図でも状態前進が不可避なら `&mut self` が正解である。**
> 上記のケースは「`&mut self` で書くのが正しい設計」であって、CQS 違反を消す目的で
> `&self` + 内部可変性（`Cell` / `RefCell` / ロック）へ書き換えてはならない。
> それは [interior-mutability.md](interior-mutability.md) の「`&self` への偽装」に当たり、
> 借用チェッカの保護を失わせる。

## 禁止パターン

- `&mut self` + 戻り値を安易に使う（分離できるなら分離する）
- 「便利だから」を理由に違反する
- 内部可変性で `&self` + 戻り値に変えて違反を隠蔽する
