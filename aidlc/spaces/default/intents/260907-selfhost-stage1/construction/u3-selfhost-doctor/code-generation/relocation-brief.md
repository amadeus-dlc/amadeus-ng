# U3 再配置の作業指示（判定をコマンド側集約へ移す）

2026-09-12。担当 1 名（直列）。承認済み `code-generation-plan.md` は凍結成果物なので、
この逸脱と手順はここに書く。計画本文は書き換えない。

## 背景と裁定

承認済み計画 §「責任と配置」2 は D1–D5 の判定を Query use case に置いた。これは
`coding-rules/cqrs-boundaries.md` 規則 6 の 2026-09-02 追記（クエリ側ユースケースは
`dao.find → View` だけで、判断・導出・選択・文言組立を持たない）に反する。計画側の誤りである。

利用者の裁定（[doctor-placement-questions.md](doctor-placement-questions.md) Q2、原文）:

> doctor は、コマンド側集約が処理してイベントを吐き出し、RMU がイベントからリードモデルを
> 作り、クエリ側がその結果を表示です。

Q3 = A: 記録が無い初回も同じ流れをメモリ上のストアで回し、表示だけ行う（C7 の DC1 =
ファイル・イベントを一切作らない、を維持する）。記録があるときは既存ストアへ追記し、
`HEALTH_CHECKED` も従来どおり投影する。

Q1 = A: D3.c / D3.d の対象集合はコンパイル済み `stage-graph.json` 全体（33 ステージ）。
現実装のままでよい。

## 着地済みのもの（触る前に読むこと）

- `modules/core/command/domain/src/workspace/` に集約 `WorkspaceDoctor`、イベント
  `WorkspaceDoctorEvent`、第一級コレクション `DoctorChecks`（`evaluate` が判定の正本）、
  観測の値オブジェクト群、`doctor_checks/`（環境・フック・heartbeat・配布資産・記録の評価器）。
- `modules/core/command/domain/tests/workspace_doctor_contract.rs` 8 件が green
  （`tdd-logs/08-domain-doctor-red.log` → `08-domain-doctor-green.log`）。
- Step 1〜6 の実装と受入は着地済み（`implementation-verification.md`）。`doctor_contract`
  10 件（debug / release）、corpus 19 観測のバイト一致、DC1 / DC2 / DC10。

つまり**判定の移設先はもう在る**。残っているのは配線の切替と、クエリ側に残った判定の撤去である。

## 目標の配線

```
合成ルート (modules/app/aidlc/src/runtime/doctor.rs)
  └ 観測: DoctorObservationDaoImpl (query interface-adapter) → DoctorObservationView
  └ 写像: View → core_command_domain::workspace::DoctorObservation
        ↓
コマンド側 use case (書込み) — WorkspaceDoctor::start / diagnose → WorkspaceDoctorEvent → store
        ↓
RMU (core-read-model-updater) — イベントから read_* 表へ非正規化投影（行・並び・passed/failed・終了コード）
        ↓
クエリ側 (DAO → use case) — 行を引いて View を返すだけ
        ↓
描画 (runtime/doctor.rs::render) — 現状のまま
```

規則の根拠は `coding-rules/cqrs-boundaries.md`:

- 両側を知ってよいのは **RMU と合成ルートだけ**。だから View → ドメイン観測の写像は
  `modules/app/aidlc` に置く。クエリ側のクレートはドメインに依存しない（`Cargo.toml` に
  相手が現れないことで機械強制される）。
- 観測（ファイル・環境・ストアの読取）は「読むだけ」なのでクエリ側に残す。規則 5 により
  コマンド側へは移さない。
- 判定・集計・文言は集約 1 箇所。RMU は集約の答えを写すだけ、クエリ側は書かれた答えを読むだけ。
- DAO は 1 表 1 引当（`cargo lint` の `dao-single-table`）。DTO（`*View`）は DAO と同じ
  `port/` に同居する。

## やること

1. **コマンド側ユースケース**を `modules/core/command/use-case/` に追加する。
   `find_by_id` → 無ければ `WorkspaceDoctor::start`、有れば `diagnose` → `store` の
   規則 5 正規形にする。`find_by_id` だけで終わる経路を作らない。
2. **リポジトリ実装**を `modules/core/command/interface-adapter/` に追加する。既存の
   `IntentExecutionRepositoryImpl` と同型（event-store-adapter-rs）。初回のメモリ上ストアも
   同じリポジトリ型で開けるようにする（Q3 = A）。
3. **RMU 投影**を追加する。`WorkspaceDoctorEvent` → `read_*` 表。行（順序・ラベル・fix・成否）に
   加えて `passed` / `failed` / 終了コードまで焼き込み、クエリ側が数えないようにする。
   **監査シャードへは書かない** — 監査語彙は upstream 互換の `HEALTH_CHECKED` だけであり、
   診断イベントを監査行に足さない。RMU のクレート doc（`read_*` 表の正本）も追従する。
4. **クエリ側を表示専用へ削る**。`DoctorReportUseCase` は `dao.find(対象) → DoctorReport` に
   なる。`orchestration/doctor_report_use_case/` 配下の評価器 7 本（`*_checks.rs`、
   `stage_frontmatter.rs`、`stage_schema.rs`）は撤去する。`DoctorCheck` / `DoctorCheckId` /
   `DoctorReport` は DTO なので `port/` へ寄せる（規則 6）。撤去後に
   `modules/core/query/use-case/Cargo.toml` の `regex` が未使用になったら外す。
   `no-backward-compatibility.md` に従い、移設元に薄い転送や dead code を残さない。
5. **合成ルート**を上の順に組み替える。`render` の出力・終了コード・`audit_exists` による
   記録の有無・`RecordHealthCheckUseCase` の呼出しは**変えない**。
6. **ドメインの中立性**を保つ。`core-command-domain` に serde / SQL / ファイル I/O を
   持ち込まない（`domain-persistence-neutrality.md`）。

## やらないこと

- 外部の振る舞いを変えること。行の綴り・並び・fix・書式・終了コード・副作用は現状のまま。
- 既存テストの期待値を緩めること。閾値・アサート・カバレッジ床を下げない。
- 配布資産（`.claude/` 配下）・`tests/golden/` の封印済み fixture の変更。
- U4 の担当（D2.e の接続定義、`statusLine` の扱い）へ踏み込むこと。
- 承認済み `code-generation-plan.md` の書き換え。

## TDD の進め方（Testing Contract 準拠）

各段で red（失敗の実行確認）→ green（最小実装）→ refactor。実行ログは
`tdd-logs/09-…` 以降へ連番で残す（既存 01〜08 の形式に合わせ、先頭にコマンドと UTC 時刻、
末尾に `# exit=<値>`）。

推奨の段取り（依存順）:

| 段 | 対象 | 単位限定コマンド |
| --- | --- | --- |
| 09 | リポジトリ（永続化と再構成、メモリ上ストア） | `cargo test -p core-command-interface-adapter --test <新規>` |
| 10 | コマンド側ユースケース | `cargo test -p core-command-use-case --test <新規>` |
| 11 | RMU 投影 | `cargo test -p core-read-model-updater --test <新規>` |
| 12 | クエリ側 DAO + 表示専用ユースケース | `cargo test -p core-query-interface-adapter --test doctor_observation_contract` / `cargo test -p core-query-use-case --test doctor_report_contract` |
| 13 | 合成ルートと契約（不変の確認） | `cargo test -p aidlc --test doctor_contract`（debug と `--release`） |

既存の `doctor_report_contract` は判定のテストなので、表示専用になった時点で**判定の期待値は
ドメイン側の `workspace_doctor_contract` が持ち、クエリ側は行の写しを確かめる**形へ組み替える。
テストを消して件数を減らす方向の整理はしない。

## 完了の申告に含めるもの

1. 段ごとの red / green のログ経路と件数。
2. 触ったファイルの一覧（`source-manifest.json` の更新に使う。全経路、作成・変更・削除の別）。
3. `cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -D warnings`、
   `cargo lint`、上表の単位限定コマンドの実測結果。
4. 外部の振る舞いが不変であることの根拠（`doctor_contract` 10 件が debug / release で成功、
   corpus のバイト一致が維持されていること）。
5. 判断に迷って独断で決めた点があれば、その箇所と理由（親が裁定へ回す）。

## Sources

- [doctor-placement-questions.md](doctor-placement-questions.md)（利用者裁定 Q1 / Q2 / Q3）
- [implementation-verification.md](implementation-verification.md) §5-12、§9
- [progress-status.md](progress-status.md)
- `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/cqrs-boundaries.md`、
  `use-case-rules.md`、`gateway-taxonomy.md`、`no-backward-compatibility.md`、
  `domain-persistence-neutrality.md`
- [contract-summary.md](../../../inception/contract-design/contract-summary.md) C5・C7
