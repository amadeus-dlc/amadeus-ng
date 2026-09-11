# 明示された必須規則

**状態: 回答確認済み・工程承認待ち。** 以下は [依頼原文](../../project-description.json) と [確認事項](practices-discovery-questions.md) で人間が明示・確認した制約だけを整理したもの。新たな禁止事項や実装済み機能を推定して追加しない。

## Mandated

- ALWAYS 今回は独立した先行の最小通し実装を設けず、通常の作業単位で必須差分を依存順に実装する。（根拠: 確認事項Q1、内容確認）

- ALWAYS 現行 `main` のコードを実測し、実装に必要な契約差分だけを設計成果物に記録する。（根拠: 基準と正本、進め方の規律）
- ALWAYS 実装を TDD の red → green → refactor の順で進め、Quint・ITF・ゴールデンを外側の受入ゲートとする。（根拠: 進め方の規律）
- ALWAYS 既存の90%カバレッジ床・相対ゲート・CIを維持する。（根拠: 現状の品質ゲート、完了条件、承認済み計画）
- ALWAYS Boltを [Pull Request](https://github.com/amadeus-dlc/amadeus-ng/pulls) 1本に対応させ、直列1本・squash-mergeで進め、CI成功と収束ルールをマージ条件にする。（根拠: 進め方の規律）
- ALWAYS 上流仕様と現行コードの不一致を人間へ提示して裁定を求める。ゴールデンの版差は切替条件2を判定する前に裁定を受ける。（根拠: 基準と正本、進め方の規律）
- ALWAYS 本リポジトリで `target/release/aidlc` による開始・質問・ゲート承認・完了の実地スモーク、同バイナリの自己診断成功、CI全ジョブ成功を切替の完了条件にする。（根拠: 目的と到達条件）
- ALWAYS 切替後はホストを直近の安定タグ、ターゲットを開発版として2版運用する。（根拠: 目的と到達条件）
- ALWAYS 配布元のステージ・エージェント・プロトコル・コンパイル済みグラフを再利用する。（根拠: 基準と正本）
- ALWAYS 規則は `memory/`、参照資料は `knowledge/documents/`、横断知識は `knowledge/aidlc-shared/`、工程成果物は intent 記録に置き、参照資料は `knowledge onboard` で目録化する。（根拠: 基準と正本）
- ALWAYS 会話と成果物を日本語で記述し、術語に初出の注釈を添える。（根拠: 進め方の規律）

## Forbidden

- NEVER 既存コードを再設計・再文書化したり、削除済みの過去記録を現存する根拠として扱ったりする。（根拠: 基準と正本）
- NEVER 配布ステージ類を独自に書き直す。（根拠: 基準と正本）
- NEVER 上流仕様との不一致を独自に読み替えて実装を進める。（根拠: 進め方の規律）
- NEVER AIの判断だけで GitHub Issue を起票する。（根拠: 進め方の規律）
- NEVER リポジトリ直下に手書きの文書ツリーを新設する。（根拠: 基準と正本）
- NEVER 今回の実装範囲に自律実行、センサー・プラグイン・他ハーネス、配布一般化、OTel、インストーラ、仕様12・13号の全文執筆、スモークで踏まない既存課題を追加する。（根拠: スコープ外）

## Sources

- [desc] Initial description: [依頼原文](../../project-description.json)。
- [scope] Workflow-selected scope: `selfhost-stage1`。
- [Q1] [確認事項](practices-discovery-questions.md): 通常の作業単位で進める。全体の内容確認は `Looks correct`。
- 現物との照合と未決事項は [evidence.md](evidence.md) を参照。

## Assumptions & Open Questions

- 工程承認待ちであり、`memory/team.md`・`memory/project.md` への反映は行っていない。
- ゴールデンの採用版、必要呼出集合と担当範囲は要求分析で裁定する。通常の作業単位で進める方針は回答確認済み。
