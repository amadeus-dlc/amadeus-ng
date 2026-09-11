# U1: 本家2.7.1の受入採取基盤

## 実装結果

本家2.7.1の固定277ファイルを検証してから、隔離したワークスペースでCLI・フック・ハッシュ・受領・診断を実行し、生観測を保存する採取基盤を実装した。独立した2回の採取が、明示した比較規則の範囲で一致した。保存先は `tests/golden/upstream-a277af21/`。旧2.6.40ゴールデンとRust本体は変更していない。

正常開始のstage1ケースは、実在する `src/base.ts` を初期入力に持ち、本家が `Brownfield`、`bugfix scope, 9 stages`、`reverse-engineering`開始と判定した。以降の個別受領の検査は、初期状態を明示した合成ケースである。全9段を実際の人間と完走した記録ではない。

## 変更ファイル

- 採取元: `upstream-source.ts`、`upstream-manifest.sha256`。本家の完全なコミット、277ファイルのハッシュ一覧、固定元検証を共有する。
- 採取入口: `recapture-cli.sh`、`recapture-hash-canonical.sh`、`capture-cli.ts`、`capture-hash-canonical.ts`、`capture-supplemental.ts`。固定元以外を実行前に拒否し、既存の採取先を保護する。
- 生観測: `capture-observation.ts`。入力、隔離環境、固定配布との差分、全標準入出力、終了値・signal・起動失敗を保存する。
- 必須契約: `capture-stage1.ts`、`capture-doctor.ts`、`capture-learnings.ts`、`capture-extensions.ts`。保護有効の許可・拒否、追加フック、読取り候補と更新操作を採る。doctor/learningsは親が並行実装した。
- 比較: `normalization.json`、`corpus-normalization.ts`、`prepare-corpus.ts`、`verify-corpus.ts`。保存物のハッシュとファイル集合、役割を保つ対称正規化、全バイト差を検査する。
- 検査: `capture-source.test.ts`、`capture-corpus.test.ts`、`compare-corpus.test.ts`、`capture-doctor.test.ts`、`capture-learnings.test.ts`。
- `.github/workflows/ci.yml`: 既存の配布検査ジョブに上記テストと保存コーパス検証を追加。既存ジョブ・品質閾値は変更していない。

全ソースの一覧は[source-manifest.json](source-manifest.json)、要求との対応は[traceability.json](traceability.json)に記録した。番号回答のローカル配布修正は親が所有する前提是正であり、本家の採取元へ混ぜていない。

## TDDと検証

承認済みの3境界で失敗を実行確認してから実装した。既存の検証能力を確認する回帰テストは、最初から成功したものと区別する。実際の失敗出力を[test-evidence/](test-evidence/)へ保存した。

| Redで検出したこと | Greenへの変更 |
| --- | --- |
| 2.7.1の277ファイルが旧262ファイル条件に拒否される | 固定元の実測マニフェストへ移行 |
| 異なるコミットを早期に拒否しない、未検証hashモジュールが実行される | 採取入口とhash入口で実行前検証 |
| 補完採取に生の逐次観測がない | 生観測保存を共通化 |
| `.claude/settings.json`の変更前後が採取されない | 固定配布との差分として追加・変更・削除を保存 |
| 起動不能時にstatusがundefinedになりJSONから消える | 非実行をnull＋errorで明示。signalと非ゼロも保持 |
| 固定文言の改変、余分なファイル、独立採取の未知差を見逃す | ハッシュ/集合/全バイト比較 |
| 実測パス・時刻の正規化がない | 対称なliteral対応表 |
| 別IDを同じ値へ潰せる | 対応の一意性を検証。後続入力のID取り違えも検出 |
| 不正UTF-8の異なるバイトを同じ置換文字として比較する | 損失のないバイト表現で比較 |
| 同じ秒/秒跨ぎによるLast Updatedだけの差 | 正規化後にbefore/afterが同一の差分だけ除去し、状態値の差は拒否 |
| 計画承認・内容確認・レビュー・ゲート・追加フック等の必要観測がない | 1振る舞いずつ保護有効ケースを追加 |

U1の5ファイルをまとめた検査は **42テスト成功・失敗0、201 assertions**。その後、Brownfield/9段/RE開始と表示6操作の無変更を追加確認し、対象のcapture-corpusは **8テスト成功・失敗0、59 assertions**。[doctor/learningsの証跡](doctor-learnings-evidence.md)にはそれぞれのRed/Greenと限界を記録している。

保存コーパス検証、独立2回の比較、`git diff --check`は成功。親が実装前の `cargo test --workspace` 2,354件と、既存Bun検証74件の成功を確認済み。U1はRustを変更しておらず、Rustカバレッジや全CIジョブをこのBun結果で代用しない。

## 採取範囲と引継ぎ

[採取証跡](capture-evidence.md)に固定元、コマンド、142プロセス観測とhash32ケース、正規化の根拠を保存した。[追加ケースと引用](stage1-case-inventory.md)をC2/C3/C6/C7へ対応付けた。

旧版320ファイルとの照合は、[全数台帳](legacy-differences.json)と[全バイト差分](legacy-differences.patch)へ保存した。220ファイルが完全バイト一致。配布JSON、来歴、入力、状態、監査、標準出力を分類した。[Rust受入の移行一覧](legacy-consumer-inventory.md)に従い、U2が旧参照と比較項目を2.7.1へ揃える。

## 計画との差分・未完了

比較境界のテストは、不正UTF-8、IDの後続入力、正規化後の空差分の実測問題を受けて10件へ増やした。境界・品質閾値は変更していない。doctor/learningsの個別採取とテストは、親との所有分担で追加ファイルに分離した。計画本文とTesting Contract、承認済みテスト手順は変更していない。

U1の実装・採取・CI接続は用意した。Step 7の独立レビューは親がこれから行うため、計画のStep 7は未チェックで残した。Rustの2.7.1適合とCQS是正はU2、Native診断はU3、実際のClaude・人間・nativeバイナリによるスモークと安定タグ切替はU4の未完了作業である。

## 第1回レビュー指摘の是正

R-01〜R-03を、現在の計画への人間の再承認と正規begin成功後に是正した。計画末尾の第1回Reviewは変更していない。第2回の独立レビューは未実施である。

- **R-01**: initial_files・changed_files・fixture_changesごとに、FF/FEの異なるバイトを両辺sealしても区別する回帰を追加し、3件のRedを確認した。正常なUTF-8だけを文字列化し、不正UTF-8は数値バイト配列として比較する。JSONコンテナ自体もUTF-8を厳密に検証して、不正ならseal済みでも拒否する。比較器の14テストが成功。
- **R-02**: CIのdepth1 submoduleには本家の祖先が含まれないことを、fork tipだけを取得した隔離repoの `git archive` 終了128で再現した。検査前に本家の固定SHAを明示fetchするCI手順を追加。同じCI手順を隔離repoで実行してarchive成功を確認し、採取元9テストが成功した。実際の本家HTTPSへの固定SHA fetchも終了0。
- **R-03**: 検査対象JSONではなく、Unit DAG・単位定義・承認済み割当表の主担当/Directory/支援列から対象FRを導く。説明欄のUnit名を割当に使わない。未割当FRの追加と担当FRの削除、Unit宣言の欠落を拒否し、zero-Unitと既存US→ACを維持する。`traceability-unit-requirements.patch` を正規同期で3配布先へ適用した。回帰30テストが成功し、U1のtraceability.jsonと要求・割当表は変更せず実検査が `pass: true`、指摘0件となった。

U1検査と追跡検査の6ファイルは74テスト成功・失敗0（352 assertions）。追加したUnit宣言欠落の境界を含む追跡検査は30テスト成功・失敗0（129 assertions）。配布・承認を含む既存6ファイルも92テスト成功・失敗0（636 assertions）。保存コーパスの検証、新しい独立2回の採取相互比較、保存済みコーパスと新規採取の比較はいずれも成功した。保存された本家の期待バイトは変更していない。

詳細・コマンド・Red/Greenは[レビュー是正証跡](review-repair-evidence.md)へ記録した。新しい同期パッチ、3配布先、回帰テストをsource-manifestへ追加し、既存の親所有パスを保持した。

## Sources

- [承認済み計画](code-generation-plan.md)と[テスト手順](unit-test-instructions.md)。
- [要求書](../../../inception/requirements-analysis/requirements.md)、[契約C1–C8](../../../inception/contract-design/contract-summary.md)。
- 本家固定コミット `a277af218f0df7f325d3b8be7b6d90fce2c5bd40` の配布実バイトと実行結果。

## Assumptions & Open Questions

None.
