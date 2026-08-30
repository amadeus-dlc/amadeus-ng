//! `StatePosition` — 観測されたワークフロー state の位置 (state 束縛の対象)。
//!
//! steering 連鎖の state 束縛 (`h`) が指す**対象そのもの**を名前付きで運ぶ: どの intent の・
//! 何番目まで進んだ歴史の・どの採番版か。素材文字列の連結はここでは組まない — ダイジェスト
//! の計算と直列化は codec (アダプタ層) が持つ。ストア採番の [`StoreVersion`] を含むため
//! ドメインではなくポート入出力 VO としてユースケース層に置く (ADR-010 — 集約は version を
//! 持たない)。

use core_command_domain::orchestration::IntentId;

use super::store_version::StoreVersion;

/// state 束縛の対象 — intent 識別子・集約の通番・ストア採番版の三つ組。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatePosition {
    intent_id: IntentId,
    seq_nr: usize,
    store_version: StoreVersion,
}

impl StatePosition {
    /// 観測した位置を束ねる。
    #[must_use]
    pub const fn new(
        intent_id: IntentId,
        seq_nr: usize,
        store_version: StoreVersion,
    ) -> StatePosition {
        StatePosition {
            intent_id,
            seq_nr,
            store_version,
        }
    }

    /// intent 識別子。
    #[must_use]
    pub const fn intent_id(&self) -> &IntentId {
        &self.intent_id
    }

    /// 集約の通番 (最後に適用したイベントの番号)。
    #[must_use]
    pub const fn seq_nr(&self) -> usize {
        self.seq_nr
    }

    /// ストア採番の楽観 version。
    #[must_use]
    pub const fn store_version(&self) -> StoreVersion {
        self.store_version
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_position_carries_its_three_coordinates() {
        let id = IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").unwrap();
        let position = StatePosition::new(id.clone(), 3, StoreVersion::new(4));
        assert_eq!(position.intent_id(), &id);
        assert_eq!(position.seq_nr(), 3);
        assert_eq!(position.store_version(), StoreVersion::new(4));
    }
}
