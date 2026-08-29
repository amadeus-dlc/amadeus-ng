//! harness 文脈の infrastructure 層 — **言語拡張の受け皿**（憲章のみ。実体は U7 以降）。
//!
//! # 何を置くクレートか
//!
//! harness（Claude Code などの実行ハーネス）側で必要になる**汎用機構**だけを置く
//! (`coding-rules/infrastructure-layer.md`)。判定基準は 1 つ —
//! **その部品は相手方システムの契約を知るか**。知らずに標準ライブラリを汎用に延長するだけ
//! なら infrastructure、知るなら gateway であり interface-adapter 層に属する。
//!
//! | 置く（言語拡張） | 置かない（gateway → interface-adapter） |
//! |---|---|
//! | プロセス起動・stdio 配線の薄いプリミティブ | ハーネスの JSON プロトコルを組み立てる Presenter |
//! | 環境変数・設定の読取機構 | フックの発火条件を知る Controller |
//! | 計測・ロギングの配管 | RPC クライアント・DB アクセス（オーナー明言で禁止） |
//!
//! # なぜ空のまま置くのか
//!
//! `core-infrastructure` と対になる**文脈ごとの infrastructure** という配置規則を、実体が
//! 生まれる前に固定しておくためである（2026-08-29 オーナー裁定）。置き場が無いと、最初に
//! 必要になった機構が `harness-claude`（アダプタ層）へ紛れ込み、後から剥がすことになる。
//!
//! 依存方向: infrastructure は domain / use-case / interface-adapter を**知らない**。逆は
//! どの層から依存してもよい。したがって本クレートの `[dependencies]` は空である。

#![forbid(unsafe_code)]
