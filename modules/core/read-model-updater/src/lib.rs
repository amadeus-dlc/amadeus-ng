//! **コマンド側でもクエリ側でもない中間** — ReadModelUpdater（U4）。
//!
//! RMU の仕事はジャーナル（コマンド側が書いた事実）を読んでリードモデル（`aidlc-state.md` と
//! 監査シャード）を書くことであり、どちらの側にも属さない。したがって RMU は**両側に依存できる
//! 唯一のクレート**である（2026-08-24 原裁定 / 2026-08-29 是正、
//! `coding-rules/cqrs-boundaries.md` 判定表）。リードモデルを**読む**側 — クエリ API の
//! ユースケース層 — はまだ存在せず、それが起きたときは別クレートになる（そちらは
//! `core-command-domain` に**絶対依存しない**）。
//!
//! # 二層構造（2026-08-28 裁定）
//!
//! RMU は 2 つの層でできている。
//!
//! 1. **取得ループ** [`orchestration::ReadModelUpdater`] — チェックポイントを読み、
//!    `events_after` で差分を引き、投影核へ渡し、`advance_checkpoint` で前進する。
//!    ストレージ・接続・チェックポイントを知るのはこちらだけである。
//! 2. **純粋投影核** [`workspace::project`] — ドメインイベントの列とリードモデルだけを
//!    受け取り、リードモデルを作成・更新する。`JournalReader`・SQLite 接続・チェックポイントを
//!    **一切知らない**。
//!
//! 二層を潰してはならない（`coding-rules/cqrs-boundaries.md` 禁止パターン）。投影核が取得の
//! 都合を知ると、投影の規則だけを単体でテストできなくなる。
//!
//! # なぜ依存が `core-command-domain` 1 つで足りるのか
//!
//! 中間であることと両側を広く使ってよいことは別である。投影核の入口はドメインイベント 1 本に
//! 絞ってあり、コマンド側の他の型（集約の再水和・Repository・ストアのエラー）は入口に現れない。
//! したがってコマンド側から要るのはドメインイベントを持つ `core-command-domain` だけで、
//! `core-command-use-case` と `core-command-interface-adapter` は 1 つも要らない。読取側の契約
//! （`JournalReader` / `ProjectionName` / `GlobalSeqNr`）と SQLite 実装は RMU 自身が所有する。
//! 依存の少なさは規律ではなくクレート分離で保つ — 増やせば `Cargo.toml` に現れる。

#![forbid(unsafe_code)]

pub mod orchestration;
pub mod workspace;
