# ハンドオフ — クエリ側 / RMU の CQRS 是正（b37 = 明文化、以降 3 段の是正 Bolt）（2026-09-02）

## 現在地

- b36（#88）マージ済み。直後のオーナー指摘で、クエリ側が判断・導出を持ち RMU が計算結果を
  投影していないことが判明（`construction/query-side-audit/audit-1.md`）。
- 裁定（オーナー 2026-09-02）: クエリ側ユースケースは **DAO で View を読んで返すだけ**。
  計算結果は **RMU が非正規化リードモデル（SQLite の `read_*` 表）として投影**する。判断は
  集約へ戻す。詳細と 5 つの裁定は `construction/query-side-audit/read-model-spec.md` §10。
- b37 はこの明文化（規則正本 cqrs-boundaries 規則 3 / 6、gateway-taxonomy §3、監査記録、仕様）
  だけを畳む docs PR。

## 次（是正 Bolt の 3 段 — read-model-spec.md §9）

1. **判断の集約復帰**: `IntentExecution::next_decision` / `jump_resolve` / `state_binding` 材料、
   `WorkflowDefinition::scope_cost`。Quint `engine_loop` 観測面 ITF を domain へ戻す。
2. **RMU の構造化投影**: 定義ストリーム購読、`replay` による集約導出、`read_*` 表と同一 Tx、
   steering 参照入力のダイジェスト比較リフレッシュ。契約テストで表 = 集約クエリを固定。
3. **クエリ側の縮小**: DAO を `read_*` 引当へ、ユースケースを `find` → View に、分類と文言を
   コントローラ / プレゼンタへ、判断型と 2 つのパーサ（Markdown 逆パース・配布 3 ファイル）を削除。
   golden（directive / token 逐語）で外部観測を固定。

着手順は (1)→(2)→(3)。#74（park 本体）ほかのキューとの優先順はオーナー判断。
**(1) は b38 で着地**（`construction/handoff-b38.md`）。残作業キューの正本は #7 の本文。

## 教訓（学習儀式で project.md へ記録する候補）

- 設計提案は原則から全経路を導出してから現状との差分を出す — 既存実装や直前の裁定からの
  最小差分で答えを組まない（クエリ側に判断を残した是正案を差し戻された教訓）。
