# risk-and-sequencing-rationale — 順序の理由とリスク

> Delivery Planning（Inception 2.9）成果物。出典: `bolt-plan.md`、`../units-generation/unit-of-work.md`、
> `../units-generation/unit-of-work-dependency.md`（DAG）、`../units-generation/unit-of-work-story-map.md`、
> `../contract-design/contract-summary.md`（未解決項目 7 件）、`../domain-design/components.md`、
> `../requirements-analysis/requirements.md`、`../practices-discovery/team-practices.md`、確認質問
> `delivery-planning-questions.md`（Q1〜Q8）。
>
> **Bolt** = 1 つの Unit を構築フェーズに 1 回通す作業の単位（1 Bolt = 1 PR）。

## 1. 使った考え方

- **依存を守る**: `unit-of-work-dependency.md` の DAG（2.7 が決めた形）の辺をすべて満たす。トポロジカル順
  （依存が先）からの逸脱はない。
- **リスク先行**（Boehm のスパイラルモデルの考え方 — 不確実性の大きいものを先にやって判断を早く校正する）:
  根（依存の無い Unit）の並べ方と、根の直後に何を置くかをオーナーの心配 4 点（Q6）で決めた。
- **点数モデル（WSJF）は使わない**（Q2 = A）。10 Unit・強い依存で自由度が小さく、文章で理由を残す方が
  読みやすい。
- **walking skeleton は作らない**（team.md）。全体疎通は B10 のドッグフードで実証する。

## 2. 順序の理由（根の並べ方）

依存の無い Unit は U1・U2・U9・U10 の 4 つ。これをどう並べるかが本ステージの判断:

| 順 | Unit | 理由 |
|---|---|---|
| B1 | U1 ゴールデン | 心配 B（upstream 互換）への最初の手。正解データを先に確保すれば、以降の全 Bolt が TDD のオラクルとして使える。依存ゼロ・M 規模 |
| B2 | U2 ドメイン ES コア | 心配 A（ES 化の規模）の最大要素を最も早く着手し、設計が 1 PR に収まるかを早く知る。U3 以降の全コードが依存する |
| B3 | U9 正本修正 | U3 が使う `store` 動詞の正本注記と旧称除去を U3 着手前に済ませる（レビュー基準の整合）。S 規模で短い |
| B6 | U10 CI・ガバナンス | toolchain 固定・forbid 昇格・カバレッジ除外（main.rs）・branch protection を、main.rs を触る B9 より前に入れる。B2〜B5 のコード Bolt を先に通すのは、ES 化の学びを優先するため（U10 はいつでも差し込める） |

残りは依存順で一意に近い（U3 → U4 → U5 → U6 → U7 → U8。U5 と U6 は入れ替え可能だが、書く側の定型（U5）を
先に確立して読む側（U6）へ展開する方が自然）。

## 3. オーナーの心配 4 点と手当て

| 心配 | 手当て（どの Bolt で・どう） |
|---|---|
| A. ES 化の規模（U2/U3 が 1 PR に収まるか） | B2・B4 を早く置き、各 Bolt の functional-design 後に PR 規模を見積もる。1 日超なら中断してオーナーと分割を相談（§4） |
| B. upstream 互換（ゴールデン一致） | B1 でゴールデンを先に確保。B5（投影）・B8（continue_token）・B9（CLI/フック）で突合 |
| C. フック 4 本の実機動作 | B9 で Claude Code 上の実機確認（オーナー同席）。contract-summary の未解決項目「フック stdin スキーマ」は B1 の採取で確定 |
| D. ドッグフードで初めて繋がるリスク | B9 でバイナリ全体を手動実行（`.claude/settings.json` の切替）し、B10 の実地スモーク前に疎通を一度確認する。skeleton を作らない代わりの手当て |

## 4. リスク登録

| # | リスク | 起きやすさ | 影響 | 手当て |
|---|---|---|---|---|
| R1 | U2 / U3 / U6 / U7（L 規模）が 1 PR = 1 日に収まらない | 中 | 中（PR 肥大・レビュー負荷） | functional-design 後に見積もり。超過見込みなら Bolt 着手前にオーナーと分割を相談（Unit の再分割は units-generation へ差し戻し、または同 Bolt 内で「先行 PR + 本 PR」の 2 PR を例外許可するかをオーナー裁定） |
| R2 | 投影の監査行・状態ファイルがゴールデンにバイト一致しない（フェーズ境界のトリオ行順など） | 中 | 高（NFR1） | B1 のゴールデンを網羅的に採取（approve / skip / jump / park の各経路）。contract-summary C5 の未解決（Started / GateApproved の行順）を B5 の functional-design で先に確定 |
| R3 | Quint モデル改訂（`audit_lock.qnt`）が遅れて B4 のゲートが赤のまま | 中 | 中 | B4 の Bolt 内でモデル改訂を実装より先に着手（Q2a = A の範囲内で順序を工夫） |
| R4 | フック契約の読み違い（stdin スキーマ・終了コード） | 低〜中 | 中 | B1 でフック入出力ゴールデンを採取、B9 で実機確認 |
| R5 | `unsafe_code` forbid 昇格・toolchain 固定で既存ビルドが赤 | 低 | 低 | B6 で先に入れる（B9 より前）。赤なら同 Bolt で修正 |
| R6 | ドッグフードで初めて見つかる統合不具合 | 中 | 中 | B9 の手動疎通で前倒し発見。B10 は修正 PR の余地を見込む |
| R7 | 正本（coding-rules）と実装の乖離が B3 以降に再発 | 低 | 低 | 各 Bolt のレビューで正本照合（project.md Mandated） |

## 5. 逸脱の記録

- トポロジカル順からの逸脱: なし。
- 2.7 の申し送り（U9 → U3 の「着手前に済んでいるのが望ましい」）: B3（U9）を B4（U3）の前に置いて満たした。
- U10 を根 4 つの最後（B6）に置いた: 早期の CI 硬化より ES 化の学びを優先。ただし B9（main.rs）より前。
