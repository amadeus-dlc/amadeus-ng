**Collaborator:** aidlc-devsecops-agent

## Contribution

2026-09-07、規則草案4点と依頼原文を、既存の検査設定・配布管理実装に照らして独立確認した。`HEAD` と `main` はともに `1dc727e00a26c27258d916cb3a5e0592c6c48c1c`、配布元は `801c570062f67dc8f4952ee5fc601381d09db7ec`。`mise trust` は未信頼設定なしを返した。他の支援者の成果物は参照していない。

### 既存の検査と草案への補足

| 観点 | 確認した事実 | 草案へ統合する内容 |
|---|---|---|
| 整形 | `rustfmt.toml` は `style_edition = "2024"`、幅100、Unix改行。`ci.yml:80` 以降で workspace と独立した `tools/lint` の双方に整形検査を実行する。 | 草案の既存設定維持に同意する。 |
| 静的検査 | `Cargo.toml` の workspace lints は unsafe を禁止し、unwrap・expect・panic・添字アクセス等を deny にする。各10クレートの manifest に workspace lints の継承がある。独立クレートは `tools/lint/Cargo.toml` で unsafe 禁止を別途定義する。 | `clippy.toml` がテスト内の unwrap / expect を許容する点も適用範囲に含める。全コードで一律禁止と書かない。 |
| 独自lint | `.cargo/config.toml` の `cargo lint` は `tools/lint` を起動する。`tools/lint/src/check.rs:27` 以降の6規則と `domain_getter` の1規則で計7規則。 | 規則本文より検出範囲が狭いという草案の注意は妥当。lint成功を全設計規則の遵守証明にしない。 |
| 依存監査 | `ci.yml:164` 以降の audit ジョブが `cargo audit` と `cargo audit --file tools/lint/Cargo.lock` を実行する。`ci-success` の `needs` に audit はない。 | 今回の「CI全ジョブ成功」は audit も含む。集約成功だけで完了としない。既存の集約設定を変更する承認とは解釈しない。 |
| 専用セキュリティ検査 | `.github/`、`scripts/`、`.cargo/` と対象manifestを検索し、CodeQL・Semgrep等の専用SAST（ソースコードの脆弱性検査）、DAST（実行中のアプリに対する検査）、Gitleaks等の秘密情報検査の実行設定は確認できなかった。 | rustc・Clippy・独自lint・依存監査が存在することと、専用セキュリティ検査一式の導入済みという主張を分ける。GitHub側の秘密情報検査設定は本調査では未確認。未確認を無効と断定しない。今回新規導入の要件は追加しない。 |

依存ライブラリに現在どの脆弱性があるかは判定していない。上記は現物の設定確認であり、`cargo audit`、全テスト、最新CIを再実行・再取得した結果ではない。

### 配布元と同期の管理

- `.gitmodules` は `j5ik2o/aidlc-workflows` を配布元として指定する。`scripts/aidlc-sync.ts:7` にあるとおり、コミットの正本は gitlink（親リポジトリが記録するsubmoduleのコミット参照）であり、同期処理が自動的に版選択やfetchをする構成ではない。
- `scripts/aidlc-sync.ts:268` 以降は配布元の未コミット変更を拒否し、追跡対象の配布ファイルを一時領域へコピーして、`scripts/aidlc-sync/patches/` のパッチを適用する。`installed.json` は各ファイルのSHA-256と実行権限、および保持設定を記録する台帳であり、配布元のコミット記録そのものとは役割が異なる。
- 同期処理には管理対象パス・シンボリックリンク・台帳形式の検査がある。`--apply` の排他と実行前の配布元再確認もある。`ci.yml:23` 以降の配布検査は `bun scripts/aidlc-sync.ts --check` と関連回帰テストを実行する。これを維持し、配布資産への独自修正はパッチへ記録するという草案に同意する。
- 配布検査のcheckoutとBun導入、およびレビュー状態検査の再利用ワークフローはコミットSHA固定である。一方、他のジョブには `actions/checkout@v4`、`dtolnay/rust-toolchain@master` 等も残る。「すべての外部ActionをSHA固定済み」とは記載できない。今回の最短経路とは独立した固定化作業を追加しない。
- `rust-toolchain.toml` のRust版は `1.95.0`。`Cargo.toml` の `event-store-adapter-rs` は `=3.0.0` の完全固定であり、他の依存はmanifestの範囲指定とlockfileを組み合わせる。全依存をmanifestで完全固定しているとは記載しない。

### 未解決事項の扱い

担当範囲から新しい質問は追加しない。ゴールデン（観測結果の比較用データ）の採用版については、草案どおり要求分析で人間の裁定を受ける。配布資産とゴールデンの版差を独断で正規化・許容しない。`cargo audit` の成功要否、TDD、配布再利用、範囲外の検査・配布一般化を追加しないことは依頼原文で決まっており、再質問は不要である。

### 根拠

本節のパスはリポジトリルート相対。依頼原文は `aidlc/spaces/default/intents/260907-selfhost-stage1/project-description.json` をJSON文字列として復号して確認した。規則は `memory/org.md`、`team.md`、`project.md`、`phases/inception.md` と `knowledge/aidlc-shared/coding-rules/README.md` の優先順を適用した。供給経路の主張は `.gitmodules`、`scripts/aidlc-sync.ts`、`scripts/aidlc-sync/installed.json`、`.github/workflows/ci.yml`、`.github/workflows/review-thread-resolution.yml` に基づく。設定ファイルの存在は、実行成功やGitHub側の保護設定を証明しない。

## Positions

- AGREE: CI全ジョブ成功には audit を含める — 利用者が明示した完了条件であり、現在の集約ジョブの対象より広い。
- AGREE: 静的検査の検出範囲と設計規則の射程を分ける — no-public-fields の現実装の境界は規則本文の緩和を意味しない。
- AGREE: 配布元を固定して同期パッチで独自変更を管理する — gitlink・更新台帳・同期検査という既存の管理方法に一致する。
- AGREE: セキュリティの仮想的な懸念から検査や配布一般化を追加しない — 今回は実地スモークに必要な経路と既存品質基準の維持が範囲である。
- AGREE: ゴールデンの版差は人間裁定まで未確定のまま保持する — 観測互換を独断で読み替えないという明示要件に従う。
