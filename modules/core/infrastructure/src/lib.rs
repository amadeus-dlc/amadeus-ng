//! core 文脈の infrastructure 層 — **言語拡張**。標準ライブラリを汎用に延長する機構だけを
//! 置き、相手方システムの契約 (プロトコル・スキーマ・語彙) は一切知らない
//! (`coding-rules/infrastructure-layer.md`)。
//!
//! 現体現はファイル I/O のプリミティブ 3 種 — アトミックな tmp+rename 書込 (`atomic`)、
//! 追記専用オープン (`append_only`)、fs メタデータ問合せ (`fs_meta`)。
//!
//! **置かないもの**: RPC クライアント・DB アクセス・外部サービス結合。それらは相手方の契約を
//! 知る gateway であり、interface-adapter 層 (Repository 実装・外部システムクライアント) に
//! 属する (`coding-rules/gateway-taxonomy.md`)。
//!
//! 本クレートはポリシーも持たない — 封じ込め検査・W_OK バリア判断などの規律は、それを必要と
//! する層 (Gateway・投影ライタ) が持つ。
//!
//! 依存方向: infrastructure は domain / use-case / interface-adapter を**知らない**。逆は
//! どの層から依存してもよい。

#![forbid(unsafe_code)]

pub mod append_only;
pub mod atomic;
pub mod canon_json;
pub mod codec;
pub mod collections;
pub mod fs_meta;
pub mod secret_file;
