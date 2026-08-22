# delivery-planning-questions — Bolt 順序の確認質問

> Delivery Planning（Inception 2.9）の質問票。出典: `../units-generation/unit-of-work.md`（10 Unit）、
> `../units-generation/unit-of-work-dependency.md`（依存 DAG: 根 = U1/U2/U9/U10、U5 と U6 は互いに独立）、
> `../units-generation/unit-of-work-story-map.md`（FR → Unit）、`../contract-design/contract-summary.md`（契約と
> 未解決項目）、`../domain-design/components.md`、`../requirements-analysis/requirements.md`（DoD = 実地スモーク +
> doctor green + CI green）、`../practices-discovery/team-practices.md`（Bolt = PR・直列・squash-merge・
> skeleton: off・TDD）。
>
> **Bolt** = 1 つの Unit（またはまとまった数 Unit）を構築フェーズ（設計 → 実装 → テスト）に 1 回通す作業の
> 単位で、本プロジェクトでは 1 Bolt = 1 PR。本ステージは Unit の依存 DAG（2.7 が決めた形）の上で
> **どの順に Bolt を出すか**を決める。依存は守るが、順序の選択は価値・リスクの判断。
>
> user-stories / team-formation は Skip のため、ペルソナやチームの参照は無い（全 Bolt の担い手は AI 開発者
> エージェント + オーナーのレビュー）。

---

## Q1. 何から作るか（着手方針）

- A. **土台先行 + リスク早出し**（推奨）— 依存の根（canon-json・ドメイン ES コア）から始め、最大のリスク
  （SQLite ストア + 投影の upstream 互換）を早い Bolt で潰す。価値の見える CLI は土台の後
- B. 価値先行 — 動く CLI が早く見える順（ただし依存上、U7 は U1/U4/U5/U6 の後にしか出せない）
- C. 形式スコアリング（WSJF: 価値 + 緊急度 + リスク低減 ÷ サイズ）で機械的に並べる
- X. Other (please specify)

[Answer]: A

## Q2. 形式的なスコアリングモデルを使うか

- A. 使わない（推奨）— 10 Unit・依存 DAG が強く、トポロジカル順の自由度は小さい。リスク判断を文章で残す
- B. WSJF 風に点数化して並べる（重み: リスク低減 > 価値 > サイズ）
- X. Other (please specify)

[Answer]: A（初回回答「quintは使いたい」は Q2a で確定）

## Q2a. Q2 追問 — 「quint は使いたい」の意味の確認

Q2 への回答「quintは使いたい」（Other）を受けた追問。Q2 が訊いたのは Bolt の**順序付けの点数モデル**（WSJF）で、
Quint（状態機械の形式検証ツール）はそれとは別物。Quint は team.md の Testing Posture で「毎 PR の受入ゲート」
として維持が確定しており、Bolt 計画でもそのまま残る。ここでは Bolt 計画の中で Quint をどう位置づけるかを確認する。

- A. 従来どおり — 毎 PR の Quint ゲート（`scripts/quint-gate.sh`）を維持し、状態機械の意味論を変える Bolt
  （U2 ドメイン ES コア、U3 ストア + ロック退役）では **Quint モデルの改訂を同じ Bolt に含める**（推奨）
- B. モデル先行 — U2/U3 では Quint モデル（`engine_loop.qnt` / `audit_lock.qnt` 改訂版）の改訂を**実装より先に**
  完了させる（Bolt 内の最初の工程、または独立の先行 Bolt）
- C. 全 Unit で Quint モデルを新設・拡張する（投影・CLI・フックにもモデルを書く）
- X. Other (please specify)

[Answer]: A

## Q3. Bolt の大きさ

- A. **1 Bolt = 1 Unit**（推奨）— Bolt = PR の直列運用と一致。10 Bolt
- B. 関連 Unit を束ねて Bolt 数を減らす（例: U5 + U6、U9 + U10 → 8 Bolt）
- C. Unit を横断する薄いスライス（Unit 境界を跨ぐ Bolt）
- X. Other (please specify)

[Answer]: A

## Q4. Bolt の並列実行

- A. **直列のみ**（推奨）— team.md「オープンな PR は常に 1 本」。依存の無い Unit（U1/U2/U9/U10）の並列性は
  「どれを先に出すか」の選択肢としてだけ使う
- B. 依存の無い Unit は並列 Bolt を許す（PR 直列運用の例外）
- X. Other (please specify)

[Answer]: A

## Q5. 外部依存（チームの外に待たされるもの）

- A. **実質なし**（推奨）— upstream ピンは固定済み（3c3146cf）、0b 採取は bun で再現可能、必要な外部権限は
  オーナーの GitHub 設定権限（branch protection）とオーナーのレビュー/承認のみ
- B. あり（X で具体的に: 何を・誰が・どの Bolt を止めるか）
- X. Other (please specify)

[Answer]: A

## Q6. いちばん心配な点（早めに手を打つため） (select all that apply)

- A. ES 化の規模 — U2（ドメイン ES コア）と U3（ストア + ロック退役 + Quint 改訂）が 1 PR に収まるか
- B. upstream 互換 — 投影の監査行・状態ファイル・逐語文言がゴールデンにバイト一致するか
- C. フック 4 本の実機動作 — Claude Code 側の契約（stdin/終了コード）の読み違い
- D. 最後のドッグフード（実地スモーク）で初めて全体が繋がるリスク
- X. Other (please specify)

[Answer]: A, B, C, D

## Q7. 構築フェーズの回し方（設計と実装の順）

- A. **Unit ごとに設計 → 実装を完結**（unit-major）（推奨）— 1 Unit の設計・実装・PR を終えてから次へ。
  Bolt = PR 直列と一致し、動くコードが早く出る。自律スウォーム（並列自動ビルド）は使わない
- B. 全 Unit の設計を先に済ませてから実装（stage-major）— 設計段階で全体整合を見たい場合
- X. Other (please specify)

[Answer]: A

## Q8. 構築フェーズの承認の仕方

- A. **毎 Bolt でゲート**（gated）（推奨）— 各 PR をオーナーが確認してから次へ（要求 A2 の前提と一致）
- B. 自律（autonomous）— 最初の Bolt 以降は承認なしで連続実行
- X. Other (please specify)

[Answer]: A

## Consolidated Summary Confirmation

- 着手方針は**土台先行 + リスク早出し**（Q1 = A）: 依存の根（canon-json・ドメイン ES コア）から始め、SQLite ストア + 投影の upstream 互換という最大リスクを早い Bolt で潰す。CLI は土台の後
- 点数モデル（WSJF）は使わない（Q2 = A）。リスク判断は文章で残す
- Quint は毎 PR の受入ゲートとして維持し、状態機械の意味論を変える U2（ドメイン ES コア）・U3（ストア + ロック退役）では Quint モデルの改訂を同じ Bolt に含める（Q2a = A）
- 1 Bolt = 1 Unit（Q3 = A）→ 10 Bolt。Bolt = PR で直列のみ（Q4 = A）。依存の無い Unit の並列性は着手順の選択肢としてだけ使う
- 外部依存は実質なし（Q5 = A）: upstream ピン固定済み、0b は bun で再現可能、必要な外部権限はオーナーの GitHub 設定権限とレビュー/承認のみ
- 心配な点は 4 つすべて（Q6 = A, B, C, D）: ES 化の規模（U2/U3 が 1 PR に収まるか）、upstream 互換（ゴールデン一致）、フック 4 本の実機動作、最後のドッグフードで初めて全体が繋がるリスク — 順序付けとリスク対策に反映する
- 構築の回し方は **Unit ごとに設計 → 実装を完結**（unit-major、Q7 = A）。自律スウォームは使わない
- 承認は**毎 Bolt でゲート**（gated、Q8 = A）

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
