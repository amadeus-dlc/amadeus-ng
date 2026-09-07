# pending-revision — U3 nfr-requirements（次の書込機会で適用する改訂案）

> **旧 3 項目（2026-08-23 レビュー所見由来）は 2026-09-07 の再走（Modify）で閉じた**: 1 = `scripts/coverage.sh` の `TOLERANCE` 0.05 → 0.01 は B5 で達成済み
> （実測 `:40`）、2 = `clippy::indexing_slicing` / `clippy::panic` の workspace lint 昇格は達成済み（実測 `Cargo.toml:43-44`）、3 = `cargo audit` の `audit` ジョブが
> advisory（`ci-success` の `needs` 外）である旨は NFR4.1 に注記した。
>
> 本ファイルの以下は 2026-09-07 再走レビュー（advisory、iteration 1、**READY**: Critical 0 / Major 2 / Minor 4、`security-requirements.md` 末尾 `## Review`）の
> 所見。advisory は終端受領のため成果物は凍結され、per-unit・gate false のため承認ゲートにも載らない。**次の U3 nfr-requirements 書込機会**（Request Changes
> または後続ステージからの折り戻し）で以下の確定文面を適用する。所見はすべてコンダクタが再実測して有効と確認した（`reopened()` :203-215、`# Panics` 3 か所
> :347 / :1506 / :2364、`dto/` 32 エントリ、`proptest` 3 クレート、requirements.md :133-135）。

## 追記 2026-09-07 — 再走レビューの所見（R-01〜R-06）

4. **R-01（Major）NFR4.7 の直列化根拠**: 「同一プロセス内は `&mut self` の排他で直列化」を次に差し替える —「同一プロセス内でも `reopened()`
   （`intent_execution_repository_impl.rs:203-215`）で同じストアを指す別ハンドルを持てる（本家ストアの `Clone` は基底状態を共有 — 実装固有テスト
   `the_volatile_store_is_shared_by_the_reopened_handle`）。直列化の実体はハンドル数に関わらず本家の楽観 version CAS（NFR3.3）であり、`&mut self` は
   1 ハンドル内の排他にすぎない。同一プロセス・複数ハンドルの競合は `Conflict` として観測される（契約テストの Conflict がこの経路）。別プロセスは
   `SQLITE_BUSY` → `Io { kind: WouldBlock }`」。§3 Tampering の「並行書込による上書き」欄にも「複数ハンドル / 複数プロセス」の 2 経路を書き分ける。
5. **R-02（Major）NFR4.3 の panic 例外の網羅**: 「例外は 2 つだけ」を「例外は domain の `# Panics` 公開 API **3 か所**（`IntentExecution::replay` :347 /
   `apply_event` :1506 / 誕生変換 `From<(Started, DateTime<Utc>)>` :2343-2370、`#[allow(clippy::expect_used, reason = …)]` 付き — モジュール doc
   `intent_execution.rs:40` が「この 3 か所だけ」と宣言）と、アダプタの `SnapshotStrategy::default` の `unwrap_or(NonZeroUsize::MIN)`（panic しない形）」に改める。
6. **R-03（Minor）上流 requirements.md NFR3 の失効記述**: `requirements.md:133-135` の合格基準「改訂版 `audit_lock.qnt` の ITF 準拠」と集約名 `WorkflowExecution` は
   ADR-007（ロック退役）/ B12（改名）で失効している（contract-summary §3 に `~~audit_lock.qnt~~ → journal_protocol.qnt` の記録あり）。U3 側は読み替えを
   明示する — NFR2.5 / NFR3.3 の出典欄に「requirements.md NFR3 の `audit_lock.qnt` は ADR-007 で `journal_protocol.qnt` に置換済み（contract-summary §3）」を注記。
   **requirements.md 本文の訂正は U3 の書込範囲外** — inception requirements-analysis への後方ジャンプか人間裁定で行う（Issue は起票しない）。
7. **R-04（Minor）§1 の「永続化 DTO 3 型」**: 「永続化 DTO 群（`dto/` に 1 型 1 ファイル、実測 32 エントリ = DTO 30 + `mod.rs` + `tests.rs`）: 集約
   `IntentExecutionDto`・鍵 `IntentExecutionAggregateKeyDto`・イベント族 `IntentExecutionEventDto` + 変種 DTO 16 本（`started_dto.rs` … `practices_affirmed_dto.rs`）・
   共通の `DtoDecodeError` / `dto_vocabulary`。intent / workflow-definition 系の DTO も同居するが U3 の要求対象は IntentExecution 系」に改める。
   tech-stack-decisions §1「永続化表現」行も同じ文面に同期。
8. **R-05（Minor）NFR4.4 の見出し精度**: 「改竄の検出（完全性）」→「不変条件・通番・manifest を破る改竄の検出（完全性）」。本文に「検出できるのはこれらを
   破る改竄に限る — 同じ形式で整合した書き換えは検出しない（暗号学的完全性は非要求）」を足す。
9. **R-06（Minor）`proptest` の所在**: NFR2.2 と tech-stack §1 テスト行の「`proptest` は `core-command-domain` の値オブジェクトのみ」を「`proptest` はアダプタに無い
   （dev-dependency は `core-infrastructure` / `core-command-domain` / `core-query-use-case` の 3 クレート — 実測 `grep -l proptest modules/*/*/Cargo.toml`）」に改める。
10. **軽微（レビュアー補足）**: NFR2.4 の `ci.yml:74-95` は `:74-97`（`tools/lint` の 3 ステップの終端）が正。
