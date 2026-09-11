# Project-Level Rules

> Project-specific specialisation and corrections. Loaded after `org.md` and
> `team.md` as strict-additive guidance; contradictions with broader policy
> are rejected. Populated by practices-discovery and the self-learning loop.
>
> Use sparingly: most teams don't need a project layer. Reach for it
> only when this specific project needs stable, durable guidance beyond the
> team practice (for example, package-specific release checks or an additional
> regression suite for a legacy component).

## Way of Working

<!-- Project-specific specialisation. Example: -->
<!-- This monorepo requires package-scoped branch names and a package owner -->
<!-- review in addition to the team's normal merge policy. -->

## Walking Skeleton

<!-- Project-specific specialisation. Example: -->
<!-- The walking skeleton must exercise the legacy service adapter as well -->
<!-- as the new service boundary. -->

## Testing Posture

<!-- Project-specific specialisation. -->

## Deployment

<!-- Project-specific specialisation. -->

## Code Style

<!-- Project-specific specialisation. -->

## Tech Stack

<!-- Technology choices locked for this project. -->

## Decided

<!-- Decisions made in earlier stages that should not be re-asked. -->
<!-- Format: DECIDED: [decision] (Stage [slug], [date]) -->

## Scope Overrides

<!-- Custom scope rules for this project. -->

## Forbidden

<!-- Populated by practices-discovery affirmation gate. -->
<!-- Format: NEVER [behavior] (affirmed [date]) -->
<!-- Example: NEVER throw exceptions across service layer boundaries (affirmed 2026-05-17) -->

- NEVER 既存コードを再設計・再文書化したり、削除済みの過去記録を現存する根拠として扱ったりする。（根拠: 基準と正本） (affirmed 2026-09-07)
- NEVER 配布ステージ類を独自に書き直す。（根拠: 基準と正本） (affirmed 2026-09-07)
- NEVER 上流仕様との不一致を独自に読み替えて実装を進める。（根拠: 進め方の規律） (affirmed 2026-09-07)
- NEVER AIの判断だけで GitHub Issue を起票する。（根拠: 進め方の規律） (affirmed 2026-09-07)
- NEVER リポジトリ直下に手書きの文書ツリーを新設する。（根拠: 基準と正本） (affirmed 2026-09-07)
- NEVER 今回の実装範囲に自律実行、センサー・プラグイン・他ハーネス、配布一般化、OTel、インストーラ、仕様12・13号の全文執筆、スモークで踏まない既存課題を追加する。（根拠: スコープ外） (affirmed 2026-09-07)
## Mandated

<!-- Populated by practices-discovery affirmation gate. -->
<!-- Format: ALWAYS [behavior] (affirmed [date]) -->
<!-- Example: ALWAYS use Result<T,E> for fallible operations in service layer (affirmed 2026-05-17) -->

- ALWAYS 今回は独立した先行の最小通し実装を設けず、通常の作業単位で必須差分を依存順に実装する。（根拠: 確認事項Q1、内容確認） (affirmed 2026-09-07)
- ALWAYS 現行 `main` のコードを実測し、実装に必要な契約差分だけを設計成果物に記録する。（根拠: 基準と正本、進め方の規律） (affirmed 2026-09-07)
- ALWAYS 実装を TDD の red → green → refactor の順で進め、Quint・ITF・ゴールデンを外側の受入ゲートとする。（根拠: 進め方の規律） (affirmed 2026-09-07)
- ALWAYS 既存の90%カバレッジ床・相対ゲート・CIを維持する。（根拠: 現状の品質ゲート、完了条件、承認済み計画） (affirmed 2026-09-07)
- ALWAYS Boltを [Pull Request](https://github.com/amadeus-dlc/amadeus-ng/pulls) 1本に対応させ、直列1本・squash-mergeで進め、CI成功と収束ルールをマージ条件にする。（根拠: 進め方の規律） (affirmed 2026-09-07)
- ALWAYS 上流仕様と現行コードの不一致を人間へ提示して裁定を求める。ゴールデンの版差は切替条件2を判定する前に裁定を受ける。（根拠: 基準と正本、進め方の規律） (affirmed 2026-09-07)
- ALWAYS 本リポジトリで `target/release/aidlc` による開始・質問・ゲート承認・完了の実地スモーク、同バイナリの自己診断成功、CI全ジョブ成功を切替の完了条件にする。（根拠: 目的と到達条件） (affirmed 2026-09-07)
- ALWAYS 切替後はホストを直近の安定タグ、ターゲットを開発版として2版運用する。（根拠: 目的と到達条件） (affirmed 2026-09-07)
- ALWAYS 配布元のステージ・エージェント・プロトコル・コンパイル済みグラフを再利用する。（根拠: 基準と正本） (affirmed 2026-09-07)
- ALWAYS 規則は `memory/`、参照資料は `knowledge/documents/`、横断知識は `knowledge/aidlc-shared/`、工程成果物は intent 記録に置き、参照資料は `knowledge onboard` で目録化する。（根拠: 基準と正本） (affirmed 2026-09-07)
- ALWAYS 会話と成果物を日本語で記述し、術語に初出の注釈を添える。（根拠: 進め方の規律） (affirmed 2026-09-07)
## Corrections

<!-- Project-specific corrections from human feedback. -->
<!-- Format: NEVER/ALWAYS [behavior] (learned [date]) -->
