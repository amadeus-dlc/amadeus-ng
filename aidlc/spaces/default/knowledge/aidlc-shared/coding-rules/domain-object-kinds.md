# ドメインオブジェクトの種類 — エンティティ・値オブジェクト・ファーストクラスコレクション・ドメインイベントが基本、それ以外は人間の裁定

**裁定日**: 2026-09-02（オーナー規律「ドメインモデルの実装（ドメインオブジェクト）はエンティティか、
値オブジェクトが基本です。ドメインサービスを作るときは人間の裁定が必須。エンティティにはローカル
エンティティか、集約のルートエンティティ（グローバルエンティティ）があります。また、配列やコレクションを
隠蔽するためのファーストクラスコレクションを作ることがあります。これ以外のドメインオブジェクトを
実装したい場合は、必ず実測ありの問題と対策内容付きで人間の裁定にかけてください」）。同日追補「あと、ドメインイベントもありました。これもドメイン層ですね」— ドメインイベントを 4 種目として明記
**関連**: [domain-services.md](domain-services.md)（ドメインサービスは最後の手段 — 本規則で「人間の
裁定が必須」に強化）、[abstract-data-type.md](abstract-data-type.md)（土台）、
[aggregate-references.md](aggregate-references.md)、[aggregate-commands.md](aggregate-commands.md)、
[ubiquitous-language.md](ubiquitous-language.md)
**機械強制**: レビュー基準

## 原則

ドメインモデル（`modules/core/command/domain`）に実装してよいドメインオブジェクトの種類は、
既定では次の 4 つだけである。

| 種類 | 何か | 本リポジトリの例 |
| --- | --- | --- |
| **エンティティ** | 識別子で同一性が決まるオブジェクト。**集約のルートエンティティ（グローバルエンティティ）** — 自前の ID を持ち、集約の外から ID で参照される — と、**ローカルエンティティ** — 集約の内側でだけ識別される — の 2 種 | ルート: `Intent` / `IntentExecution` / `WorkflowDefinition` / `CompiledDefinition`。ローカル: 集約内で位置や slug で識別される要素（`StageEntry` 等） |
| **値オブジェクト** | 属性の値で同一性が決まる不変のオブジェクト（Domain Primitive を含む — `parse` が唯一の入口の AVDM） | `StageSlug` / `IntentId` / `StageIndex` / `DefinitionRevision` / `ScopeMetadata` / `ScopeCost` |
| **ファーストクラスコレクション** | 配列・コレクションを隠蔽し、そのコレクションに対する操作と不変条件を所有する型 | `Checkboxes` / `OrderedAuditEvents` / `StageGraph` / `ScopeGrid` |
| **ドメインイベント** | **エンティティの一種** — イベントごとに自前の識別子 `XxxEventId` を持つ（オーナー裁定 2026-09-02）。集約のコマンドが返す「起きた事実」の記録で、内容（値）を運び、集約を埋め込まない。1 コマンド 1 イベント（ADR-002 / [aggregate-commands.md](aggregate-commands.md)）。形は `XxxEvent { id: XxxEventId, aggregate_id: XxxId, .. }` — **集約の ID をイベントの `id` に流用してはならない** | `IntentExecutionEvent`（`Started` / `GateApproved` …）/ `IntentEvent::Created` / `WorkflowDefinitionEvent::Defined` / `CompiledDefinitionEvent` |

- **ドメインサービスを作るときは人間の裁定が必須**である。[domain-services.md](domain-services.md) の
  「どの型も所有できない操作だけ」という条件を満たすと自分で判断しても、置く前に裁定にかける
  （既存のドメインサービスの改修は対象外。新設が対象）。
- **上記以外の種類のドメインオブジェクト**（例: `Manager` / `Helper` / `Context` / `Policy` /
  `Strategy` / `Resolver` の類、判断だけを持つ型、手続きの束としてのモジュール、集約と構造同一の
  写し）を実装したいときは、**実測ありの問題**（何が・どのコードで・どう困っているか）と
  **対策内容**（何を作り、なぜ 3 種では表現できないか）を添えて**人間の裁定にかける**。
  裁定前に実装しない。
- ドメインイベントは**エンティティ**であり、自前の `XxxEventId` と、どの集約の事実かを示す
  `aggregate_id: XxxId` を**別々のフィールド**で持つ。`Started { id: IntentExecutionId }` のように集約 ID を
  イベントの `id` に流用した形は誤り（オーナー指摘 2026-09-02 — b39 で作り込んだ誤りで、是正 Bolt で直す）。
- **新しい**ドメインイベントは**集約のコマンドの戻り値**としてだけ生まれる（集約の再構成経路 `replay` /
  `apply_event` はイベントを**作らない**。保存済みイベントを永続化 DTO から**検査付きで復号**するのは新しい
  事実の生成ではなく、アダプタ層・RMU の正当な仕事 — [domain-persistence-neutrality.md](domain-persistence-neutrality.md)）（
  イベント族は enum + 変種ペイロードの形 — [aggregate-commands.md](aggregate-commands.md) /
  [module-visibility.md](module-visibility.md) §追記 2026-09-01）。「イベントっぽい型」を集約の外で
  組み立てるのは、この種類ではなく 5 段目の「それ以外」に当たる。

## 判定フロー

```text
1. それは識別子で同一性が決まるか？
   ├─ Yes → エンティティ。集約の外から ID で参照されるか？
   │         ├─ Yes → 集約のルートエンティティ（自前の ID、Repository はこの ID で引く）
   │         └─ No  → ローカルエンティティ（集約の内側でだけ識別）
   └─ No  → 次へ
2. 属性の値で同一性が決まる不変の値か？
   ├─ Yes → 値オブジェクト（`parse` / `new` が入口、`Eq` は意味論で）
   └─ No  → 次へ
3. 配列・コレクションを隠蔽し、その操作と不変条件を持たせたいか？
   ├─ Yes → ファーストクラスコレクション
   └─ No  → 次へ
4. 集約のコマンドが返す「起きた事実」の記録か？
   ├─ Yes → ドメインイベント（1 コマンド 1 イベント、内容を運ぶ、集約を埋め込まない）
   └─ No  → 次へ
5. どの型も所有できない操作か？
   ├─ Yes → ドメインサービス候補 — **人間の裁定を仰ぐ**（自分で置かない）
   └─ No  → 所有すべき型のメソッドにする（domain-services.md）
6. それでも 1〜5 のどれでもない型が要ると考えるなら
   → 実測ありの問題 + 対策内容を添えて**人間の裁定**。裁定前に実装しない
```

## 既に裁定済みの周辺概念（本規則の対象外）

- **集約** — 種類ではなく境界（ルートエンティティ + その内側のローカルエンティティ・値オブジェクト・
  ファーストクラスコレクション）。
- **判断型**（`NextDecision` / `JumpDirection` 等） — 集約のクエリの**戻り値**としての値オブジェクト。
  判断そのものは集約が持つ（project.md Corrections「集約は FSM」）。

## 禁止パターン

- 4 種のどれでもない型を「便利だから」「upstream がそう呼んでいるから」で domain に置く
- ドメインサービスを裁定なしに新設する（「純関数だから」「どの型も所有できないと思ったから」は理由にならない）
- 集約と構造同一の memento 双子・`Material` のような復号中間表現（[aggregate-commands.md](aggregate-commands.md)
  2026-08-30 裁定で撤去済みの形を再び作る）
- 裁定にかける際に実測（問題が起きているコード・症状）を添えない
