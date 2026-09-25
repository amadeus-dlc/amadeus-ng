# ドメイン層はドメインの概念で分ける — 技術駆動パッケージングの禁止

**裁定日**: 2026-09-25（オーナー「ドメイン層は技術駆動パッケージングは禁止です」「modules/core/command/domain/src ここはentities/,value-objects/みたいな区切り方はNG」）
**関連**: [module-visibility.md](module-visibility.md)（`pub mod` は名前空間として意味を持つ階層だけ）、
[domain-object-kinds.md](domain-object-kinds.md)（オブジェクトの**種類**の定義 — 置き場所の分け方ではない）、
[abstract-data-type.md](abstract-data-type.md)（1 ファイル 1 公開型）、
[ubiquitous-language.md](ubiquitous-language.md)
**根拠の調査**: [packaging-principles-research-20260925.md](../../packaging-principles-research-20260925.md)
**機械強制**: `cargo lint`（`domain-packaging` — `modules/core/command/domain/src` の下のディレクトリ名・ファイル名・インライン `mod` 名が、下記「禁止する名前」に完全一致したら所見。特定の型名が `_event` などで終わる区間は鳴らない）

## 原則

ドメイン層（`modules/core/command/domain/src`）のディレクトリとモジュールは、**ドメインの概念**で分ける。
**技術の種類（DDD のパターン名・実装の役割）で分けてはならない。**

```text
✕ 技術駆動 — パターンの種類で束ねる
domain/src/
  entities/        intent.rs, intent_execution.rs, ...
  value_objects/   stage_slug.rs, intent_id.rs, ...
  events/          intent_execution_event.rs, ...
  services/        ...

○ ドメイン駆動 — 境界づけられたコンテキスト → 型が所有するサブツリー
domain/src/
  orchestration/          intent.rs, intent_execution.rs, intent_execution_event.rs, ...
    intent_execution_event/   started.rs, gate_approved.rs, ...   (イベント族の変種)
  workflow_definition/    stage_slug.rs, stage_graph.rs, ...
    stage_node/               ...                                  (型に従属する部品)
  workspace/              ...
```

分け方は次の 2 段だけである。

1. **最上位は境界づけられたコンテキスト**（`orchestration` / `workflow_definition` / `workspace`）。
   ここだけが `pub mod` であり、ユビキタス言語の所属を示す名前空間になる（[module-visibility.md](module-visibility.md)）。
2. **その下のディレクトリは、特定の型が所有するサブツリーだけ**。イベント族の変種ファイル
   （`intent_execution_event/started.rs` — [module-visibility.md](module-visibility.md) §追記 2026-09-01）と、
   1 つの型に従属する部品（`stage_node/` など）がこれに当たる。ディレクトリ名は所有者の型名の snake_case にする。

## 禁止する名前

ドメイン層のディレクトリ名・モジュール名に、パターンや技術の役割を表す語を使わない。例:
`entities` / `entity` / `value_objects` / `value_object` / `vo` / `aggregates` / `aggregate` /
`domain_events` / `events` / `services` / `service` / `domain_services` / `repositories` / `repository` /
`factories` / `factory` / `models` / `model` / `types` / `common` / `shared` / `utils` / `util` /
`helpers` / `helper` / `misc`（`cargo lint` の検出対象と同じ一覧）。

`intent_execution_event/` のように**特定の型名**が `_event` などで終わるのは禁止の対象ではない。
禁止されるのは、種類そのものを名前にして複数の概念を束ねるディレクトリである。

## なぜか

- **変わる理由でまとめるため。** 1 つのドメイン概念への変更は、その集約・値オブジェクト・イベントを
  一緒に変える。種類で分けると、1 つの変更が `entities/`・`value_objects/`・`events/` に散らばる
  （Martin の CCP。Clio や PairSmell の実証研究でも、変更が散らばる分割ほど後で設計上の問題になる —
  [調査](../../packaging-principles-research-20260925.md) §3）。
- **モジュールはドメインの物語を語るため。** モジュール名はユビキタス言語の一部である（Evans
  『DDD Reference』Modules）。`value_objects` はドメインの語ではなく実装の語であり、
  [ubiquitous-language.md](ubiquitous-language.md) の「実装の都合から出た語を持ち込まない」にも当たる。
  Evans『Domain-Driven Design』の Modules の章は、これを「技術駆動パッケージングの落とし穴
  （Pitfalls of Infrastructure-Driven Packaging）」として戒めている（原文は今回未取得 — 記憶による）。
- **種類は置き場所ではなく型が語るため。** あるオブジェクトがエンティティか値オブジェクトかは、
  型の設計（[domain-object-kinds.md](domain-object-kinds.md)）が表す。ディレクトリで重ねて表すと、
  種類の見直し（値オブジェクトをエンティティへ昇格するなど）が移動を伴う破壊的変更になる。

## 禁止パターン

- ドメイン層に `entities/`・`value_objects/`・`aggregates/`・`events/`・`services/` などの
  種類別ディレクトリ・モジュールを作る
- 境界づけられたコンテキストの内側を、さらに種類別に分ける（`orchestration/value_objects/` など）
- 所有者の型が無い「置き場」としてのディレクトリ（`common/`・`shared/`・`utils/`）を作る

## 射程

- 対象は**ドメイン層**（`modules/core/command/domain/src`）。
- インターフェイスアダプタ層の `dto/`（[gateway-taxonomy.md](gateway-taxonomy.md) §5b）や、
  use-case 層の `port/`（[cqrs-boundaries.md](cqrs-boundaries.md) 規則 6）は、既存の裁定で
  置き方が定められた**技術境界**なので、本規則の対象外である。
