# ADR 0003: Quint の適用範囲と運用

- **ステータス**: **Accepted**（2026-08-22 オーナーレビューで確定: 決定 7 のマージゲートは「実験として採用」— 試行条項を参照。敵対的レビュー 1 巡反映済み）
- **日付**: 2026-08-22
- **対応**: `00-policy.md` §1 D9・§3 A9。リスク R6（exit/panic 経路のロック漏れ）の検査手段。R7 は**ガードの状態機械論理のみ**本 ADR が受け持ち、ダイジェスト計算互換の本体はゴールデン互換層（00-policy §7 検証戦略 (1)）が受け皿
- **関連**: `01-domain-model.md` §1-3（E4 分類）・§6（状態機械の三陣）
- **環境実測**: quint **0.32.0**（ローカル確認済み）。`quint run` = ランダムシミュレーション（`--out-itf` で ITF トレース出力、`--invariants` 複数指定、rust バックエンド）。**`--max-samples` の既定は `--seed` 指定時 1、それ以外 10000**（--help に明記。シード固定だけでは 1 トレースに縮退する）。`run` に `--temporal` は無く、時相性質は `quint verify`（Apalache。`--invariant` / `--temporal` / `--inductive-invariant`）でのみ検査できる。同一シード＋明示 `--max-samples` の再実行は決定的（実測確認済み）。ITF 出力のトップレベルは `#meta` / `vars` / `states` のみで**アクション情報を含まず**、`#meta` には timestamp が埋まるためファイルはバイト再現しない。

## コンテキスト

D9 で Quint 採用は確定済み。未決なのは適用範囲の確定、モデルの配置、CI への組み込み方、そして ITF トレースによる実装準拠テストの設計である。決定論コアは状態機械が密（抽出 74 件、実体約 55 件）で、特にエンジンループ・監査ロック・swarm 収束は「実装してから発見すると最も高くつく」領域になる。

## 決定

1. **適用範囲は `01-domain-model.md` §6 の三陣を正式採用**する。第一陣（directive loop / checkbox+effectivePlanAction / ApprovalGate / 監査ロック / park・jump・per-unit カーソル / Stop フック forwarding loop）はフェーズ A の**実装前**にモデル化する。ステージグラフのコンパイル不変条件は状態遷移ではないため Quint 対象外とし、Rust の proptest で検査する（`00-policy.md` §3 A9 のとおり。これに伴い 01 §3.1 の当該不変条件の E4 タグは proptest に改訂済み）。
2. **E4 トレーサビリティ規約**: コンテキスト別仕様で E4 に分類した不変条件は、対応する Quint 定義名（`module::invariant_name`）を必ず併記する。名前のない E4 は認めない。この grep が検出するのは「E4 不変条件に対応する定義名がモデルに存在するか」**のみ**であり、モデル・コード間の命名同期は決定 6 の対応表が担う。
3. **配置とモジュール構成**: コードリポジトリ（A8）のルートに `formal/` を置き、コンテキスト別サブディレクトリに置く。1 状態機械 1 モジュールを基本とし、**密結合な機械群は 1 モジュールへの合成を認める**。第一陣の対応は次で確定する:
   - `formal/orchestration/engine_loop.qnt` — directive loop＋ApprovalGate（skeleton 往復含む）＋CheckboxState/effectivePlanAction＋park・jump・per-unit カーソルの合成モデル
   - `formal/orchestration/stop_hook.qnt` — forwarding loop（no-progress counter・carve-out）
   - `formal/workspace/audit_lock.qnt` — 監査ロック＋audit-first（クラッシュ・reap・再入を含む）
   A8 確定（ADR 0005）に伴いリポジトリルート `formal/` へ移設済み。
4. **CI 構成**:
   - 毎 PR: `quint typecheck` ＋ `quint run --seed <固定値> --max-samples <明示値（初期値 1000、較正で調整）>` をモデルごとに実行。**シード固定でも `--max-samples` を明示しなければ 1 トレースに縮退する**（環境実測）ため、明示を必須とする。この構成は決定的で高速。**検査範囲は invariant のみ**で、temporal 性質はここでは検査されない。
   - nightly ＋エンジン関連変更時: シードなしのランダム実行（サンプル数大きめ。失敗時は quint が出力する再現用シードを記録）と `quint verify`（Apalache — invariant ＋ **temporal**。JVM を CI にセットアップ）。
   - **フィクスチャ鮮度ゲート**: `.qnt` 変更を含む PR では ITF フィクスチャ再生成ジョブを走らせ、`#meta`（timestamp 等）を正規化した上での diff 一致を強制する（不一致は fail）。distribution の drift guard と同じ「生成物は純関数」の規律をこの層にも適用する。
   - quint CLI のバージョンは devDependencies（package.json）でピン留めする。開発側 npm 依存であり、配布物のランタイム排除（D1）とは矛盾しない（00-policy §7 と同じ扱い）。
5. **ITF 準拠層（モデル準拠テスト）**: ITF トレースにはアクション情報が含まれない（環境実測）ため、**モデル規約として `lastAction` 相当の状態変数を持たせ**、トレースにどの遷移を取ったかを残す。Rust 側は `itf` クレート（リポジトリ名は informalsystems/itf-rs）でトレースをパースし、`lastAction` で **domain 層の純粋ステップ関数**を駆動して各状態を照合する。`lastAction` で一意に駆動できない非決定箇所は「`next_state ∈ step(prev_state)` のメンバーシップ検査」で照合する。フィクスチャは `#meta` を正規化して `tests/conformance/fixtures/` にコミットし、Rust CI のジョブは npm 不要とする（再生成は決定 4 の鮮度ゲートが担う）。準拠テストが CLI ではなく domain 層 API に結合することは、クリーンアーキテクチャ（01 §7）の強制力としても働く — ステップ関数が用意できない設計はこのテストが書けないので、その時点で設計を疑う。
6. **モデルとコードの対応規約**: モデルの状態変数名・アクション名は正準用語（01）から導出し、各 `.qnt` のヘッダに「モデル型 ↔ Rust Domain Primitive」の対応表を置く。この表の陳腐化は決定 2 の grep では検出できないため、レビュー項目とする（機械可読化＋型名存在照合の CI 化は未確定事項）。
7. **第一陣の完了条件（Definition of Done）**:
   - `engine_loop.qnt`: 「有効プランが SKIP のステージに run-stage を放出しない」「ゲート迂回トレースが存在しない」「park → resume で位置が保存される」「stale re-report が冪等 done になる」を invariant / temporal として検査し green。
   - `audit_lock.qnt`: 「相互排他」「クラッシュ（exit・panic 相当のアクション）を含む全経路でロックが解放または reap 可能」「audit-first（監査 emit 失敗時に state が変化しない）」を検査し green。
   - `stop_hook.qnt`: 「no-progress counter が cap 到達で必ず forwarding を解放する（停止できないトレースが存在しない）」「carve-out の許可経路が固定順で forwarding に勝つ」を検査し green。
   - 以上が揃うまで対応する Rust 実装をマージしない。
   - **mutation テストの必須化**（2026-08-22 追加 — 第 1 回実験の学び）: 各 named invariant について、対応するガード・遷移を意図的に壊した変異モデルが violation になることを確認し、検査力を証明する。素朴な green は「ガードで違反状態が作れないだけの空回り」でありうることが slice 1 v1 で実証された（チェック除去後も 10000 サンプル green、恒真式の invariant も 1 本）。等価ミュータント（到達不能な単一ガード除去）は検査対象にならないため、意味のある変異を選ぶこと。
   - **試行条項**（2026-08-22 オーナー決定）: このマージゲートは**実験として**採用する。最初のモジュール `engine_loop.qnt` の完了時点で、コスト（所要日数・手戻り）と効果（発見された仕様矛盾の数と質）を評価し、残り 2 モジュールおよび第二陣への適用を再判断する。評価結果は本 ADR に追記する。「どこまで現実的か」を測ること自体を第一陣の成果物とする。
     - 第 1 回評価データ（2026-08-22）: slice 1 v1 は typecheck 一発・run green だったが、敵対的レビュー＋mutation テストで空回り 2 件・恒真式 1 件・忠実性バグ 1 件（jump forward の介在 Pending 素通し）を検出。v2 で prev 状態スナップショットによるフレーム条件等を導入し、9 不変条件 green（2000×40）＋mutation 3/3 検出を確認。モデル化の副産物として upstream 未明文化の派生不変条件（at_most_one_active）も顕在化した。「green の質」を測る mutation テストの必須化を上記のとおり決定に昇格。
     - **試行の正式評価（2026-08-22、第一陣 3/3 完了時）**: engine_loop v2 / audit_lock v2 / stop_hook v1 のすべてで「モデル執筆 → green → 敵対的レビュー/mutation → 是正」ループが実装前に本物の欠陥を検出した（忠実性バグ 1・空回り/恒真 3・経路盲目 4・要件顕在化 2 — seed-2 の handoff 環境遷移と upstream 未明文化の at_most_one_active を含む）。コストは 1 モデルあたり抽出込みで小さく仕様理解を加速する側に働いたため、**マージゲートは第二陣（bolt / swarm_convergence）にも継続適用する**。DoD は「named invariant ごとの mutation ＋状態遷移レベル不変条件の併置＋ in-module witness（浅い経路は負形式 run、深い経路は決定的シナリオ `run r_*` ＋ `quint test`）」の 3 点で確定。
     - 第 2 回評価データ（同日、audit_lock.qnt）: v1（5 不変条件 green・mutation 3/3）に対し敵対的レビューが**別種の穴**を実証 — 所有権移転がアクションラベル経由でしか守られておらず、経路を変えた違反（acquire 横取り・非保持者解放・死者 reap・crash がロックを消す）が全不変条件を素通り。教訓は「**観測ラベル依存の不変条件は経路を変えた違反に盲目 — 状態遷移レベル（prev→current の関係）の不変条件を併置せよ**」。v2 で 4 本追加し 9 不変条件 green（5000×40）＋mutation 9/9。あわせて**到達性 witness のモジュール内定義**を規約化（負形式 `--invariant "not(w_x)"` で violation = pass。一時的な反証 run はファイル外に証拠が残らず、経路消滅の退行に CI が盲目だった）。以後のモデルは (1) named invariant ごとの mutation、(2) 状態遷移レベル不変条件の併置、(3) 到達性 witness の in-module 定義、を DoD とする。

## 帰結

- モデル・仕様・実装の三つ組の保守コストが、対象を三陣に限定した形で発生する。E4 の named-invariant 規約により、そのコストが「空文句の形式手法」に堕ちることを防ぐ。
- ITF フィクスチャの再現性は**「`#meta` を除く内容一致」**であり、バイト一致ではない。照合・鮮度ゲートは正規化 diff で行う（決定 4）。
- `lastAction` 変数の維持が全モデルの規約コストとして加わる（`--mbt` を不採用にした代替コスト）。
- 第一陣のモデル化がフェーズ A のクリティカルパスに入る。engine の `next` 21 分岐・`report` 13 段ガードの写経は仕様理解の最短経路でもあり、実装前の投資として妥当と判断する。

## 検討した代替案

- **TLA+ 直書き**: 不採用。Quint の型検査・CLI 統合・ITF 出力が開発ループに勝る（Apalache 経由で同じ検査系に到達できる）。
- **proptest のみ**: 不採用。到達可能性・時相性質・並行インターリーブが書けない。純関数の性質検査（グラフコンパイル等）は proptest 側が適任で、役割分担を決定 1 で固定した。
- **`--mbt`（model-based testing メタデータ）**: 当面不採用。実験的フラグへの依存は避け、`lastAction` 手動規約（決定 5）でアクション情報を代替する。安定化したら再評価し、規約コストと比較する。

## 未確定事項

- Apalache の JVM バージョンと CI セットアップの詳細、`--max-samples` / `--max-steps` の較正は第一陣モデルの実測で確定する。
- `itf` クレートの API 互換（バージョン・`lastAction` の型表現）は実装開始時に確認する。
- モデル↔コード対応表の機械可読化と型名存在照合の CI 化。
