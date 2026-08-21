//! I/O 低水準基盤 — アトミック書込・プロセス spawn (A4)・テレメトリ配線 (A10)。依存できるのはアダプタ層と composition root のみ (D4)。I/O の責務は Gateways にある。
//!
//! 本クレートはポリシーを持たない — 封じ込め検査・W_OK バリア判断・reap 政策などの規律は
//! `core-interface-adapter` の Gateway が持つ。ここに置くのは低水準プリミティブのみ
//! (11-workspace §4「実 Gateway」の材料)。

#![forbid(unsafe_code)]

pub mod append_only;
pub mod atomic;
pub mod fs_meta;
pub mod process_probe;
