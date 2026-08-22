# contract-design-questions — Unit 間・外部境界の契約設計の確認質問

> Contract Design（Inception 2.8）の質問票。出典: `../units-generation/unit-of-work.md`（10 Unit）、
> `../units-generation/unit-of-work-dependency.md`（DAG と §4 統合点: Q7 = A〜D の 4 境界）、
> `../domain-design/components.md`（コンポーネントとポート）、`../domain-design/decisions.md`（ADR-001〜007）、
> `../requirements-analysis/requirements.md`（NFR1 upstream 互換・NFR3 監査完全性）、`docs/specs/deviations.md`
> （逸脱台帳 — upstream 互換面の変更管理）。
>
> 「契約」とは、境界をまたぐ 2 者の間の正式な取り決め（何が・どんな形で・どの手段で渡り、失敗時にどうなるか）。
> 本ステージは Unit 間の境界と、システムの外（Claude Code ハーネス）に見せる面を一度に扱う。

---

## Q1. 外部（システムの外に見せる）契約の範囲

本システムの利用者は Claude Code ハーネス（`.claude/` の設定・スキル・フック登録）で、バイナリ `aidlc` の
CLI 面を通じてやり取りする。

- A. **CLI 面を唯一の外部契約**とする（推奨）— 動詞集合（ROUTES）、directive の JSON（10 種・28KiB 上限・
  `continue_token`）、フック 4 本の stdin/stdout/終了コード、逐語文言、`AIDLC_*` 環境変数。正本は upstream
  仕様（D6）と 0b ゴールデン（U1）。SQLite ファイルや内部ポートは外部契約にしない
- B. A に加えて SQLite ファイル形式（journal / snapshot / checkpoint）も外部契約にする（他ツールが直接読む想定）
- C. 外部契約なし（ハーネスは同一システム内として扱い、契約化は内部境界のみ）
- X. Other (please specify)

[Answer]: A

## Q2. ポート trait（Repository / EventStore）の契約の書き方

- A. **Rust の trait シグネチャ**（fenced `rust`）を契約の正本にし、not-found の挙動・楽観 version 競合の
  エラー型・永続化エラーの型・トランザクション所有を doc コメントで規定する（推奨）— 層 = クレートなので
  シグネチャがそのまま強制される
- B. 言語中立の shared-schema（yaml）で操作と型を記述し、Rust は写像とする
- X. Other (please specify)

[Answer]: A

## Q3. ドメインイベント語彙と投影規則の形式化

U2（発行）⇄ U4（投影）の境界。1 ドメインイベント → upstream 監査行 N 行・状態ファイル差分。

- A. **AsyncAPI 風の yaml** でイベント名・ペイロード（フィールドと型）・投影先（監査行の見出し・フィールド順、
  状態ファイルの差分）を 1 表にまとめる（推奨）— U4 の実装とゴールデン突合の根拠になる
- B. イベント名とペイロードの列挙のみ（投影規則は functional-design に委ねる）
- X. Other (please specify)

[Answer]: A

## Q4. SQLite スキーマの契約

U3（所有）⇄ U4（差分読取）の境界。

- A. **DDL**（fenced `sql`）で journal / snapshot / checkpoint の 3 テーブルを固定し、`seq_nr` / `version` の
  単調性、チェックポイント単調性、同一 Tx 書込の制約を明記する（推奨）
- B. 概念スキーマ（列名と意味）のみ。DDL は U3 の実装時に決める
- X. Other (please specify)

[Answer]: A

## Q5. バージョニングと破壊的変更の方針

- A. **外部面（CLI・フック・文言）は D6 に従い逸脱台帳で管理**（破壊的変更 = 台帳登録 + ADR）。内部のポート・
  イベント・スキーマは stage-1（単一バイナリ・単一クローン）ではバージョン管理しないが、イベントと
  スナップショットに `schema_version`（整数）フィールドだけ**予約**し、追加フィールドは消費側が無視する
  （additive-safe）（推奨）
- B. イベント・スキーマに最初から明示バージョンと互換変換（upcaster）を持たせる
- C. 内部契約はバージョン管理も予約もしない（必要になったら考える）
- X. Other (please specify)

[Answer]: A

## Q6. 境界ごとのエラー・タイムアウト・リトライの挙動

- A. **ストア**: 楽観 version 競合は即 `Err`（ユースケースが 1 回だけ再水和して再試行、それでも競合なら CLI が
  エラー終了。ワンショット CLI で同時実行は稀）。I/O エラーは `ErrorKind` を保持して上げる（監査 C24 の趣旨）。
  **投影**: 冪等なので失敗は次回コマンド末尾のキャッチアップで修復（NFR3）。**CLI**: 終了コードとエラー文言は
  upstream 互換。タイムアウトはローカル I/O のみなので設けない（推奨）
- B. 競合時のリトライなし（即エラー終了、利用者が再実行）
- C. ストアに指数バックオフのリトライ（最大 3 回）を入れる
- X. Other (please specify)

[Answer]: A

## Q7. 契約の所有者（誰が仕様を持ち、破壊的変更を誰が合意するか）

- A. **ポート trait = 消費側（ユースケース層: U5/U6）が所有**し、実装側（U3）は準拠する（DIP の向きどおり）。
  **イベント語彙 = U2（ドメイン）所有**、**投影規則 = U4 所有**、**SQLite スキーマ = U3 所有**、
  **CLI・フック面 = U7 所有**で正本は upstream 仕様 + ゴールデン（U1）。破壊的変更は所有 Unit の Bolt で
  ADR を添えて合意（推奨）
- B. すべて提供側（実装する Unit）が所有する
- X. Other (please specify)

[Answer]: A（記号の意味を括弧書きで添えた再提示に対する回答）

## Consolidated Summary Confirmation

- 外部契約は CLI 面のみ（Q1 = A）: 動詞集合・directive JSON（10 種・28KiB・continue_token）・フック 4 本の入出力と終了コード・逐語文言・`AIDLC_*` 環境変数。正本は upstream 仕様（D6）+ 0b ゴールデン。SQLite ファイルと内部ポートは外部契約にしない
- ポート trait（Repository / EventStore）の契約は Rust の trait シグネチャが正本（Q2 = A）。not-found・楽観 version 競合・永続化エラー・Tx 所有は doc で規定
- ドメインイベント語彙と投影規則は AsyncAPI 風 yaml で 1 表（Q3 = A）: イベント名・ペイロード・投影先（監査行の見出し/フィールド順・状態ファイル差分）
- SQLite スキーマは DDL で固定（Q4 = A）: journal / snapshot / checkpoint、seq_nr / version / チェックポイントの単調性、同一 Tx 書込
- バージョニング（Q5 = A）: 外部面は逸脱台帳（破壊的変更 = 台帳登録 + ADR）。内部は stage-1 ではバージョン管理せず、イベント・スナップショットに `schema_version`（整数）だけ予約。追加フィールドは消費側が無視
- エラー挙動（Q6 = A）: 楽観 version 競合は即 Err → ユースケースが 1 回だけ再水和して再試行、なお競合なら CLI がエラー終了。I/O エラーは ErrorKind 保持。投影は冪等で次回キャッチアップ修復。CLI の終了コード・文言は upstream 互換。タイムアウトなし
- 所有者（Q7 = A）: ポート trait は使う側（ユースケース層 U5/U6）、イベント語彙は U2、投影規則は U4、SQLite スキーマは U3、CLI・フック面は U7（正本は upstream 仕様 + U1 ゴールデン）。破壊的変更は所有 Unit の PR で ADR を添えて合意

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
