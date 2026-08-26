# nfr-requirements-questions — U3 SQLite EventStore と WorkflowExecutionRepository（`u3-event-store-repository`）

> NFR Requirements（Construction 3.2）の質問票（Unit: U3、kind: library、Bolt: B5、規模 L）。出典: `../functional-design/{functional-spec,rules,entities}.md`（BR1.1〜BR5.2、
> レビュー所見 1〜3 反映済み）、`../../../inception/requirements-analysis/requirements.md`（NFR1〜NFR5、FR1.2 / FR1.3）、`../../../inception/contract-design/
> contract-summary.md`（C3 / C6）、`aidlc/spaces/default/codekb/docs/technology-stack.md`（既存依存: serde / serde_json / md5 / nix / libc、tokio 未導入、
> `#![forbid(unsafe_code)]`）、`aidlc/spaces/default/memory/team.md`（Testing Posture、CI ゲート、サプライチェーン裁定）、`functional-design-questions.md`（Q1〜Q4 = A）。
>
> **質問なし。** 基盤選択（ストア配置 / 直列化 / ドライバ / Quint）は機能設計 Q1〜Q4 で裁定済み。次の前提を確認して成果物へ進む。

## 前提（確認事項）

- P1. **依存追加は 2 つ**: `rusqlite`（`bundled` — SQLite を同梱ビルド。`libsqlite3-sys` が C をコンパイルするため cc が要る。CI の ubuntu ランナーは既定で可）と
  `tokio`（`rt` + `macros`、current_thread）。ワークスペース依存に固定版で追加し、`cargo audit` の対象に入れる（NFR4、team.md の裁定）。`md5` は退役と同時に除去。
  `tools/lint` の独立 `Cargo.lock` は変更なし。
- P2. **安全側の SQLite 設定**: `synchronous` は既定（FULL — 電源断でもコミット済み Tx を失わない）、`journal_mode` は既定（DELETE。WAL は付随ファイルを増やす
  ため使わない）、`busy_timeout` 5000ms、`foreign_keys` 不要（単一 DB・外部キー無し）。ファイル権限はプロセス umask 既定（upstream の他のワークスペース
  ファイルと同じ扱い。秘密情報を含まない）。
- P3. **脅威モデルはローカル単一ユーザ**: ストアは `.gitignore` 配下（C6 制約 4）で、改竄・欠損は `Corrupt`（復号不能・不変条件違反・スナップショット欠落）として
  検出して拒否する（panic しない）。暗号学的完全性（署名 / HMAC）は要求しない（U2 NFR と同じ立場）。
- P4. **品質ゲート**: TDD（契約テストを両実装で先行）、PBT（ワイヤのラウンドトリップ、`PROPTEST_RNG_SEED` 固定）、ITF 準拠（`journal_protocol.qnt`、全アクション網羅、
  fixture ≥ 6）、クラッシュ再構成テスト、カバレッジ 90% 床維持（adapter クレートに除外を足さない）、clippy 全ルール deny 維持、`cargo lint` 自己テスト
  （`reap-decision-locality` 削除後）。
- P5. **性能は非目標**（NFR5）: 数値目標なし。設計上の上限は「コマンド 1 回 = SQLite Tx 1 回 + 投影差分読取」で、スナップショット毎 store のため replay は通常 0 件。
  `bundled` のビルド時間増は許容（CI のキャッシュで吸収）。
- P6. **ログ・秘密情報**: ストア層はログ出力を持たない（`tracing` は ADR 0004 で application / adapter に後続導入 — 本 Unit では計装しない）。ペイロードの
  人間入力は逐語のまま保存（upstream 同等）。環境変数・資格情報を読まない（パスは composition root から注入）。

## Consolidated Summary Confirmation

- NFR1（upstream 互換）= ストア追加・ロック dir 非生成を逸脱台帳 # 4 で確定（パス）、既存の互換ファイルには触れない。NFR2（品質ゲート）= P4。
  NFR3（監査完全性）= 再構成の決定性・健全性検査・ITF・クラッシュ再構成。NFR4（セキュリティ / サプライチェーン）= 依存 2 追加 + md5 除去、audit、forbid unsafe、
  panic しない、改竄は Corrupt 検出。NFR5 = 非目標の明示。
- 成果物は security-requirements.md / tech-stack-decisions.md / traceability.json。

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
