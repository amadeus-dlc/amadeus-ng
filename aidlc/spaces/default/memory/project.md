# Project-Level Rules

> Project-specific specialisation and corrections. Loaded after `org.md` and
> `team.md` as strict-additive guidance; contradictions with broader policy
> are rejected. Populated by practices-discovery and the self-learning loop.
>
> Use sparingly: most teams don't need a project layer. Reach for it
> only when this specific project needs stable, durable guidance beyond the
> team practice (for example, package-specific release checks or an additional
> regression suite for a legacy component).

## Way of Working

<!-- Project-specific specialisation. Example: -->
<!-- This monorepo requires package-scoped branch names and a package owner -->
<!-- review in addition to the team's normal merge policy. -->

## Walking Skeleton

<!-- Project-specific specialisation. Example: -->
<!-- The walking skeleton must exercise the legacy service adapter as well -->
<!-- as the new service boundary. -->

## Testing Posture

<!-- Project-specific specialisation. -->

## Deployment

<!-- Project-specific specialisation. -->

## Code Style

<!-- Project-specific specialisation. -->

## Tech Stack

<!-- Technology choices locked for this project. -->

## Decided

<!-- Decisions made in earlier stages that should not be re-asked. -->
<!-- Format: DECIDED: [decision] (Stage [slug], [date]) -->

## Scope Overrides

<!-- Custom scope rules for this project. -->

## Forbidden

<!-- Populated by practices-discovery affirmation gate. -->
<!-- Format: NEVER [behavior] (affirmed [date]) -->
<!-- Example: NEVER throw exceptions across service layer boundaries (affirmed 2026-05-17) -->

- NEVER 複数の PR を同時にオープンにしない（PR は直列運用、オーナー明言 (affirmed 2026-08-22)
2026-08-22。新規発見——実測の PR 履歴だけでは直列を断定できないが (affirmed 2026-08-22)
オーナー明言を第一級証拠として採用した。org.md 既定の trunk-based / (affirmed 2026-08-22)
squash-merge 一般則の再掲は当セクションに含めない——それらは org 層で (affirmed 2026-08-22)
既にロードされ機械強制の裏取りもないため、二重記載を避ける）。 (affirmed 2026-08-22)
- NEVER フィールドを既定で公開にしない（デフォルト private、公開はアクセサ (affirmed 2026-08-22)
経由。`cargo lint` no-public-fields ルールで機械強制、正本は (affirmed 2026-08-22)
`coding-rules/field-visibility.md`）。 (affirmed 2026-08-22)
- NEVER モジュールを既定で公開にしない（デフォルト private、公開は (affirmed 2026-08-22)
ファサードの `pub use` 経由。現状は既存の `unreachable_pub` deny lint (affirmed 2026-08-22)
（私有 mod 化により実効化）で機械強制されており、`cargo lint` への (affirmed 2026-08-22)
ルール化は未実施・予定である——開発者レビュー指摘により、 (affirmed 2026-08-22)
no-public-fields（フィールド専用）とは別の強制手段として書き分けた。 (affirmed 2026-08-22)
正本は `coding-rules/module-visibility.md`）。 (affirmed 2026-08-22)
## Mandated

<!-- Populated by practices-discovery affirmation gate. -->
<!-- Format: ALWAYS [behavior] (affirmed [date]) -->
<!-- Example: ALWAYS use Result<T,E> for fallible operations in service layer (affirmed 2026-05-17) -->

ALWAYS コード・仕様・レビューを書く前に、コーディング規則の正本 `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/`（オーナー裁定、1ルール1ファイル、インデックスは同ディレクトリの README.md）を読んで従う。規則はレビューと `cargo lint` で強制される (affirmed 2026-08-22)

- ALWAYS テストは t_wada 提唱の red-green-refactor（TDD）で書く。新規 (affirmed 2026-08-22)
プロダクションコードはレイヤーごとに red-green-refactor（失敗するテストを (affirmed 2026-08-22)
先に書く）で実装する。Quint モデル検査・ITF 準拠テスト・ゴールデンパリティ (affirmed 2026-08-22)
は TDD サイクルの外側の受け入れゲートとして維持し、TDD の red を代替 (affirmed 2026-08-22)
しない。テストピラミッド（ユニット層を厚く、結合・E2E層を薄く）を意識 (affirmed 2026-08-22)
した配分（定性のみ、比率は定めない）にする（オーナー明言 2026-08-22、 (affirmed 2026-08-22)
インタビュー Q1〜Q3 で確定）。 (affirmed 2026-08-22)
- ALWAYS PR は Bolt 単位で出す。Bolt ブランチは `main` へ squash-merge し、 (affirmed 2026-08-22)
コミット名は Bolt slug とする。PR は直列運用とし、オープンな PR は常に (affirmed 2026-08-22)
一度に1本のみとする（オーナー明言 2026-08-22）。 (affirmed 2026-08-22)
- ALWAYS GitHub Issue をそのまま intent とする（1 Issue = 1 intent）。 (affirmed 2026-08-22)
Issue のスコープを縮めない（オーナー明言 2026-08-22）。 (affirmed 2026-08-22)
- ALWAYS コード・仕様・レビューを書く前に、コーディング規則の正本 (affirmed 2026-08-22)
`aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/`（オーナー裁定、 (affirmed 2026-08-22)
1ルール1ファイル、インデックスは同ディレクトリの README.md）を読んで (affirmed 2026-08-22)
従う。規則はレビューと `cargo lint` で強制される (affirmed 2026-08-22)
（project.md ## Mandated に既に登録済み、affirmed 2026-08-22）。 (affirmed 2026-08-22)
- ALWAYS 会話および人間可読成果物は日本語で書く（コード識別子・固定トークンは (affirmed 2026-08-22)
英語のまま）（オーナー明言 2026-08-22、org.md/project.md 既定の適用）。 (affirmed 2026-08-22)
- ALWAYS マージ前に CI 3ジョブを全緑にする — check（`cargo fmt --all --check` (affirmed 2026-08-22)
→ `cargo clippy --workspace --all-targets -- -D warnings` → `cargo lint` (affirmed 2026-08-22)
→ `cargo test --workspace`）、quint（`scripts/quint-gate.sh`）、coverage (affirmed 2026-08-22)
（`scripts/coverage.sh`、絶対90%床 + PR 相対ゲート）（`.github/workflows/ (affirmed 2026-08-22)
ci.yml` 実測）。**この3ジョブは branch protection の required status (affirmed 2026-08-22)
checks として機械強制する**（インタビュー Q4、選択肢 A——`gh api` 実測で (affirmed 2026-08-22)
`main` に branch protection / ruleset が未設定であることが判明したため、 (affirmed 2026-08-22)
従来「ブロッキングゲートとして実行する」としていた文言を、実態（CI は (affirmed 2026-08-22)
走るが赤でもマージ可能）に合わせて修正し、機械強制の設定自体をオーナー (affirmed 2026-08-22)
裁定として確定した。設定作業は `evidence.md` の確定アクションを参照）。 (affirmed 2026-08-22)
- ALWAYS プロダクトコードでは `unwrap`/`expect` を使わない。テストコードのみ (affirmed 2026-08-22)
`clippy.toml`（`allow-unwrap-in-tests` / `allow-expect-in-tests`）で許容する (affirmed 2026-08-22)
（`Cargo.toml` workspace lints、オーナー規約）。 (affirmed 2026-08-22)
- ALWAYS 新規カスタム `cargo lint` ルールには検出力を証明する赤例テストを (affirmed 2026-08-22)
添える（Quint ゲートと同じ Definition of Done。coding-rules/README.md (affirmed 2026-08-22)
に明記、オーナー裁定）。 (affirmed 2026-08-22)
- ALWAYS `unsafe_code = "forbid"` を `[workspace.lints.rust]` として (affirmed 2026-08-22)
workspace 全体に適用する（従来はクレート個別 attribute のみで app スタブ (affirmed 2026-08-22)
に漏れがあった。インタビュー Q6、選択肢 C で workspace lints への昇格を (affirmed 2026-08-22)
確定）。 (affirmed 2026-08-22)
- ALWAYS `.github/workflows/ci.yml` に `permissions: contents: read` を (affirmed 2026-08-22)
明示する（least privilege。インタビュー Q6、選択肢 D で確定）。 (affirmed 2026-08-22)
- ALWAYS 依存追加・更新時は `cargo audit`（RustSec advisory DB）を CI で (affirmed 2026-08-22)
実行する。対象には `tools/lint` の独立 `Cargo.lock` も含める (affirmed 2026-08-22)
（インタビュー Q6、選択肢 A で確定）。 (affirmed 2026-08-22)
- ALWAYS ツールチェーンバージョンは `rust-toolchain.toml` で固定する (affirmed 2026-08-22)
（floating stable による CI 突然赤リスクの解消。インタビュー Q6、 (affirmed 2026-08-22)
選択肢 B で確定）。 (affirmed 2026-08-22)
- ALWAYS 実装は委譲し、メインセッション（Fable 5）は要求明確化・設計・計画・監査・レビュー・最終統合判断に温存する — 期待される資源節約が調整コストを上回るとき、スコープの明確な実行タスクをサブエージェントへ渡す。モデルは Sonnet（境界の明確な定型実装）/ Opus（複雑・高リスクで強い推論を要する実装）/ Fable 5 直接（安全にも効率的にも委譲できない極めて困難で密結合な作業）から選び、委譲オーバーヘッドが節約を上回る小さく明確なタスクはメインセッションに残す。委譲プロンプトには必ずスコープ・所有ファイル・受入基準・検証手順を書き、書込スコープは重複させない。完全な diff のレビュー・最終検証の確認・統合結果の受入判断はメインセッションの責任として残る（同文が docs/CLAUDE.md § Fable 5 Delegation Policy にもあるが、CLAUDE.md は Task/Agent 委譲時に配送されない — stage-graph.json の rules_in_context は memory/ の org・team・project・phases の 4 本のみ — ため、memory 層の本行を正本とする。オーナー裁定 2026-09-03） (learned 2026-09-03) <!-- cid:260822-stage1-selfhost:functional-design:2dfd9c437a3a668a1d044432979a735f900f8db7cfdc8093fa3c36864a27d30f -->
## Corrections

<!-- Project-specific corrections from human feedback. -->
<!-- Format: NEVER/ALWAYS [behavior] (learned [date]) -->
- ALWAYS 人間への質問文では、初出の術語・圧縮語（例: 「実行時採取」）をその質問文の中で平易に注釈してから選択肢を示す（術語のまま問うて差し戻された教訓） (learned 2026-08-22) <!-- cid:260822-stage1-selfhost:requirements-analysis:04954ca4c14c9b012f99211168f6eedf0ea2fc93d9fe1e1d1bb5bf6a7cb59d8c -->
- ALWAYS 集約は FSM として設計する — 状態としてのデータ・状態遷移（&mut self コマンド、ガード付き Err 拒否）・判断（クエリメソッド）を同じ集約型に閉じ込め、ユースケースは進行管理・フロー制御のみ（ビジネスロジック禁止）。導出ロジックを独立ドメインサービスやユースケースに置かない（オーナー統一ルール 2026-08-22、横展開） (learned 2026-08-22) <!-- cid:260822-stage1-selfhost:domain-design:16168d8ea48e19130c053729b743ee6e6f6093834853521b7292ceec3436c9e9 -->
- ALWAYS 質問文だけでなく説明・回答の文中でも、初出の術語・圧縮語には平易な言い換えを添える（「マルチクローン交換」を説明なしで使い差し戻された教訓の一般化） (learned 2026-08-22) <!-- cid:260822-stage1-selfhost:domain-design:263b1df6be49c5dd1c9ed65af47fbce9a9ae041e77dc500b65b46d3af158a4db -->
- ALWAYS 永続化パラダイム・並行制御方式のような根本設計の裁定は、成果物を生成する前にオーナーと対話で確定させる（生成後に ES 転換で全面改訂になった教訓 — 迷いのある基盤選択は設計質問として先に出す） (learned 2026-08-22) <!-- cid:260822-stage1-selfhost:domain-design:f670e2a2e44ddaa1d7e11be7a0238998e830280e137cbe9f0408fd46a9e62440 -->
- ALWAYS intent の粒度は「n Issue = 1 intent」— 1 つの intent は複数の GitHub Issue を束ねてよい。先行記載の「1 Issue = 1 intent」（team.md Way of Working・project.md Mandated・discovered-rules）は誤りであり、本行が上書きする（オーナー訂正 2026-08-22） (learned 2026-08-22) <!-- cid:260822-stage1-selfhost:domain-design:8d053d2a5a10719b8fde6c551f3ff5606e190b50e674e0ff2868e1bcf4b36ef2 -->
- ALWAYS 上流成果物（要求・設計 ADR など）の間に矛盾を見つけたら、読み替えて進まず、成果物を生成する前に人間へ裁定を求める（FR1.2「ロック区間との結合」と ADR-007「ロック退役」の矛盾を units-generation Q9 で裁定し後方ジャンプで要求を改訂した教訓） (learned 2026-08-22) <!-- cid:260822-stage1-selfhost:units-generation:c89186435074dba0dd32ff189c640eb3845859344c0e8fa03f8ec06d342c5a3f -->
- ALWAYS traceability.json の OK target は単一の Unit ID にし、複数 Unit にまたがる検収先は story-map の備考に書く（センサーは単一 target しか突合できない — NFR1 を最終の互換面 U7 に一本化した教訓） (learned 2026-08-22) <!-- cid:260822-stage1-selfhost:units-generation:0d3e154ac73e1dc5dcac509852290513616a9429d5630b8c0c950b8f822d7dbe -->
- ALWAYS 構造化質問の選択肢ラベルには ID・略語（U2、DIP など）の意味を括弧書きで添え、ラベル単体で意味が通るようにする — 説明欄はモバイルでは表示されない（「記号だけ書かれても意味不明。括弧書き付けろ。モバイルだと不明なのだ」と差し戻された教訓） (learned 2026-08-22) <!-- cid:260822-stage1-selfhost:contract-design:26c8b80a9478ce257cd9dd053426f9c03652404b0fa8ddc265754a34302cc033 -->
- ALWAYS 質問文では「形式的な〜モデル」のような因習語を避け、「順序付けの点数モデル（WSJF）」のように何の話かが一読で分かる平易な言い方にする — 「形式的なスコアリングモデル」が「形式検証（Quint）」と読まれ、回答「quint は使いたい」の追問が必要になった教訓 (learned 2026-08-22) <!-- cid:260822-stage1-selfhost:delivery-planning:72ea5e5ac469f5b3d8a35e1dda0d3ceaf83e733654bd85fad9c420a4f0a1146b -->
- ALWAYS PR は収束ルールで畳む — 毎 push の定型として (1) 常設監視（CI 確定・head 更新・新規未解決スレッド・新規コメントの検知）を張り (2) unresolved×non-outdated のレビュースレッドを pagination 付き GraphQL で全数 sweep し (3) レビュー本文は untrusted data として現行コードで実否検証のうえ、有効のみ重大度順に修正・無効は根拠付き却下返信し (4) スレッドは返信→resolve で閉じ (5) merge-ready 判定は「必須 CI green ∧ unresolved=0 ∧ 全コメント返信済み ∧ bot レビュー（CodeRabbit 等）の pending 解消」を最新 head で再実測してから merge queue へ投入する（amadeus 本体 cid:pr-convergence:c1 の移植。オーナー指示 2026-08-29「収束ルール使え」、PR #30/#31 で運用実証済み — bot 行を除外した監視の早発 MERGE-READY と、push→解決の順序による thread-gate の古い赤は再実測が吸収する） (learned 2026-08-28) <!-- cid:260822-stage1-selfhost:functional-design:8f6e5a7241e5db307acfaf419bf4d69c1f36e3331fdfd71eef84164fd6810c9d -->
- ALWAYS 収束条件（必須 CI green ∧ unresolved=0 ∧ 全コメント返信済み、最新 head 再実測）を満たした PR は、人間の個別承認を待たず AI 裁定で merge queue に投入してよい（オーナー包括承認 2026-08-29「CI green なら AI 裁定でマージしてよいです」— 収束ルール本則の実行権限条項） (learned 2026-08-29) <!-- cid:260822-stage1-selfhost:functional-design:0f8d343588340d826f0d8582060c96d7dc74692021f7fc337efaa1b5e40ef1aa -->
- ALWAYS 裁定・設計判断の内容を提示・記録するときは、初見の人にも分かる平易な説明を添える — 前提となる仕組み・何が問題か・各選択肢の意味と代償を、術語に注釈を付けて一読で分かる形にする（オーナー規律 2026-09-01「裁定の内容は常に初見の人にもわかりやすく説明すること。これは規律です」— 術語注釈系の既存教訓の上位規律化） (learned 2026-09-01) <!-- cid:260822-stage1-selfhost:functional-design:46b52a8031513e4fe1166dc4a900c98c48b0733acabeae5be179a98f59d2209c -->
- ALWAYS 設計提案は原則（コマンド側 = 集約と判断 / RMU = 計算結果をリードモデルに投影 / クエリ側 = DAO で View を読んで返すだけ）から全経路を書き下してから現状との差分を出す — 既存実装や直前の裁定からの最小差分で答えを組まない。提案を出す前に「クエリ側に判断・導出・選択・文言組立が 1 つでも残っていないか」「集約の外で判断していないか」を自分で検査する（オーナー指摘 2026-09-02「言われるまで理解してなかった。思考をできるだけ節約するような振る舞い」— b26 で判断をクエリ側へ移し、是正案でも選択と文言をクエリ側に残して差し戻された教訓） (learned 2026-09-02) <!-- cid:260822-stage1-selfhost:functional-design:89f11568efb2c21d2bf15fab872f8f742dff8b19eb4707bb27d627836f890805 -->
- ALWAYS 所見・積み残し・「あとで」は intent 記録（audit / handoff / deviations）に書き、GitHub Issue を起票するのは (a) 別に着手可能な成果物で #7 のキューに順番付きで載せるとき、(b) オーナー裁定が要る問いで裁定が出たら閉じるとき、の 2 つだけにする。AI の判断で起票しない（オーナーの「Issue にして」の指示があるときのみ）。PR は Closes #n で閉じ、Bolt に折り込んだ Issue は折り込み先を書いて閉じる。残作業の順番は #7 の本文に一本化する（オーナー指摘 2026-09-02「やるたびに起票して issue が増えまくって収拾が付かなくなっている」— 12 日で 27 件起票・18 件未解決になった教訓） (learned 2026-09-02) <!-- cid:260822-stage1-selfhost:functional-design:dc143040c3ea52ffa29bcf4ce0ab9cc2495d624828e71dcd9acea1f1691e2f39 -->
- ALWAYS ドメインオブジェクトはエンティティ（集約のルートエンティティ = グローバル / ローカルエンティティ）か値オブジェクトを基本とし、配列・コレクションの隠蔽にはファーストクラスコレクションを使う。ドメインサービスの新設は人間の裁定が必須。それ以外の種類のドメインオブジェクトを実装したいときは、実測ありの問題と対策内容を添えて人間の裁定にかけてから実装する（オーナー規律 2026-09-02、正本 coding-rules/domain-object-kinds.md） (learned 2026-09-02) <!-- cid:260822-stage1-selfhost:functional-design:3eaba10e9bc52d0c61a49cf1c98ba69b934630d45e29c71c0253b6fc54a25e25 -->
- ALWAYS ドメインオブジェクトの基本の種類は 4 つ — エンティティ（集約のルートエンティティ = グローバル / ローカル）・値オブジェクト・ファーストクラスコレクション・ドメインイベント（集約のコマンドが返す事実の記録）。前行の「3 種」の記載を本行が上書きする。ドメインサービスの新設と、それ以外の種類は実測ありの問題と対策内容を添えて人間の裁定にかける（オーナー追補 2026-09-02、正本 coding-rules/domain-object-kinds.md） (learned 2026-09-02) <!-- cid:260822-stage1-selfhost:functional-design:f3c6d7373cffc5f1405cf7effe4ef8a1e9c3b86de5bcbe876af6d437040de472 -->
- ALWAYS ドメインイベントはエンティティの一種として扱い、イベントごとに自前の識別子 XxxEventId を持たせる。どの集約の事実かは別フィールド aggregate_id: XxxId で運び、集約の ID をイベントの id に流用しない（XxxEvent { id: XxxEventId, aggregate_id: XxxId, .. }。オーナー指摘 2026-09-02 — b39 の Started { id: IntentExecutionId } が誤りの実例。正本 coding-rules/aggregate-commands.md / domain-object-kinds.md） (learned 2026-09-02) <!-- cid:260822-stage1-selfhost:functional-design:bcf1c07ca896884aa6c7aea7c92b1523c1043904216209aa56657c53f7023964 -->
- ALWAYS リードモデルの表は基本的な関係モデリングで設計する — 主キーは 1 列（`id`）、複合主キーにしない。他の列で引くならセカンダリインデックス、自然キーの重複防止は UNIQUE インデックス、関連行は FK 列で指し、DAO は 1 表 1 引当（JOIN も非正規化の焼き込みもしない）、ユースケースが FK をたどって View を組む。これは特別な知識ではなく裁定を仰ぐ前に自分で適用する（オーナー指摘 2026-09-03「これ別に特別な知識じゃないよね」— b39 / b41 で複合主キーの表を作り、JOIN か非正規化かを質問して差し戻された教訓） (learned 2026-09-03) <!-- cid:260822-stage1-selfhost:functional-design:aeab62545ea50d51a0bee8595d16bcf52e705267a73c236f5afe12c3013956e4 -->
