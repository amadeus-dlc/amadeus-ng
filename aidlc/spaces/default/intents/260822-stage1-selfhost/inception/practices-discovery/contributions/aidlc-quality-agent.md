**Collaborator:** aidlc-quality-agent

## Contribution

品質エンジニア観点の独立検査。一次証拠（`.github/workflows/ci.yml` / `scripts/coverage.sh` / `scripts/quint-gate.sh` / `code-quality-assessment.md` / `design-audit-2026-08-22.md` / `coding-rules/README.md`）を全て自分で読み直し、加えて `gh api` でブランチ保護の実態を新規に実測した。HEAD は草案と同じ `c4d8d95` で照合している。

### 1. 裏取り結果 — 草案の実測主張はすべて一次証拠と一致

- CI `check` ジョブの順序（fmt → clippy `-D warnings` → `cargo lint` → `cargo test --workspace`）: `ci.yml` と一致。
- カバレッジ絶対床 `ABSOLUTE_THRESHOLD=90.0` / 相対ゲート `TOLERANCE=0.5` / 実測 94.87〜95.29% / シード固定後 0.01 への引き締め計画: `coverage.sh` 冒頭コメントと一致。
- 不変条件 run 27 本の内訳を `quint-gate.sh` から実数で確認: engine_loop 9 + audit_lock 10 + stop_hook 8 = 27。witness は audit_lock 7 + stop_hook 5 = 12。決定的シナリオは `quint test --match 'r_.*'`。草案の数値は正確。
- テストファイル実在確認: ITF 準拠 2 本（`modules/core/domain/tests/audit_lock_conformance.rs` / `engine_loop_conformance.rs`）、統合 4 本（golden_parity / fs_workspace_lock / append_only_symlink / workflow_definition_repository_impl）。草案の内訳と一致。
- `cargo lint` 機械強制 3 ルール（checkbox-vocabulary / reap-decision-locality / no-public-fields）と赤例テスト DoD: `coding-rules/README.md` と一致。

### 2. 【重大・新規実測】「ブロッキングゲート」の機械強制は存在しない

`gh api repos/amadeus-dlc/amadeus-ng/branches/main/protection` は **404 "Branch not protected"**、`gh api repos/amadeus-dlc/amadeus-ng/rules/branches/main` は **空配列**（2026-08-22 実測）。つまり CI 3 ジョブは PR で走るが、**赤のままマージすることを止める仕組みは GitHub 側に何もない**。全緑マージは現状「運用規律」であって「機械強制」ではない。

- discovered-rules.md の ALWAYS「…全ステップをブロッキングゲートとして実行する」は、CI が実行される事実と、赤でマージが止まる保証を混同している。このまま `team.md` へ昇格すると、存在しない強制を存在すると記録することになる。
- このプロジェクトの明文化された機械化原則（型 → 既存 lint → `cargo lint`、coding-rules/README.md）に照らすと、マージゲートだけが規律頼みで残っているのは原則との不整合でもある。

**修正案**（discovered-rules.md の当該 ALWAYS を置換）:

> ALWAYS マージ前に CI 3 ジョブを全緑にする — check（`cargo fmt --all --check` → `cargo clippy --workspace --all-targets -- -D warnings` → `cargo lint` → `cargo test --workspace`）、quint（`scripts/quint-gate.sh`）、coverage（`scripts/coverage.sh`、絶対 90% 床 + PR 相対ゲート）（`.github/workflows/ci.yml` 実測）。

**evidence.md への追記案**（検査した証拠に追加）:

> `gh api repos/amadeus-dlc/amadeus-ng/branches/main/protection` → 404 Branch not protected、`.../rules/branches/main` → `[]`（2026-08-22 実測）。CI 全緑マージは branch protection / ruleset による機械強制ではなく運用規律である。

**インタビュー項目の追加案**（必須扱いを推奨）: required status checks（check / quint / coverage）を `main` に設定して全緑マージを機械強制するか。設定するなら stage-1 のスコープ（D 束の隣）に含めるか。

### 3. 【重大】Testing Posture の `- **Ordering**:` フィールドが自己完結していない

org.md の契約は「Code Generation resolves those fields independently from coverage, tooling, and scope notes」——つまり Code Generation は **Ordering フィールド単体を機械読取する**。現行文言「各テスト可能レイヤーについて、red → green → refactor のサイクルを回してから次のレイヤーへ進む」には 2 つの問題がある:

1. **周辺散文との矛盾**: 直後の段落は「TDD サイクルは主にユニットテスト層に適用し、ITF 準拠テスト・ゴールデンパリティは TDD サイクルの外側」と正しく限定しているが、その限定は Ordering フィールドの外にある。フィールドだけを読む Code Generation は「ITF 準拠テストも red 先行で書く」と解釈しうる（契約の正本が Quint 側にある以上、これは実装不能な指示になる）。
2. **「次のレイヤーへ進む」条件が未定義**: サイクル完了の判定（新規テスト緑だけか、既存スイート緑 + lint 緑まで含むか）が書かれていない。

**置換文言案**（インタビュー #3 の裁定が草案どおり「外側のゲート」に確定した場合。裁定が変わればそれに合わせてこのフィールド内に折り込むこと——散文への退避は不可）:

> - **Ordering**: ユニットテスト層（インライン `#[cfg(test)]`、集約本体は PBT 同居）は red（失敗するテストを先に書く）→ green（最小実装で通す）→ refactor（設計規則・lint に適合させて整理する）のサイクルを実装に先行して回す。ITF 準拠テストとゴールデンパリティは TDD サイクルの外側のレイヤー横断受け入れゲートとし、当該実装の完了後・同一 Bolt 内で緑にする。あるレイヤーのサイクル完了条件は「新規テスト緑 + 既存スイート緑 + lint 緑」であり、これを満たしてから次のレイヤーへ進む。テスト配分はテストピラミッドに従う（ユニット層を厚く、結合・E2E 層を薄く）。

なお `- **Methodology**: tdd` は org.md の値例（`test-after`）と同形式で問題ない。

### 4. discovered-rules のリンタ ALWAYS は CI の一部しか記述していない

現行文言は check ジョブの 4 ステップのみを列挙し、quint ジョブ（形式検証ゲート）と coverage ジョブ（90% 床 + 相対ゲート）が漏れている。この 2 ジョブも `ci.yml` 上は check と同格の PR ゲートであり、team.md へ昇格された際にマージ条件の正本が不完全になる。§2 の置換案は 3 ジョブを揃えて記述しているので、そちらの採用で同時に解消する。加えて、org.md 既定の 80% 床より厳しい **90% 床 + 相対ゲート** は「stricter posture の affirm」に相当するため、Testing Posture の昇格文面に確実に残すこと（現草案は残している——維持を支持）。

### 5. `skeleton: off` の根拠文言が証拠より強い

三層品質保証（Quint / ITF / ゴールデン）が実証しているのは**決定論コア〜アダプタ層まで**である。walking skeleton の目的である「薄い縦串の疎通」の対象——ユースケース本体 → composition root → CLI の縦経路——は現状**テスト 0 本・コード未着手**であり、何も実証していない。「skeleton の目的は、既存の三層品質保証が既に果たしている」（team-practices.md § Walking Skeleton）は言い過ぎで、正確には「アーキテクチャ内側は実証済み、未着手の縦串は未実証」である。

**修正案**: 当該段落を「アーキテクチャの内側（決定論コア〜アダプタ層）の疎通と契約適合は三層品質保証が実証済みである。一方、未着手のユースケース本体〜composition root〜CLI の縦串は未実証であり、skeleton を立てるかどうかはこの縦串をどの Bolt で最初に通すかの裁定に等しい」と弱め、`skeleton: off` は提案のままインタビュー #1 で裁定する（結論自体には異議なし。inside-out の残工程を Bolt 1 で最小疎通させるなら、実質それが skeleton の役割を果たす）。

### 6. インタビューギャップの追加提案（優先度付き）

インタビュー疲れを避けるため、**必須は (a)(b) の 2 件**、残りは delivery-planning での裁定に回してよい。

- **(a) 必須**: マージゲートの機械強制（§2 — required status checks を設定するか）。
- **(b) 必須**: **未着手レイヤー（composition root / CLI）のテスト戦略とカバレッジ扱い**。テストピラミッドの頂点（E2E / CLI スモーク）が現状未定義であり、stage-1 = セルフホスト切替の受け入れ確認はまさにこの層になる。また composition root / `main.rs` はユニットテスト困難な典型であり、workspace 全体 90% 床に新規の未計測コードとして直撃する。`coverage.sh` には除外設定が一切ない（意図的に既定のまま）ため、「除外を許すか、書き方（薄い main + テスト可能な App 関数）で吸収するか」の方針が要る。
- (c) 推奨: **macOS 検証**。CI は `ubuntu-latest` のみだが、セルフホスト先はオーナーの macOS 実機。FS ロック・シンボリックリンク防御・`mktemp` 互換などプラットフォーム感応の実装が既にあり、スクリプト側は bash 3.2 互換まで配慮済みなのに CI に macOS ジョブがない。stage-1 の完了条件に macOS ジョブ追加（または「ローカル実機検証で代替」の明示的裁定）を含めるか。
- (d) 任意: `main` への push トリガーが無く、マージ後の `main` は次の PR まで未検証（`workflow_dispatch` 手動のみ）。PR 直列運用ならリスクは小さいが、容認の明示を取るか。
- (e) 任意: ITF 準拠テストは engine_loop / audit_lock の 2 モデルのみで **stop_hook は未カバー**。stage-1 スコープで stop_hook 相当を Rust 実装するなら、その Bolt の Ordering（外側ゲート）に stop_hook の ITF 準拠テスト追加を含めるか。
- (f) 任意: `rust-toolchain.toml` 不在（既知・微細、code-quality-assessment 束外観察）。stable 追従のため clippy 新規 lint がローカル/CI 間・時点間で差分を生みうる。固定するか。

### 7. 細部の精度向上（軽微・統合時に取り込み可能）

- team-practices.md § Testing Posture の「ITF 準拠テスト（`modules/core/domain/tests/`、2本）」に、対象モデル名（engine_loop / audit_lock）と再生トレース数 15 本を併記する（`code-quality-assessment.md` §品質保証の全体像と整合し、(e) の未カバー判定も読み手が自力でできるようになる）。
- インタビュー #5 の見出しは C27 のみだが、**C28（理由なし `// amadeus-lint: allow(rule)` の抑制成立）** は project.md に昇格済みの「規則は…`cargo lint` で強制される」ルールの実効性を直接損なう穴なので、D 束一括の中に埋めず見出しに併記する。
- Quint モデルの mutation テスト（engine_loop 3/3 等）は**一回性の証明**であり、`quint-gate.sh` の再帰ゲートには含まれない。将来 Bolt でモデル自体を改訂する場合の DoD（mutation 再検証を要するか）は未定義——規則化するかは任意の追加確認でよい。
- discovered-rules の trunk-based NEVER 2 件・squash-merge・PR 直列は git/PR 履歴実測 + オーナー明言に裏付けられており、証拠等級の扱い（履歴だけでは直列を断定せず明言を第一級証拠とした evidence.md の判断）は適切。支持する。

## Positions

- AGREE: `- **Methodology**: tdd` の値と、TDD をユニット層に適用し ITF/ゴールデンを外側のレイヤー横断ゲートと位置づける整理 — 実行経路の実態（実装同時 vs Quint モデル駆動の別経路）と一致し、オーナー明言「t_wada 流 TDD + テストピラミッド」とも矛盾しない。
- AGREE: カバレッジ 90% 床 + 相対ゲート 0.5pp を org 既定 80% より厳しい affirm として Testing Posture に残す判断 — `coverage.sh` 実測と一致し、後退を防ぐ。
- AGREE: インタビュー #2（ピラミッド比率の定量化要否）を立てたこと — inception ガードレール「曖昧語は計測可能な閾値とセット」に照らして正しい提起。数値比率をオーナーが不要とする場合の代替として「新規挙動は既定でユニットテスト、結合はアダプタ境界のみ、E2E は CLI スモークのみ」という配置規則での充足を提案する。
- AGREE: 推測を規則化せず、オーナー明言と機械強制済み規約のみを discovered-rules に載せた抑制 — 証拠等級の規律として適切。
- OBJECT: discovered-rules の「全ステップをブロッキングゲートとして実行する」 — branch protection / ruleset が未設定（`gh api` 実測 404 / 空配列）であり、機械強制は存在しない。§2 の置換案と evidence 追記・インタビュー必須項目化が必要。
- OBJECT: `- **Ordering**:` フィールドの現行文言 — Code Generation が単体で機械読取する契約に対し、ITF/ゴールデンの限定が散文側に退避しており、フィールド単体では「全レイヤー red 先行」と誤読可能。§3 の置換案で自己完結させること。
- OBJECT: Walking Skeleton の根拠文言「skeleton の目的は既存の三層品質保証が既に果たしている」 — 三層が実証するのはコア〜アダプタまでで、縦串（ユースケース〜CLI）はテスト 0 本。§5 の弱めた文言に差し替えること（`skeleton: off` の結論自体には異議なし）。
- OBJECT: インタビュー一覧に品質ゲートの欠落が 2 件不足 — (a) マージゲートの機械強制、(b) 未着手レイヤー（composition root / CLI）のテスト戦略と 90% 床への影響。いずれも stage-1 の完了条件に直結するため必須項目として追加すること。
