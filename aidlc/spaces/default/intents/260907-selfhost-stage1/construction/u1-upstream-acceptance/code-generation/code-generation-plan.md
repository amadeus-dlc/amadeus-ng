# U1: 本家2.7.1の受入基盤の実装計画

## 対象と完了条件

本家固定コミット `a277af218f0df7f325d3b8be7b6d90fce2c5bd40` の `dist/claude/` を実行し、配布JSON・CLI・フック・正規化/ハッシュの比較材料を `tests/golden/upstream-a277af21/` に採取する。契約C1の来歴、入力、標準出力・標準エラー、終了コード、状態・監査の差分を追跡可能にする。

FR1のうち採取と比較基盤を担当する。U2が既存のRust実装・テスト参照・比較項目を2.7.1に揃えた後に、B1として統合する。U1の検査成功だけでFR1やセルフホストの達成とはしない。既存の2.6.40採取バイトを上書きしない。現行受入参照を残さない移行・不要資産の整理はU2で一括確認する。

会話で裁定済みの2.7.1を採用する。下のTesting Contractに含まれる過去の「採用版は未裁定」は採取時点の記録であり、再裁定事項ではない。CQS（更新と読取りの分離）の例外は設けず、Rustの報告イベント・RMU・クエリ変更はU2が担当する。

## 現コードで確認した差分

- `recapture-cli.sh:24–35` は2.6.40、配布262ファイル、旧マニフェストへ固定されている。
- `capture-cli.ts:172–183` は複数の保護を無効化して採取する。これだけで承認保護を検証したとは扱えない。
- `capture-cli.ts:292–310` は既存の正規化設定と採取元メタデータを入力とし、CLIと主要4フックを採る。主なケースはclassicの連続操作である。
- `capture-supplemental.ts:9–25` も旧ピン固定。複数部分の継続配送と会話中の停止制御などを補っている。
- 旧正規化はUUID形と長いbase64url文字列を一括置換する。C1が求める入力間の識別子対応を新基盤で検証する必要がある。
- `git show a277af21:dist/claude/.claude/tools/aidlc-version.ts` の版宣言は `2.7.1`。配布元forkの作業ツリーは採取元として流用しない。
- 今回は調査と計画のみ。採取・新規テスト・Rustの全件テストは未実施。

## テストする公開境界

Plan Approvalは次の3境界も確認対象とする。内部関数の呼出し順や実装の字面をテストしない。

| 境界 | 観測する契約 | テストファイル |
| --- | --- | --- |
| 採取入口 | 固定した配布物だけを受理し、異なる版・改変・欠落を拒否し、失敗時に既存の採取先を壊さない | `scripts/goldens/capture-source.test.ts` |
| 採取結果 | 入力・出力・状態/監査・前提条件・来歴を再生可能な形で保存し、プロセス失敗を取りこぼさない | `scripts/goldens/capture-corpus.test.ts` |
| コーパス検査・比較CLI | 同一条件の再採取を照合し、固定文言・ID対応・欠落・余分なファイル・不正な正規化を検出する | `scripts/goldens/compare-corpus.test.ts` |

Standardに従い各境界5–8件を目安に、下のテスト手順書の7件ずつを実装する。新しい業務層、DB層、UIはこのpackaging単位に存在しないため追加しない。実ファイルと子プロセスを使い、時刻・乱数・取得失敗等の外部境界だけ必要に応じて制御する。

## 実装順

各振る舞いを1件ずつ、Red（失敗確認）→Green（最小実装）→Refactor（成功を維持した整理）の順に完了する。全テストを先に書く方式は採用しない。

- [x] **Step 1 — 実行基盤と開始時点の確認**（NFR2・NFR3・NFR4）
  - 実装前に既存Bun検査と `cargo test --workspace` の基準結果を取る。失敗があれば変更による失敗と区別する。
  - Bunの既存テストランナーを使用する。承認後、最初のテストファイルを用意して `bun test ./scripts/goldens/capture-source.test.ts` がテストを実行することを確認する。コマンド不在・テスト未検出はRedに数えない。
  - 既存作業を保持し、U1の変更範囲を下表へ限定する。Rustの本体と受入参照移行には着手しない。
- [x] **Step 2 — 採取元の検証をTDDで実装**（FR1・NFR1・NFR2）
  - Red: 固定元の正常受理から始め、異なるコミット、ファイル改変/欠落/追加、取得失敗、出力先保護の各ケースを順番に追加する。
  - Green: 既存採取入口を2.7.1へ切り替え、固定コミットから配布ファイル一覧とSHA-256を実測して固定する。実行前に全体の一致を検証する。取得方法が違っても同じ検証を必須にする。
  - Refactor: CLI/フック・ハッシュ・補完採取で必要な固定情報と検証だけを共有する。任意の版を信用する汎用ダウンローダーには広げない。
- [x] **Step 3 — 観測保存をTDDで実装**（FR1・NFR1・NFR2・NFR4）
  - Red: 採取された標準入出力・終了値・状態/監査・初期ファイル・環境・逐次入力が欠けるケースを公開採取入口で再現する。
  - Green: 検証済み配布物を隔離した一時ワークスペースへ置き、既存のCLI・4フック・hash-canonical・補完ケースを2.7.1で実行する。生の観測と比較用データを区別し、失敗やsignal、初期化失敗を成功ケースへ丸めない。
  - 外部の実セッション・認証情報を使わない。親のAIDLC設定が採取へ混入しない環境を作り、採用設定とテスト用の合成前提を来歴へ記録する。
  - Refactor: 重複するファイル保存・子プロセス処理を必要な範囲で整理する。
- [x] **Step 4 — 比較と正規化をTDDで実装**（FR1・NFR1・NFR2）
  - Red: 固定文言の1バイト差、入力間のIDの取り違え、未知の差、ファイル欠落/追加、正規化の非対称を検出する試験を順番に追加する。
  - Green: `verify-corpus.ts` のCLIで採取来歴・必要ファイル・比較規則・再採取差分を検証する。終了値・標準出力/エラー・状態/監査の各面を比較し、キー集合だけの一致で成功としない。
  - 環境依存値は実測値に基づき対称に正規化する。識別子は役割ごとに対応関係を保ち、固定ID、固定コミット、監査語彙、状態値を消さない。継続トークンは返された値を次の入力に用い、別トークンを渡した拒否も別途検証する。
  - Refactor: 同一規則を両辺へ適用する責務を整理する。採取済み期待値を手で修正しない。
- [x] **Step 5 — スモークで使う契約の採取をTDDで補う**（FR1・NFR1・NFR2・NFR4）
  - 対象は要求確認の実測表と契約C2・C3・C6・C7から決め、固定コミットの該当ファイル/行を一覧に引用する。
  - CLIの開始・継続・報告、decision/answer/link/review、内容確認・実装計画承認、主要4フックと実発火する追加フック、更新補助処理、診断の採用部分をケースへ結び付ける。
  - Red→Green→Refactorを繰り返し、保護を有効にした不正入力、回答前、古い受領、別セッション、対象/内容変更後の拒否を採取する。既存の保護無効ケースと別に分類する。
  - 合成会話/初期ファイルを用いた契約テストは実地スモーク成功に数えない。採れない必要ケースは理由・根拠・引継ぎ先を記録し、必要範囲が未検証ならU1完了にしない。Rust固有の投影障害等は本家の観測を捏造せずU2の内部契約テストへ渡す。
- [x] **Step 6 — 独立した2回の再採取と全差分確認**（FR1・NFR1・NFR2）
  - クリーンな2つの出力先で採取し、配布JSONは完全バイト一致、決定的出力は完全一致、非決定値を含む出力はC1の明示規則適用後の一致を確認する。
  - 取得日時等の来歴差はフィールド単位で報告する。未知の差を一括除外しない。hash-canonicalの関数本体は固定元から抽出した実バイトと照合する。
  - 旧データとの差分を全数分類し、Rustの既存ゴールデン参照・キー集合比較・スキップの移行一覧をU2へ渡す。
- [ ] **Step 7 — CI接続とU1の引継ぎ**（FR1・NFR2・NFR3・NFR4）
  - 3つのBunテストを既存CIの検査対象へ追加する。通常CIでは保存コーパスを検査し、再採取時だけ固定元の取得を行う。既存検査・カバレッジ閾値は維持する。
  - 単位テストと採取検証を実行し、Red/Greenのコマンド・結果、未検証事項、U2への変更一覧を `code-summary.md` に残す。採取手順と根拠は同じ記録内に置く。
  - 全変更ファイルを `source-manifest.json`、FR1とNFR1–NFR4の対応を `traceability.json` に記録し、独立レビューを受ける。
  - U1単独ではmainへ統合しない。B1全体のカバレッジ・Quint/ITF・CI全ジョブ・レビュー収束はU2完了時にも確認する。

## 変更するファイルと責任

| 対象 | 変更内容 |
| --- | --- |
| `scripts/goldens/recapture-cli.sh`、`recapture-hash-canonical.sh` | ピン更新、検証、明示的な採取先指定 |
| `scripts/goldens/capture-cli.ts`、`capture-hash-canonical.ts`、`capture-supplemental.ts` | 2.7.1での採取、前提と全観測面の保存 |
| `scripts/goldens/upstream-source.ts` | 固定元情報と採取前検証の共有部分 |
| `scripts/goldens/capture-stage1.ts` | 実測したスモーク必須契約の追加採取 |
| `scripts/goldens/verify-corpus.ts` | 公開のコーパス検証・比較CLI |
| `scripts/goldens/capture-source.test.ts`、`capture-corpus.test.ts`、`compare-corpus.test.ts` | 上記3境界のテスト |
| `tests/golden/upstream-a277af21/` | 本家からの新規採取物、来歴、明示的な比較設定 |
| `.github/workflows/ci.yml` | U1検証を既存Bun検査へ追加 |
| 本単位の記録ディレクトリ | 実装/テスト手順、採取根拠、差分一覧、結果、変更ファイル一覧 |

本家・vendor・配布ステージ本文は編集しない。旧採取物は差分比較の入力として保全する。既存の自動検査修正と専用スコープはB1の前提変更で、U1の新規実装と混同しない。

## 品質基準と停止条件

workspace行カバレッジ床90.0%、相対条件 `head >= base - 0.01`、乱数シード20260823、既存除外1ファイルを維持する。U1のBun検査をRustカバレッジの代わりにしない。ネットワーク取得不能、必要な採取契約の矛盾、再現しない未知差、実装計画承認の拒否はそれぞれ明示し、成功扱いで先へ進まない。

## Testing Contract

```json
{
  "version": 1,
  "methodology": "tdd",
  "source": "team",
  "ordering": "各変更で失敗するテストを先に実行して red を確認し、最小の実装で green にしてから、テストの成功を維持しながら refactor する。",
  "scope": "selfhost-stage1",
  "test_strategy": "standard",
  "project_type": "brownfield",
  "applicable_notes": [
    {
      "layer": "org",
      "text": "We treat tests as a first-class deliverable in every Bolt. The specific\nmethodology (TDD, BDD, ATDD, or classic test-after) is affirmed at\npractices-discovery and recorded in `team.md` under this heading with explicit\n`Methodology` and `Ordering` fields; Code Generation resolves those fields\nindependently from coverage, tooling, and scope notes.\n\nWhen no posture has been affirmed, our default per scope is:\n- **Methodology**: test-after\n- **Ordering**: implement each applicable testable layer, then write and run\n  that layer's tests.\n- `mvp`, `enterprise`, `feature`, `infra`, `classic` add an 80% line-coverage\n  floor and CI execution before merge.\n- `bugfix`, `security-patch` add a targeted regression for the specific\n  bug/vulnerability and require the existing suite to remain green.\n- `express` uses the Minimal strategy: requirement-driven unit tests (one per\n  requirement, with a happy-path floor per component); existing tests remain\n  green.\n- `poc`, `refactor`, `workshop` add no extra new-test floor and require the\n  existing suite to remain green.\n\nThe active `Test Strategy` still applies in every scope and determines test\nvolume/types. Scope floors are additive; they never reduce or replace the\nselected strategy.\n\nBuild and Test verifies defined coverage floors and affirmed quality targets;\nthey may not be weakened to make a step pass.\n\nAffirm a stricter posture in `team.md` if the team commits to one."
    },
    {
      "layer": "team",
      "text": "- **Methodology**: tdd\n- **Ordering**: 各変更で失敗するテストを先に実行して red を確認し、最小の実装で green にしてから、テストの成功を維持しながら refactor する。\n- 上記は依頼原文で確定済みであり、`org.md` の未確定時の `test-after` 既定は適用しない。\n- workspace の行カバレッジ床 90.0% とベース比較を維持する。現設定の相対許容は 0.01 パーセントポイント、乱数シードは `20260823`、除外は `modules/app/aidlc/src/main.rs` 1 ファイルである。相対条件は `head >= base - 0.01` であり、絶対床90.0%に許容誤差を適用しない。これらは実測設定として保持し、テストを通すために緩和しない。\n- Quint（状態遷移を検査する仕様言語）3モデル、ITF（モデルの実行トレースを実装で再生する形式）、ゴールデン（観測結果の比較用データ）を外側の受入検査とする。ゴールデンの採用版は未裁定である。\n- `cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo lint`、`cargo test --workspace`、独立した `tools/lint` の検査を維持する。\n- `cargo audit` は依存ライブラリの脆弱性検査である。現在の CI 集約では必須対象外だが、今回の「CI 全ジョブ green」はこのジョブの成功も要求する。CI設定の変更が承認されたという意味ではない。\n- ゴールデンのバイト一致とキー集合の比較を区別し、未検証ケースと既知の出力差を要求分析へ渡す。既存検査の成功を完全互換の証明へ拡大解釈しない。具体的な限界は [evidence.md](evidence.md) に記録する。\n- 本工程ではテスト・カバレッジを再実行していない。既存設定の存在と、検査の成功を区別する。"
    }
  ],
  "obligations": {
    "strategy": "standard",
    "strategy_volume": [
      "Five to eight tests per component.",
      "Unit tests plus integration tests for key boundaries.",
      "Add E2E, performance, or security tests when requirements demand them."
    ],
    "scope_floor": [
      "Keep the existing test suite green.",
      "This scope adds no extra new-test floor beyond the selected test strategy."
    ],
    "combination_rule": "Apply every selected-strategy obligation and every scope-floor obligation; neither replaces the other, and a targeted scope regression may add the narrowest necessary test type beyond the strategy default."
  },
  "plan_profile": {
    "methodology": "tdd",
    "runner_step": "Verify the existing test runner/configuration and record the exact unit-scoped command.",
    "runner_ready_before_first_test": true,
    "testable_layers": [
      "Data model / database behavior",
      "Repository / data access",
      "Business logic",
      "API / endpoint",
      "Frontend behavior"
    ],
    "steps": [
      "Project structure and production configuration skeleton.",
      "Verify the existing test runner/configuration and record the exact unit-scoped command.",
      "Data model / database behavior - Red: write the failing tests and record the failing command output.",
      "Data model / database behavior - Green: implement only enough behavior to pass.",
      "Data model / database behavior - Refactor: improve the implementation while tests stay green.",
      "Repository / data access - Red: write the failing tests and record the failing command output.",
      "Repository / data access - Green: implement only enough behavior to pass.",
      "Repository / data access - Refactor: improve the implementation while tests stay green.",
      "Business logic - Red: write the failing tests and record the failing command output.",
      "Business logic - Green: implement only enough behavior to pass.",
      "Business logic - Refactor: improve the implementation while tests stay green.",
      "API / endpoint - Red: write the failing tests and record the failing command output.",
      "API / endpoint - Green: implement only enough behavior to pass.",
      "API / endpoint - Refactor: improve the implementation while tests stay green.",
      "Frontend behavior - Red: write the failing tests and record the failing command output.",
      "Frontend behavior - Green: implement only enough behavior to pass.",
      "Frontend behavior - Refactor: improve the implementation while tests stay green.",
      "Environment/build configuration.",
      "Documentation and traceability."
    ]
  },
  "input_sha256": "sha256:b510bda44274b1fcf27f9154152da5867f09514c2073fcd78fe101cf98ea7d07",
  "contract_sha256": "sha256:d904f82d20fc4ba2d0045d5697ecae08fac96371ae5ab4ab6ec4913ae045ab55"
}
```

## Sources

- [要求書](../../../inception/requirements-analysis/requirements.md): FR1、NFR1–NFR4。
- [単位定義](../../../inception/units-generation/unit-of-work.md): U1の責任とU2への引継ぎ。
- [契約書](../../../inception/contract-design/contract-summary.md): C1、C2・C3・C6・C7の採取対象。
- [実行計画](../../../inception/delivery-planning/bolt-plan.md): B1=U1+U2、直列統合。
- `scripts/goldens/` の既存5ファイル、旧 `normalization.json`・来歴、固定コミットの版宣言。

## Assumptions & Open Questions

採取する公開境界と実装順は本計画の承認対象。採取ツリーの件数・SHA-256、2.7.1で変化した出力の一覧は実行で確定し、未測定値を記入しない。上流と契約の意味が食い違う場合は裁定を求める。


## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-08T01:02:28Z
**Iteration:** 2
**Request Challenge:** review:fe7f9a07e1066bb9c2e6e938e62ac4e0

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|---|
| R-01 | Major | aidlc/spaces/default/intents/260907-selfhost-stage1/construction/u1-upstream-acceptance/code-generation/code-generation-plan.md > Step 4（scripts/goldens/corpus-normalization.ts > comparableObservation） | 生観測のinitial_files・changed_files・fixture_changesをbase64からUTF-8文字列へ復号するため、不正UTF-8の異なるバイトを同一視する。独立した2つのobservations.jsonでinitial_filesの同じパスにFFとFEをそれぞれ保存し、各コーパスをsealして比較すると、期待する終了1に対して実際は終了0となった。固定ファイルの不正UTF-8を検出する既存テストは、この観測JSON経路を通らない。 | 3つのファイル面でもバイトを失わない比較にし、それぞれのFF対FEを拒否する回帰テストを追加する。正常UTF-8の時刻・パスの対称正規化は維持し、採取した生バイトを変更しない。 | Resolved |
| R-02 | Major | aidlc/spaces/default/intents/260907-selfhost-stage1/construction/u1-upstream-acceptance/code-generation/code-generation-plan.md > Step 7（.github/workflows/ci.yml > aidlc-distribution / Checkout with pinned distribution） | 新しい4テストファイルはvendor内の祖先a277af21へgit archiveするが、CIのcheckoutはsubmodules:trueだけでfetch-depthの指定がなく、浅い取得では固定祖先が存在しない。fork tip801c5700をdepth=1で新規取得した隔離リポジトリで同じarchiveを実行すると、終了128・not a tree objectを再現した。既存ローカルの全履歴で成功した42テストではこのCI前提不足を検出できない。 | CIで固定祖先を確実に取得するか、固定元検証済みの資料をテストへ供給する。浅いクローンからの実行を検証し、暗黙のローカル履歴やネットワークなしというテスト前提に依存しない。 | Resolved |
| R-03 | Major | aidlc/spaces/default/intents/260907-selfhost-stage1/construction/u1-upstream-acceptance/code-generation/code-generation-plan.md > Step 7 / traceability.json | 指定のtraceability検査はexit0でもpass:falseを返し、missing_from_upstream_idsにFR2–FR8の7件を要求する。U1の責任は単位定義どおりFR1の採取・比較とNFR1–NFR4であり、これはU1の実装欠落ではなく自動検査の単位適用範囲の不一致である。現状のまま自動検証済みとして完了できない。 | 要求や単位の割当を変えず、検査が現在の単位へ割り当てられたFR/NFRを検証するよう、正規の同期パッチと回帰テストで是正する。割当済み要求の欠落を引き続き検出し、架空のFR2–FR8対応を足して回避しない。 | Resolved |

### 解消確認

前回の指摘内容・ID・重大度を保持し、再検証結果で状態を更新した。新たな指摘はない。

- R-01: comparableFileはUTF-8への往復が完全一致する場合だけ文字列として扱い、それ以外を数値バイト配列として比較する。3面それぞれで両辺の単体検査は成功し、FF対FEの相互比較だけが終了1となる回帰を実行確認した。readJsonは外側JSONの不正UTF-8も拒否する。既存の時刻・パス正規化、ID取り違え検出、固定文言差分の検査は維持され、保存済みコーパスと新規採取も一致した。
- R-02: aidlc-distributionの採取関連テストの前に、本家の完全SHAを指定するfetchが置かれた。回帰試験はfork tipだけをdepth1取得した隔離repoでarchiveの失敗を確認し、CIから読み出した取得コマンドの通信先だけをローカルへ置換して実行後、archive成功を検査する。固定元のマニフェスト検証は残り、forkの作業ツリーへ切り替える処理もない。レビューでは実際のGitHub CI全ジョブの成功を主張しない。
- R-03: FRの検査範囲は対象JSONの自己申告から導かず、DAG・単位定義・割当表の主担当/Directory/支援列から導く。未割当FRの混入、担当FRを宣言とcoverageの両方から削除、未宣言Unit、割当表/DAG欠落、説明欄だけのUnit言及を拒否する回帰が3配布先で成功した。zero-Unitでは全要求を検査し、既存US→ACの経路も保持する。共通NFRは従来どおり要求される。実際のU1の入力を変えず検査が成功し、3配布先の実装ハッシュも一致する。

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| bun test ./scripts/goldens/capture-source.test.ts ./scripts/goldens/capture-corpus.test.ts ./scripts/goldens/compare-corpus.test.ts ./scripts/goldens/capture-doctor.test.ts ./scripts/goldens/capture-learnings.test.ts ./scripts/aidlc-traceability.test.ts | PASS: 77 tests、358 assertions、失敗0 | 第2回レビューで再実行。上記3指摘の正常/拒否境界を含む。 |
| bun .codex/tools/aidlc-sensor-traceability.ts --output-path aidlc/spaces/default/intents/260907-selfhost-stage1/construction/u1-upstream-acceptance/code-generation/traceability.json | PASS: pass=true、findings_count=0 | 実際のU1割当とFR1/NFR1–NFR4を照合。 |
| bun scripts/goldens/verify-corpus.ts tests/golden/upstream-a277af21 /tmp/amadeus-u1-review2-a | PASS | 保存物の全ハッシュ検査と新しい比較器による再採取照合が成功。 |
| bun scripts/goldens/verify-corpus.ts /tmp/amadeus-u1-review2-a /tmp/amadeus-u1-review2-b | PASS | 独立採取2組を再比較。レビュー自体では全採取をもう一周していない。 |
| bun scripts/aidlc-sync.ts --check | PASS: コピー0、削除0、保持設定確認0 | 同期パッチ、3配布実装、installed.jsonが整合。 |
| 3配布先traceability実装のSHA-256 | PASS: 全て85bea6fd7f122fcb63b6de48cb642fd6546197f1a70cc89df247e97fa734feb4 | 同じ修正が適用されている。 |
| required-sections（code-generation-plan.md） | PASS: h2_count=9、findings_count=0（追記前） | 元の計画本文の構造を確認。 |
| git diff --check | PASS | 空白差分の問題なし。 |

第1回で確認したlinter/type-checkの対応設定不在を、今回のBun試験で成功へ読み替えていない。Rustの90%床、全CIジョブ、U2のRust適合とCQS是正、U3のNative診断、U4の実地スモークは本レビューの成功宣言に含めない。

### Summary

R-01〜R-03はすべて解消済みで、追加の根拠ある問題は見つからなかった。U1の採取・比較基盤は後続の受入移行へ引き継げる。U1単独の成功をFR1全体やセルフホスト切替の達成とは扱わない。
