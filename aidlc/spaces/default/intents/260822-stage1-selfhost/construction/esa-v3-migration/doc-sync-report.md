# doc-sync-report — Bolt B7（ADR-010 / event-store-adapter-rs v3.0.0 EventEnvelope API 乗り換え）後の設計文書同期

> 実施日 2026-08-29。ブランチ `bolt/b7-esa-v3-event-envelope`。
> 正とした順序: (1) `brief-1.md`（固定裁定 1〜9 + 2026-08-29 改訂ブロック）、
> (2) `developer-report-1.md` §2（裁定ごとの実施箇所）・§4-(f)（未解消のドリフト 2 点）、
> (3) B6 の `doc-sync-report.md`（同期スタイルの前例）。
>
> **新しい設計判断はしていない。** developer-report-1 が報告したコードの実態（trait 削除・
> 型の新設/削除・署名変更）を文書へ反映しただけである。docs/specs は B6 と同じ家内書式
> （`~~打ち消し~~ → **失効（2026-08-29 / ADR-010・Bolt B7）**: ...`）で日付付き失効注記を
> 重ねた。ADR-010・contract-summary.md・functional-spec.md も過去の記録は書き換えず、
> 追記ブロックを積んだ。コード・`formal/**`・`Cargo.*`・coding-rules・memory は未変更。
> `git add` 済み、コミットは 1 本（push はしていない）。

## 1. 変更したファイルと件数

| # | ファイル | 変更行数 | 主な変更 |
|---|---|---|---|
| 1 | `docs/specs/01-domain-model.md` | +1/-1 | 集約の状態 17 → **16 属性**（`version` 列の除去。楽観ロック版数は集約の外、`RehydratedWorkflowExecution` が持ち回る）を B6 の 16→17 注記に重ねて記録 |
| 2 | `docs/specs/10-orchestration.md` | +2/-2 | ドメインイベントの封筒記述（`WorkflowExecutionEventId` / `schema_version` 込みの旧形）を失効させ本家 `EventEnvelope` へ、`WorkflowExecutionRepository` 行の `store`/`find_by_id` 署名と `v2.0.0`→v3.0.0 を更新 |
| 3 | `docs/specs/11-workspace.md` | +1/-1 | `WorkflowExecutionRepository` 行の `store`/`find_by_id` 署名（`expected_version` 引数・`RehydratedWorkflowExecution` 戻り値）と `v2.0.0`→v3.0.0 を更新 |
| 4 | `docs/specs/deviations.md` | +1/-1 | D4 行に v2.0.0→v3.0.0 の再乗り換えを追記（`manifest` 列追加以外、観測可能な差は増えていない） |
| 5 | `.../inception/contract-design/contract-summary.md` | +22/-0（3 箇所） | C3（trait 全文の直後に v3 形シグネチャの追記ブロック）、C5（`schema_version`/`WorkflowExecutionEventId` の失効と manifest 後継の追記）、C6（ピン更新・`manifest` 列・`occurred_at` ナノ秒の追記） |
| 6 | `.../inception/domain-design/decisions.md` | +33/-0 | ADR-010 末尾に「追記 2026-08-29（Bolt B7）」ブロック — 本家 v3.0.0 の背景、`=3.0.0` 乗り換え、version の持ち回り、TOCTOU 経緯の要約、`persist_event_and_snapshot`、manifest 後継 |
| 7 | `.../upstream-request-esa-event-envelope.md` | +7/-0 | 冒頭に「結果（2026-08-29 追記）」注記 — 本家 v3.0.0 が要望の方向で実装・リリースされ B7 で採用済みである旨 |
| 8 | `.../construction/u3-event-store-repository/functional-design/functional-spec.md` | +76/-14 | 新規 B7 バナー（ポート署名・`check_preconditions` の消滅・ITF 再生先の 3 点を先出し要約）+ §2 ポート署名・§3.1 store フロー・§3.2 find_by_id・§3.5 ピン/manifest・§4 ワイヤ形式（`schema_version`/`WorkflowExecutionEventId`/属性数）・§5 ITF 再生先の 6 箇所へ個別の B7 追記 |

**8 ファイル**（`git diff --stat` 実測: 166 insertions / 20 deletions。監査シャードの自動追記を除く）。

## 2. developer-report-1 §2（固定裁定 1〜9）の反映先

| 裁定 | 内容 | 反映先 |
|---|---|---|
| 1 | `=3.0.0` ピン | 10 号 §3・11 号 §3・deviations D4・contract-summary C6・functional-spec §1/§3.5 |
| 2 | ドメインイベントの payload 純化（封筒削除・`WorkflowExecutionEventId` 削除） | 10 号 §2.1・contract-summary C5・decisions ADR-010 追記・functional-spec §4.1 |
| 3 | seq_nr / 連続性検証はドメイン責務のまま | functional-spec §2（`apply_event` 署名は本 Bolt で変更なしのため追記不要と判断） |
| 4 | version を集約と memento から削除（2026-08-29 改訂版 = `RehydratedWorkflowExecution`） | 01 号・10 号 §2.1/§3・11 号 §3・contract-summary C3・decisions ADR-010 追記・functional-spec §2/§3.1/§3.2/§4.2 |
| 5 | manifest 定数 | contract-summary C5/C6・decisions ADR-010 追記・functional-spec §3.5/§4.1 |
| 6 | `JournalReader` ポートの戻り値（`JournalEntry`） | contract-summary C3 |
| 7 | 不変（rowid カーソル・checkpoint 表等） | 変更なし（不変なので追記対象外と判断） |
| 8 | v3 の新契約への追従（`ContractViolation`） | functional-spec §3.1 |
| 9 | Quint モデルは変更しない | functional-spec 新規バナー・§5 |

developer-report-1 §4-(f) が名指した 2 点のドリフト（C6 の memento 属性数 17→16、C5 の
`schema_version` 予約フィールド）は、上表の裁定 4・5 の反映と合わせて解消済みである。

## 3. 検収 grep の実行結果

ブリーフ指定の acceptance grep をそのまま実行した:

```text
grep -rn "WorkflowExecutionEventId\|schema_version\|EventStoreImpl" docs/specs/
```

6 行がヒットするが、すべて失効注記の中の取り消し線表記（`~~...~~`）内であることをスクリプトで
機械確認した（`~~...~~` の外側に出現する箇所を正規表現で走査し、0 件を確認 — 内訳は
`10-orchestration.md` 2 行、`11-workspace.md` 4 行）。ブリーフの許容条件「失効注記の中の取り消し線
表記内は許容」を満たすため **PASS** である。

ブリーフ item 3 の広域 sweep（`WorkflowExecutionEventId\|schema_version\|set_version\|EventStoreImpl\|=2\.0\.0`、
docs/specs 全体）も実行し、`10-orchestration.md` / `11-workspace.md` の該当箇所（上記と同じ）のみで
他ファイルへの残骸は無いことを確認した。

固定トークン（`BR*.*` / `FR*.*` / `C1`〜`C7` / `ADR-***` / `READY` / YAML キー等）は変更していない。

## 4. 判断に迷った点

1. **`docs/specs/deviations.md` D4 行を追加で更新した**（ブリーフの明示的な対象外）。
   ブリーフの grep 対象は `docs/specs/` 全体だが、item 3 の指定パターン（`WorkflowExecutionEventId
   \|schema_version\|set_version\|EventStoreImpl\|=2\.0\.0`）は D4 行の「v2.0.0」という表記に
   一致しない（`=2.0.0` はピン記法、D4 は地の文で「v2.0.0」と書いていたため）。しかし D4 は
   B6 でストア実装が v2.0.0 に変わった旨を記録した行であり、B7 で v3.0.0 へ再乗り換えした事実が
   同じ行に欠落するのは正確性を欠くと判断し、B6 と同じ「過去の記録は残し追記を重ねる」スタイルで
   1 文を追加した。ブリーフの所有ファイル一覧が `docs/specs/**` を包含しているため権限の逸脱ではない。
2. **`docs/specs` の記述を「失効注記」形式で統一し、地の文の直接書き換えは行わなかった**。
   ブリーフは「現在形の仕様文は現在の姿に直してよい — 大改訂になる場合のみ失効注記形式」としているが、
   今回の変更（trait 全廃・封筒構造の全面置換・`store`/`find_by_id` 署名変更）は概念レベルの
   大改訂に当たると判断し、B6 の前例（同種の乗り換えを失効注記で処理した）に倣って全箇所を
   失効注記形式にした。地の文の直接書き換えで済ませた箇所は無い。
3. **`functional-spec.md` の新規トップバナー（「⚠ 追加失効」）を B6 バナーとは別に新設した**。
   B6 バナーの内容を書き換えると B6 時点の記録が失われるため、B6 バナーはそのまま残し、
   その直後に B7 専用の追加バナーを挿入した。本文中の該当 3 箇所（§2 ポート署名・§3.1
   `check_preconditions`・§5 ITF 再生先）にも個別の追記注記を重ね、要約と詳細の二重化はあるが
   「重ねる」原則を優先した。
4. **markdown の太字ネスト崩れを 2 箇所で発見し修正した**（`docs/specs/10-orchestration.md` /
   `11-workspace.md` の `WorkflowExecutionRepositoryImpl<S>` 列）。既存の `**本家 ... を内包**`
   という太字区間の内側に `**v3.0.0**` を挿入すると `**a **b** c**` という不正なネストになるため、
   内側の太字は落として `~~v2.0.0~~ → v3.0.0（日付）` とした（外側の太字がそのまま v3.0.0 にも
   及ぶので視覚的な強調は失われない）。

## 5. 委任者から転送された実装担当の引き継ぎ事実に基づく追加修正（2回目）

委任者経由で実装担当からの引き継ぎ事実（memento の宣言順・manifest 値と版の規約・削除/署名変更の
確定リスト）が届いたため、これを正として再点検し、以下 4 件を追加修正した。
`git diff --stat` 実測: 4 ファイル・23 insertions / 18 deletions。

1. **`docs/specs/10-orchestration.md` の状態一覧（§2.1、直下 §3 の一つ上のブロック）に
   直し漏れがあった**。集約の 17 属性を列挙する行に `version` フィールドがそのまま残っており
   （1 回目の sweep は `schema_version`/`WorkflowExecutionEventId`/`EventStoreImpl`/`set_version`/
   `=2.0.0` の語彙パターンで探索したため、単独の `version` 語や「17 属性」という別表現の行を
   拾えていなかった）、17→16 属性への失効注記と `version` エントリの失効注記を追加した。
   同じ行にあった「`last_updated_at` は本家 `Aggregate::last_updated_at` の要求」という説明も
   `Aggregate` trait 自体が B7 で廃止されたため失効注記を重ねた（フィールド自体は不変）。
2. **`contract-summary.md` C3 の約束⑥「genesis は Gateway が写しに初期値 1 を載せる」が
   `Event::is_created()` 依存のまま失効していなかった**。本家 `Event` trait ごと `is_created()`
   が廃止され、genesis 判定は封筒の `seq_nr == 1` から導出する形に変わったため、失効注記を追加した。
   同じファイルの C6 DDL コメント「集約の状態の写し（17 属性）」も 16 属性へ修正した。
3. **`EVENT_MANIFEST` という内部定数名を仕様書に書いていた 4 箇所を、値と規約だけの記述へ書き換えた**
   （`contract-summary.md` C5/C6、`decisions.md` ADR-010 追記、`functional-spec.md` §3.1）。
   `core-interface-adapter` の実装詳細である定数識別子を仕様側が名指す必要はないため。
   `docs/specs/10-orchestration.md` と `functional-spec.md` の「manifest 定数」という言い回しも
   「manifest 列の値」へ統一した。
4. **manifest の版を上げる規約（1 文）を C5 に追記した** — 版はペイロードの読み方（デコード手順）が
   変わるときだけ上げ、イベント変種の追加のような additive-safe な変更では上げない。ADR-010 側は
   「版を上げる規約は C5 参照」の 1 行にとどめ、規約の正本を C5 に一本化した。

**確認して変更不要と判断したもの**: memento の宣言順（`intent_id / definition_id / definition_revision
/ stages / plan / overlay / conditional / checkbox / cursor / status / parked_at / autonomy /
approved / revision_count / seq_nr / last_updated_at`）は `functional-spec.md` §4.2 の列挙が
既にこの順（`version` は同じ位置に取り消し線で残置）と一致していたため変更していない。
`apply_event(seq_nr, occurred_at, &event)` の署名は所有ファイル内のどこにも誤った形で書かれておらず
（`decisions.md` / `components.md` にある `apply_event(&mut self, &Event)` は ADR-002 由来の抽象的な
設計原則の記述であり、`components.md` は所有ファイル外でもある）、追記不要と判断した。
`JournalEntry` / `RehydratedWorkflowExecution` の新設は 1 回目の同期で既に反映済みで、フィールド名も
引き継ぎ事実と一致することを確認した。
