# 開発規則の確認事項

## 確認済みの前提

依頼原文の明示事項は再質問しない。以下を規則草案の前提とし、未決事項への回答後に全体をまとめて確認する。

| 領域 | 明示済みの方針 | 根拠 |
| --- | --- | --- |
| Way of Working | 現行 main を基準に、Bolt（実装から統合までの単位）ごとに [Pull Request](https://github.com/amadeus-dlc/amadeus-ng/pulls) を1本、直列・squash-mergeで進める。CI全ジョブ成功とレビュー収束を統合条件にする。 | 依頼原文「基準と正本」「進め方の規律」 |
| Testing Posture | TDDのred → green → refactor。カバレッジ床90%と相対ゲートを維持し、Quint・ITF・ゴールデンを外側の受入検査にする。 | 依頼原文「現状」「進め方の規律」 |
| Deployment | 本リポジトリでreleaseバイナリを使う実地スモーク・自己診断・CI全ジョブ成功を確認する。切替後は安定タグのホストと開発版の2版運用。 | 依頼原文「目的と到達条件」 |
| Code Style | coding-rulesの正本と衝突優先順を守る。既存コードの再文書化を避け、配布資産を再利用する。会話・説明は日本語、観測契約の固定文字列は逐語で保持する。 | 依頼原文「基準と正本」「進め方の規律」、coding-rules/README.md |

## Q1: 最初に最小の通し実装を独立して作るか

walking skeleton（各部分を接続し、最小構成で端から端まで動かす先行実装）を最初の独立した作業単位にしますか。

初回の実装順と承認の扱いを決めるための質問である。専用スコープには指定がなく、原文にも先行実装の独立性は明記されていない。どちらを選んでも、各変更をTDDで検証し、完了時に本リポジトリで実地スモークを通す。自律実行へ切り替える承認にはしない。

- A. 通常の作業単位で進める（推奨） — 要求分析で確定する必須差分を依存順に実装し、独立した先行実装の工程は設けない。
- B. 最小の通し実装を先行させる — 最初の作業単位で必要部分を接続し、通し動作を確認してから残りを実装する。
- X. Other (please specify)

[Answer]: A. 通常の作業単位で進める

**User Input**: 推奨
**Mode**: guided

## 後続で裁定する事項

- ゴールデンの採用版、必要な呼出し・フック・受領の集合、バイナリと配布補助処理の担当範囲は要求分析で確定する。
- 品質担当が指摘した既存ゴールデン検査の比較粒度と、runtime呼出しの統合検査が実地スモークを代替しない点は、最終統合で根拠文書に補う。
- 受領動詞の設計に、ユースケースのCommand戻り値規則と現コードの差が影響する場合は、契約差分と正本を添えて人間へ裁定を求める。

## Sources

- [desc] Initial description: [依頼原文](../../project-description.json)。
- [scope] Workflow-selected scope: selfhost-stage1。
- [規則草案](team-practices.md)、[根拠](evidence.md)。
- [品質担当の確認](contributions/aidlc-quality-agent.md)、[開発担当の確認](contributions/aidlc-developer-agent.md)、[セキュリティ担当の確認](contributions/aidlc-devsecops-agent.md)。

## Consolidated Summary Confirmation

- 現行mainを基準に、Boltごとに [Pull Request](https://github.com/amadeus-dlc/amadeus-ng/pulls) を1本、直列・squash-mergeで進める。未解決の競合・対応すべきレビュー指摘を解消し、CI全ジョブ成功を確認する。
- 独立した最小の通し実装を先行させず、通常の作業単位で必須差分を依存順に実装する。自律実行は行わない。
- TDDのred → green → refactorを守る。既存のカバレッジ床90%・相対ゲート・Quint・ITF・ゴールデン検査を維持する。集約チェックだけでなく依存監査を含む全CIジョブを確認する。
- 本リポジトリでreleaseバイナリを使う実地スモークと自己診断を通し、切替後は安定タグのホストと開発版の2版運用とする。既存のruntime統合検査を実地スモークの代わりにはしない。
- coding-rulesの正本と衝突優先順を守り、既存コードの再文書化を避ける。配布資産を再利用し、独自変更は同期パッチへ記録する。会話・説明は日本語、観測契約の固定文字列は逐語で保持する。
- ゴールデン採用版、必要な呼出し・フック・受領の集合と実装担当範囲は、要求分析で別途裁定する。既存検査の比較粒度・未検証ケースを根拠文書に補う。

この内容で開発規則の成果物を確定する前に、認識に相違がないか確認してください。

- Looks correct
- Request changes

[Answer]: Looks correct
