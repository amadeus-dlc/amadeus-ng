# review-history-20260823 — U3 NFR Design 初版レビュー節の退避

> 2026-09-07 の unit-major 再走（Modify）で `security-design.md` を現行コードへ全面更新するにあたり、初版（2026-08-23、Bolt B5 着手前）の末尾に
> あった advisory レビュー節（READY、Minor 3）を**逐語**でここへ退避した。所見が指す本文（3 段の検査点・`within_write_transaction`・
> `entities.md` の Review・C3 の `usize` → `u64`）はいずれも失効または解消済みであり、本再走の承認根拠には使わない。要旨表は現行の
> `security-design.md` の「Review 履歴」節にある。以下は退避時点のバイト列そのもの（見出しレベルも変えていない）。

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-08-23T09:23:55Z
**Iteration:** 1（advisory, unit: u3-event-store-repository）

### Findings

| # | Severity | Location | Finding | Recommendation |
|---|---|---|---|---|
| 1 | Minor | security-design.md §2/§5、`security-requirements.md` NFR4.3 | NFR4.3 requirements 側の合格基準は「`unwrap` / `expect` / `panic!` / `indexing_slicing` を生まない」を clippy deny で機械強制する前提だが、実測（`Cargo.toml` `[workspace.lints.clippy]`）では `unwrap_used` / `expect_used` は deny 済みの一方、`indexing_slicing` と `panic` はいずれも未設定のまま（nfr-requirements レビュー iteration 1 の Major 所見2 として既出）。この所見は `pending-revision.md` に採録された 1 件（TOLERANCE 0.05→0.01、既に security-design §6-4 が反映済み）に含まれておらず、本 nfr-design の §5（サプライチェーンと境界）も `unsafe_code = forbid` と依存差分のみを扱い、この 2 lint の追加/見送りには触れていない。§2 の 3 段検査点はまさにこの種のパニック（範囲外索引・`seq_nr − 1` の減算）が起きやすい箇所であるため、機械強制の空白が実装まで持ち越される。 | 既に一度提示済みの所見であり再提起はしないが、承認ゲートで「意図的に見送る」か「B5 のスコープに `indexing_slicing = "deny"` / `panic = "deny"` の追加を含める」かを一言で確定させておくことを推奨する。 |
| 2 | Minor | `functional-design/entities.md` の `## Review`（iteration 1、08:40:40Z） | 上流 FD 成果物 `entities.md` に埋め込まれた `## Review` はいまも文字どおり `Verdict: NOT-READY`（Critical 1: genesis での `aggregate.version() − 1` の u64 アンダーフロー + 恒常的な偽陽性 Conflict、Major 2 件: C3 `usize`→`u64` の無言変更、`&self`/`&mut self` の内部可変性戦略欠落）を記録している。しかし実コードを突合すると、`rules.md` BR1.3 と `functional-spec.md` §3.1 は既に `expected = aggregate.version()`（減算なし）へ修正済みで、`entities.md` の `EventStore` / `WorkflowExecutionRepositoryImpl.store` の記述も「レビュー所見 2」「本レビュー所見 3」と名指しで参照しながら u64 具体化の理由と `RefCell` 内部可変性戦略を明記済みだった（3 件とも解消を確認）。つまり本 nfr-design（security-design.md §3、functional-spec.md §3.1）が依拠している版はすでに修正後の内容であり、バグそのものは nfr-design には伝播していない。ただし `entities.md` 末尾の `## Review` ブロックだけが iteration 2 として更新されておらず、監査証跡としては「NOT-READY のまま」に読める状態が残っている。 | nfr-design 自体の修正は不要（伝播なしを確認済み）。承認ゲートで `entities.md` の `## Review` を iteration 2 として更新するか、少なくとも「3 件とも本文で解消済み」という一文を追記し、監査証跡の齟齬を解消することを推奨する。 |

> （2026-08-23 追記: 本所見が言及する「`RefCell` 内部可変性戦略」は、その後のオーナー裁定により**前提ごと失効**した。`WorkflowExecutionRepository::store` を `&mut self` へ改訂し、`WorkflowExecutionRepositoryImpl` / `EventStoreImpl` / InMemory 両型から `RefCell` / `Rc<RefCell<_>>` / 手書き `Clone` を除去したため、内部可変性戦略そのものが存在しない — 正本 `coding-rules/interior-mutability.md` / `coding-rules/command-query-separation.md`、委任 8 で実装是正済み。所見本文は監査証跡としてそのまま残す。）
| 3 | Minor | security-design.md §5、logical-components.md §1、`contract-summary.md` C3（97-135 行） | C3（所有者 U5/U6、`contract-summary.md` §3）は `EventStore::persist_event` / `get_events_by_id_since_seq_nr` の数値パラメータをいまも `usize` で定義しており、直近の改訂（2026-08-23、C4/ADR-008）でも変更されていない。nfr-design（および依拠元の `entities.md` BR1.1）はこれを一貫して `u64` として扱い、「C3 改訂提案として所有者 U5/U6 へ申し送り — 無言の変更にしない」と明記しており、U3 側の対応としては適切（サイレントな乖離ではない）。しかし `contract-summary.md` 側にはこの申し送りに対応する注記や pending 項目が見当たらず、U5/U6 のどちらの Bolt がこの改訂を引き取るかが共有契約上は未着地のままである。U3（Bolt B5）と U5/U6 のいずれかの Bolt が異なる型で trait 実装を進めた場合、コンパイル時の型不一致リスクが残る。 | 承認ゲートで、この「申し送り」が U5 または U6 のどちらのバックログ / Bolt に着地しているかを一言で確認し、必要なら `contract-summary.md` 側にも pending 注記を残すことを推奨する（U3 側の設計修正は不要）。 |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| `bun .claude/tools/aidlc-sensor-traceability.ts --stage nfr-design --output-path nfr-design/traceability.json` | PASS（`{"pass":true,"gaps":[],"orphans":[],"missing_from_table":[],"missing_from_upstream_ids":[],"invalid_entries":[],"invalid_targets":[]}`） | NFR1.1〜NFR4.6 の 18 件すべてに過不足のない coverage |
| `bun .claude/tools/aidlc-sensor-required-sections.ts`（security-design.md） | PASS（h2_count=9） | §1〜§9 の H2 見出しを検出 |
| `bun .claude/tools/aidlc-sensor-required-sections.ts`（logical-components.md） | PASS（h2_count=5） | §1〜§5 の H2 見出しを検出 |
| 実コード突合（`modules/core/domain/src/orchestration/workflow_execution.rs` の `check_invariants`（873-925 行）/ `apply_event`（744-759 行）/ `from_snapshot`（959-1005 行）） | 一致 | §2 の「段 3」が主張する検査（長さ整合・cursor 範囲・parked_at 範囲・at_most_one_active・no_gate_bypass・SequenceGap・InvariantViolation）はすべて実装済みコードに実在し、記述と食い違いなし |
| 実ファイル突合（`Cargo.toml` `[workspace.lints]`、`scripts/coverage.sh` の `TOLERANCE=0.05`、`scripts/quint-gate.sh` の `audit_lock` ステップ、interface-adapter `Cargo.toml` の `md5 = "0.8"`） | 概ね整合 | `unsafe_code = "forbid"` は既存（新規適用不要、§5 の記述と整合）。`TOLERANCE=0.05` と `audit_lock` ステップ・`md5` 依存は現状値であり、§6 の退役手順（削除・0.05→0.01・audit_lock→journal_protocol 置換）が対象とする現状と一致。`indexing_slicing`/`panic` lint 不在は Finding #1 として記録 |
| rusqlite 実挙動の確認（設計知識ベース） | 整合 | `Transaction` の drop-rollback、`SQLITE_BUSY` の `Busy` エラー変換という§3の前提は rusqlite の実際の API 契約と一致 |

### Summary

security-design.md / logical-components.md / traceability.json は、上流（`security-requirements.md` NFR1.1〜NFR4.6、`functional-spec.md` / `rules.md` の BR1.x〜BR5.x、C3/C6）および実装済みドメインコード（`workflow_execution.rs` の `check_invariants` / `apply_event` / `from_snapshot`）の双方と高い精度で一致しており、traceability・required-sections の両センサーも PASS した。3 段の検査点（NFR3.2/4.3/4.4）・Tx と競合の設計（NFR3.3/3.5、BR1.3/BR2.3/BR2.4 と整合、rusqlite の drop-rollback/Busy 挙動も正確）・障害ドメインと再試行政策（C3 ③ と一致）・退役手順（BR3.1、順序も安全）・Quint DoD（§7）はいずれも実装可能な精度で書かれている。3 件の Minor 所見はいずれも本 nfr-design の設計そのものの欠陥ではなく、(1) 既出だが pending-revision に採録されなかった clippy lint（indexing_slicing/panic）の機械強制空白、(2) 上流 `entities.md` の `## Review` ブロックが実際には解消済みの Critical/Major を NOT-READY のまま表示し続けている監査証跡の齟齬（nfr-design 自体への伝播は無いことを実コード突合で確認済み）、(3) U3 が適切に申し送った C3 の `usize`→`u64` 改訂提案が `contract-summary.md` 側にまだ着地していないという、承認ゲートで一言確認すれば足りる残課題である。Critical 0 / Major 0 のため advisory の READY 閾値を十分に満たす。
