//! 統合テスト (e): `append_only` が symlink を拒否する (upstream `appendAuditBlockAtPath`
//! ガード 2/3 — `O_NOFOLLOW` open, 03 §6.7)。infra-io の低水準プリミティブを Gateway 経由の
//! 消費シナリオとして検証する。
#![allow(clippy::unwrap_used)]

use infra_io::append_only::open_append_only;
use std::os::unix::fs::symlink;
use tempfile::tempdir;

#[test]
fn refuses_to_append_through_a_symlinked_audit_shard() {
    let dir = tempdir().unwrap();
    let real_shard = dir.path().join("host-abc123.md");
    std::fs::write(&real_shard, "# AI-DLC Audit Log\n").unwrap();

    let attacker_link = dir.path().join("host-evil.md");
    symlink(&real_shard, &attacker_link).unwrap();

    let result = open_append_only(&attacker_link);
    assert!(result.is_err());

    // symlink でない実体は問題なく追記専用 open できる (対照)。
    assert!(open_append_only(&real_shard).is_ok());
}
