**Collaborator:** aidlc-devsecops-agent

## Contribution

セキュリティエンジニアの観点から、リード草案（`team-practices.md` / `discovered-rules.md` / `evidence.md`）を一次証拠と突き合わせて独立検査した。検査対象の性質は「ローカル実行の開発 CLI ツール、ネットワーク I/O なし、シークレット扱いなし」であり、この規模・性質に見合う水準で評価している。過剰装備の提案は意図的に避けた。

### 裏取り結果（一次証拠を自分で確認）

- **CI ゲート構成**（`.github/workflows/ci.yml` 全文読了）: `cargo fmt --all --check` → `cargo clippy --workspace --all-targets -- -D warnings` → `cargo lint` → `cargo test --workspace`、および quint / coverage ジョブ。草案の記述と一致。トリガーは `pull_request`（`main` 向け）+ `workflow_dispatch` のみで、`pull_request_target` のような特権コンテキストで信頼できない PR コードを実行する構成は**存在しない**。ワークフローはシークレットを一切参照していない。この 2 点は現状のベースラインとして健全。
- **SAST 相当の実効水準**: clippy 約50ルール deny（`Cargo.toml` `[workspace.lints]` 実測）+ rustc lint deny + カスタム `cargo lint`（赤例テスト31本）+ `#![forbid(unsafe_code)]` の組み合わせは、Rust プロジェクトにおける実効的な静的解析として十分な水準。追加の SAST 製品（CodeQL 等）は現時点では過剰装備であり、**不要と裁定する**。
- **DAST**: ネットワークサーフェス（API・常駐サービス・Web UI）が存在しないため対象外。導入しないことが正しい判断。
- **依存の極小性**（`Cargo.lock` 実測）: メイン workspace のロック済みパッケージは全60個（直接外部依存は serde / serde_json / md5 / nix / libc + dev の proptest / tempfile）。`tools/lint` は独立 `Cargo.lock`（5パッケージ）。**両方ともコミット済みであることを git 追跡で確認**——サプライチェーン管理の基礎（ビルド入力の固定）は成立している。
- **unsafe 封じ込め**: `#![forbid(unsafe_code)]` を workspace 9 クレートの lib.rs + `tools/lint` main.rs で実地確認。`grep` で `unsafe` の使用ゼロも確認。**ただし** `modules/app/aidlc/src/main.rs`（スタブ）には attribute が無く、`[workspace.lints.rust]` にも `unsafe_code` エントリが無い——強制はクレート個別 attribute 頼みである（codekb `technology-stack.md` の「infra-io を含め維持されている」は app スタブには当てはまらない。後述）。
- **ツールチェーン**: `rust-toolchain.toml` 不在（実地確認、codekb 記載どおり）。CI は `dtolnay/rust-toolchain@stable`（floating）。GitHub Actions はいずれもタグ参照（`@v4` / `@v2`）で SHA ピン留めなし。ワークフローに `permissions:` ブロックが無く、`GITHUB_TOKEN` はリポジトリ既定権限に依存している。
- **入力境界の防御**: `modules/core/interface-adapter/tests/` にシンボリックリンク防御の統合テストが存在（草案記載どおり）。ファイルシステム境界の入力検証がテストで固定されている点は評価できる。
- **md5 の用途**: ロック dir 名の upstream 互換導出という非暗号用途であり許容。codekb に用途が明記されているため将来の監査ノイズにもならない。

### 草案の欠落（DevSecOps 観点）

`team-practices.md` にはセキュリティ実践への言及が実質なく、`evidence.md` の「インタビューで確認」6項目にもセキュリティ関連が 1 件も含まれていない。lint/format/テストの網羅は優れているが、**依存脆弱性とサプライチェーンだけが実践の空白**になっている。規模相応・低コストの範囲で、以下をインタビュー確認事項に追加すべき（番号は既存 1〜6 への続番として提案）:

7. **依存脆弱性監査の導入**: `cargo audit`（RustSec advisory DB）を CI に追加するか。ロック済み60パッケージの極小ツリーなので実行コストは数秒。ブロッキングゲートにするか、週次 schedule + PR 非ブロッキングにするかはオーナー裁定。対象には `tools/lint` の独立 `Cargo.lock` も含めること（C27 の CI 未接続と同根の穴——監査からも漏れる）。`cargo-deny` への拡張（license / bans / sources）は、crates.io 公開・バイナリ配布の intent が確定した時点で license チェックを足せば十分であり、現時点では任意。
8. **ツールチェーン固定の裁定**: `rust-toolchain.toml` 不在 + CI floating stable の現状を維持するか。固定は再現性とサプライチェーンの両面で利点があるが、定期 bump の運用コストが伴う。stable 追従を続けるなら「stable 更新に伴う clippy 新規 lint で CI が突然赤くなる」リスクの受容を明文化する（`-D warnings` 運用のため実際に起こりうる）。
9. **低コスト・ハードニング 3 点のオーナー承認**（いずれも 1〜数行の変更、ただし lint 一式は 2026-08-22 オーナー規約に属するため承認が要る）:
   - `unsafe_code = "forbid"` を `[workspace.lints.rust]` へ昇格する。stage-1 で新設されるコード（ユースケース本体・composition root・CLI）こそ attribute の付け忘れが起こる場所であり、workspace 昇格なら機械検出される。草案自身が掲げる「規則の機械化優先順（型→既存 lint→`cargo lint`）」とも整合する。
   - `.github/workflows/ci.yml` に `permissions: contents: read` を明示する（least privilege）。現状はシークレット未使用・`pull_request` トリガーのみでリスクは限定的だが、既定トークン権限への暗黙依存を断つのが secure-by-default。
   - GitHub 側の Secret scanning + Push protection の設定状態を確認する。扱うシークレットが無い性質上、gitleaks 等の CI ゲート追加は過剰装備であり、リポジトリ設定（パイプラインコスト 0）で足りる。
- （任意・裁定のみ）GitHub Actions の SHA ピン留め: シークレット無し・`pull_request` のみの現状では追加リスクは限定的。mandate せず、導入するなら Dependabot（`github-actions` + `cargo` エコシステム）とセットでオーナーの好みに委ねる。

### 事実確認の注記

- codekb `technology-stack.md` の「`#![forbid(unsafe_code)]` は infra-io を含め維持されている」は、`modules/app/aidlc/src/main.rs`（スタブ、attribute なし）を含めると厳密には過大である。草案はこの主張を直接引用していないため草案の欠陥ではないが、上記 9 の workspace 昇格で記述と実態のずれ自体が解消される。
- `evidence.md` インタビュー項目 4（Deployment）がバイナリリリースの署名・チェックサムに言及している点は、配布時サプライチェーンへの正しい先回りであり支持する。SBOM・ビルド来歴（provenance attestation)の検討も、その配布 intent の時点で行えば十分——今ではない。

## Positions

- AGREE: Code Style の「リンタ 3 段構え」と discovered-rules の CI ブロッキングゲート規則 — `.github/workflows/ci.yml` と `Cargo.toml` `[workspace.lints]` の実地確認結果と完全に一致し、この性質のプロジェクトにおける SAST として十分な水準。追加 SAST は不要という判断も含めて支持する。
- AGREE: Deployment を org.md 既定のまま採らず「インタビューで確認」に回した判断 — Web サービス前提の deploy-on-merge は CLI 配布の実態に合わず、署名・チェックサム等の配布時セキュリティを確認事項に含めている点も適切。
- AGREE: `tools/lint` の CI 未接続（C27）をインタビュー項目 5 として明示した点 — 検証ツール自身が検証されない穴はサプライチェーン観点でも正しい指摘。ただし同クレートの独立 `Cargo.lock` が依存監査からも漏れる点（上記 7）まで含めて扱うべき。
- OBJECT: 「インタビューで確認」6 項目にセキュリティ実践が 1 件も無い — 依存脆弱性監査（`cargo audit`）の要否、ツールチェーン固定の裁定、`unsafe_code = "forbid"` の workspace 昇格を含む低コスト・ハードニングの 3 点（本文 7〜9）は、この規模・性質でも省略してよい水準ではなく、オーナーインタビューで裁定を得るべき。過剰装備（追加 SAST 製品・DAST・CI シークレットスキャナ）を求めるものではない。
