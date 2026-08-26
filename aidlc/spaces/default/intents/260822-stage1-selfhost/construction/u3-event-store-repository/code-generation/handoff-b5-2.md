# handoff-b5-2 — B5 park 時点の状態と再開手順（2026-08-24）

> 前回の handoff-b5.md（2026-08-23 park）以降の続き。オーナーが外出・モバイルのためレビュー不能につき park。

## いまの状態 — マージ待ちで止まっている

- **PR #29** https://github.com/amadeus-dlc/amadeus-ng/pull/29 — **MERGEABLE / CLEAN、CI 全ジョブ緑**
- 作業ツリーはクリーン（監査シャードの追記のみ）。未 push なし
- **マージはしていない。オーナーの可否判断待ち**

受入の実測（すべて緑）: `cargo test --workspace` **674**、fmt / clippy / `cargo lint` /
`tools/lint` 28 / quint-gate 全緑 / `cargo audit` 両 lock 0 件 / coverage 絶対 98.39%・
相対 head ≥ base（+1.01pt）/ 内部可変性 grep 0（doc コメント 3 行のみ）/ 旧名残存 0。

## 前回 handoff 以降にやったこと

1. 委任 7（カバレッジ回復、+41 テスト）→ 受入 → レビュー iteration 1 = READY
2. **オーナー裁定で内部可変性を全廃**（委任 8）— `store` を `&mut self` へ、
   `RefCell` / `Rc<RefCell<Connection>>` / 手書き `Clone` を除去
3. 設計文書 16 箇所を同期（共有契約 C3 の `store(&mut self)` / `usize`→`u64` を含む）
4. レビュー iteration 2 = NOT-READY（Major 3 / Minor 4）→ **7 件すべて是正**
   （委任 9 で契約テスト装置の意味論を統一、`reader()` の生存性を裁定）
5. **ファクトリ命名規約**を正本化し、リポジトリ全体の違反 9 件を是正
6. **命名監査**（`naming-audit-report.md`）→ 10 件是正、正当な例外 25 件をカタログ化
7. **ユビキタス言語**の裁定 — ドメインモデルの `set_*` を全廃、Published Language と
   Ubiquitous Language の区別を正本化
8. **ハーネスの欠陥を 1 件修正** — 回復レビュー予算の unit スコープ化（`aidlc-lib.ts`）
9. `cargo lint` の抑制に**理由の記述を必須化**（赤例テスト 3 本）

## 新設した正本（`aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/`）

`interior-mutability.md` / `command-query-separation.md` / `no-backward-compatibility.md` /
`factory-naming.md` / `ubiquitous-language.md` / `cqrs-boundaries.md` の **6 本**。
README の一覧も更新済み。ADR-009（CQRS の依存境界）も `decisions.md` に追加。

## 2026-08-24 追記 — park 後にオーナー指摘で追加した是正

park 後もオーナーからの設計指摘が続き、以下を追加で反映した（いずれも挙動不変・検査全緑）。

- **newtype アクセサを C-CONV へ** — `StageIndex::value()` → `to_usize()`、
  `GlobalSeqNr::value()` → `to_u64()`、`UnsafeLineChar::value()` → `to_char()`（呼出 56 箇所）。
  `as_str()` は既に 14 箇所で正しく、逸脱は `value()` の 3 箇所だけだった
- **良い例カタログ** `coding-rules/good-examples.md` を新設（実在ファイルを指す索引）
- **`field-visibility.md` の裁定** — `pub` も `pub(crate)` も禁止、**例外を認めない**。
  検出境界の拡張と既存違反の是正は同じ Bolt で同時に着地させる（順序の制約）

## 再開後にやること

1. **オーナーにマージの可否を確認**（squash-merge、コミット名 = Bolt slug `b5-u3-event-store-repository`）。
   マージで確定するもの・残るものは `code-summary.md` §7 の申し送り 15 件を参照
2. **U2 の後続 Bolt で `WorkflowExecutionState` の構築・可視性を再設計する**（オーナー裁定
   「次でよい」）。3 つが 1 つの作業として結合している:
   - `new(..)` が `Self` ではなく Builder を返す（`factory-naming.md` 違反）。Builder に
     フィールド名の setter 12 本（非テスト 44 箇所・鎖呼出 59 箇所）
   - フィールド 16 本が `pub(crate)`。アクセサ 17 本があるのに直接アクセス 52 箇所
   - `cargo lint` の `no-public-fields` を `pub(crate)` まで拡張（抑制コメント不可）
   完全コンストラクタ化は引数 16 個超を生むので、値オブジェクトへの束ね直しとセットで設計する。
3. マージ後 → **U4（`u4-read-model-updater`、Bolt B6）** へ。ただし着手前に次の 2 つを処理する:
   - `unit-of-work.md` の U4 責務から**ジャーナル読取とチェックポイント前進を外し**、合成ルート（U7）へ移す
   - U4 を `embedded` から**独立クレート**へ変更（ADR-009 / `cqrs-boundaries.md`）
   - RMU の形は `fn project(events: &[WorkflowExecutionEvent], read_model: &mut ReadModel)`。
     **RMU が要るのはドメインイベントだけ**（受信する側であり、取りに行く側ではない）

## 未解決のまま残るもの（マージしても消えない）

- **U3 の `unit complete` が未達**。AI-DLC のレビュー帳簿がデッドロックしており、
  オーナー裁定「AI-DLC の決まりがハードルになるなら AI-DLC を使わないモードに切り替えろ」に従って
  帳簿の充足を待たずに進めた。**記録は偽造していない** — 通っていないゲートは通っていないまま
- ハーネスのもう半分の欠陥（unit-major でゲートが遠すぎて Request Changes を記録できない）は未修正
- 機械レビューはゼロで着地（CodeRabbit は上限超過、Devin はクレジット切れ、Bugbot もスキップ）。
  AI-DLC のアーキテクチャレビューは 2 回走ったが、**その後のファクトリ改名・命名監査の是正は誰も
  レビューしていない**
- `.coderabbit.yaml` を追加済み（次の PR から効く。本 PR は生の変更ファイル数で上限判定されるため無効）

## 注意

- `.claude/` は upstream AI-DLC ハーネスの vendored コピー。今回 2 ファイルを修正しており
  **upstream からの差分になる**。外部への報告は一切していない（オーナー明言「本家に勝手に報告するな」）
- `codekb/docs/` の 2 文書は RE 生成物なので手を触れていない（旧名が残っているのは意図的）
