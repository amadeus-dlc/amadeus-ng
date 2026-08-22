# team-allocation — Bolt の担い手

> Delivery Planning（Inception 2.9）成果物。出典: `bolt-plan.md`（10 Bolt）、`../units-generation/unit-of-work.md`、
> `../units-generation/unit-of-work-dependency.md`、`../units-generation/unit-of-work-story-map.md`、
> `../contract-design/contract-summary.md`、`../domain-design/components.md`、`../requirements-analysis/requirements.md`、
> `../practices-discovery/team-practices.md`。team-formation（1.5）は classic scope で Skip のため、チーム参照はない。
>
> **Bolt** = 1 つの Unit を構築フェーズに 1 回通す作業の単位（1 Bolt = 1 PR）。**mob** = 1 つの Bolt を担う
> 作業グループ。本プロジェクトは単一 mob で、Program Board（複数 mob の割当板）は不要。

## 1. 割当

すべての Bolt を **AI 開発者エージェント（aidlc-developer-agent）** が実装し、**オーナー（人間）** が各 Bolt の
ゲートで PR をレビュー・承認する（毎 Bolt ゲート — Q8 = A）。

| Bolt | Unit | 実装 | レビュー（構築フェーズ内） | 承認 |
|---|---|---|---|---|
| B1 | U1 canon-json とゴールデン | aidlc-developer-agent | アーキテクチャレビュー（設計）、コードレビュー（実装） | オーナー |
| B2 | U2 ドメイン ES コア | aidlc-developer-agent | 同上（+ Quint ゲート） | オーナー |
| B3 | U9 正本・仕様の canon 追従 | aidlc-developer-agent（文書） | オーナーの diff レビュー（規則文面の確認 — FR9.6） | オーナー |
| B4 | U3 SQLite EventStore と Repository | aidlc-developer-agent | 同 B2 | オーナー |
| B5 | U4 ReadModelUpdater | aidlc-developer-agent | 同 B1（+ ゴールデン突合） | オーナー |
| B6 | U10 CI・ガバナンス | aidlc-developer-agent（CI 設定）+ オーナー（GitHub 設定権限） | DevSecOps 観点のレビュー | オーナー |
| B7 | U5 report ユースケース | aidlc-developer-agent | 同 B1 | オーナー |
| B8 | U6 next / continue ユースケース | aidlc-developer-agent | 同 B1 | オーナー |
| B9 | U7 CLI ディスパッチャ・フック | aidlc-developer-agent | 同 B1（+ Claude Code 上の実機確認はオーナー同席） | オーナー |
| B10 | U8 doctor とドッグフード | aidlc-developer-agent + オーナー（実地スモークの操作） | 実地スモークの記録レビュー | オーナー |

## 2. 役割分担の注記

- **オーナーが手を動かす箇所**: B6 の branch protection 設定（GitHub 管理権限）、B3 の規則文面確認（FR9.6）、
  B9 のフック実機確認、B10 の実地スモーク（ゲート承認の操作）。
- **フェーズ内のレビュー役**は AI-DLC のステージ定義に従う（functional-design などの advisory レビュー）。
  PR のマージ判断は常にオーナー。
- **並列なし**: mob が 1 つで PR は直列（team.md）。次の Bolt は前の Bolt のマージ後に着手する。
