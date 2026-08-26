# 逸脱台帳

> upstream（awslabs/aidlc-workflows v2 @ `3c3146cf`, v2.6.40）の**仕様**（観測可能な契約）からの逸脱を一元管理する。実装レベルの選択（仕様を破らない内部機構の変更）はここには載せず、各仕様書の「実装ノート」節に記録する（`00-policy.md` §2 判定原則）。upstream 追従レビュー（A7）はこの台帳を基準に diff を評価する。

| # | 分類 | upstream の挙動 | amadeus-ng の挙動 | 理由 | 記録 |
| --- | --- | --- | --- | --- | --- |
| 1 | 設計変更 | コマンド起動配線は `bun <dir>/tools/aidlc-<tool>.ts <sub>`（bun による無コンパイル実行）。Markdown 資産・フック設定・文言にこの綴りが焼き込まれる | 同じ語彙をバイナリ呼び出しに写像（ディスパッチャ形 `<executable> <noun> <verb>` ＋マルチコールによる素の `aidlc-<tool>` 形）。写像表は upstream ROUTES 表の写し（ADR 0002 決定 3）。T5 に次ぐ「許可されたテキスト変形第 2 号」として drift guard 対象 | bun ランタイムが存在しないため物理的に不可避（D1）。意味論・文言は維持 | 2026-08-22 / ADR 0002 |
| 2 | バグ修正 | 既知バグ M12: birth が `Construction Autonomy Mode` 行を書かず、`setFieldStrict` を使う set-autonomy が新規 state ファイルで必ず `State update failed: Field not found in state file: "Construction Autonomy Mode". …` で失敗する（upstream 03 の文書化済み discrepancy） | birth で行を書く（または挿入つき書込で修復）。ladder prompt からの autonomous 昇格が新規ワークフローでも成功する | upstream 自身が discrepancy として記録する実装欠陥。ゴールデン互換テストはこの 1 点のみ期待値を分岐 | 2026-08-22 / 10-orchestration §10 |
| 3 | 拡張 | `AIDLC_LOG` 環境変数は存在しない（診断は `AIDLC_HOOK_DEBUG` のフックファイルログのみ） | `AIDLC_LOG`（`RUST_LOG` 互換記法）で stderr の構造化ログを制御する。`AIDLC_HOOK_DEBUG` は互換維持で並存 | AIDLC_* 名前空間への追加。A7 の diff レビューで upstream の同名導入を監視 | 2026-08-22 / ADR 0004 決定 6 |
| 4 | 設計変更 | 状態ファイル `aidlc-state.md`・監査シャードのテキストファイル群が真実源。read-modify-write は mkdir ロック（`<record>/.aidlc-lock/`、owner.json スタンプ、reap）で直列化 | SQLite ジャーナル（~~`journal` / `snapshot` / `checkpoint`~~ → **本家 event-store-adapter-rs v2.0.0 の `journal` / `snapshot` ＋ 我々の `amadeus_projection_checkpoint`**（2026-08-27 改訂 / ADR-010・Bolt B6 — 自前スキーマは全削除）— C6、ストアファイルは `aidlc/spaces/<space>/intents/.aidlc-store.sqlite`）が真実源。遷移は楽観 version で直列化し、ロック dir は生成しない。`aidlc-state.md` / 監査シャードは ReadModelUpdater の投影として**バイト互換**で再生成（リードモデル） | ES 化（ADR-001 / 003 / 004）とロック退役（ADR-007）。観測可能な差は (a) `aidlc/spaces/<space>/intents/.aidlc-store.sqlite`（space 単位 1 ファイル）の追加（既存 `.gitignore` の `aidlc/spaces/*/intents/.aidlc-*` により git 管理外）、(b) ロック dir の非生成。互換ファイルの内容は不変 | 2026-08-23 / ADR-003, ADR-007（NFR1 の逸脱登録。SQLite ファイルのパスは 2026-08-23 Bolt B5 で確定 — `aidlc/spaces/<space>/intents/.aidlc-store.sqlite`）。2026-08-27 / ADR-010（Bolt B6）でストアの実装を本家 event-store-adapter-rs v2.0.0 に置換 — **パス・git 管理外・ロック dir 非生成・互換ファイルの内容不変は変わらず、観測可能な差は増えていない**（表の中身が本家スキーマになっただけで、ストアファイル自体は upstream 非観測） |

## 予約（決定済み・記録待ち）

- インストーラの追加（分類: 拡張 — 2026-08-22 オーナー決定）。upstream は「dist ツリーの再コピー＋新セッション」が唯一のインストール/アップグレード運用（00-overview §6.4）でインストーラを持たない。amadeus-ng は `cargo install` によるバイナリ配布と、バイナリ内蔵の資産インストーラサブコマンドを追加する。インストール後のワークスペース形状は dist コピー結果と一致させ、「新セッションまでスキル/ルールは反映されない」という運用契約は維持する。詳細は A1 の ADR で確定
