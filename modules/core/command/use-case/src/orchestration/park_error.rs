//! `ParkError` — `ParkUseCase` の失敗。

use std::fmt;

use core_command_domain::orchestration::{CommandError, IntentExecutionId, IntentId};

use super::port::RepositoryError;

/// [`super::ParkUseCase`] の失敗（材料のみ — 逐語文言は出す側が組む）。
///
/// 3 変種すべてが**そのまま伝播させるための封筒**である。park はステージを名指ししないので、
/// [`super::CommitError`] の `UnknownStage` にあたるユースケース自身の失敗を持たない —
/// 判断は集約が、媒体の失敗はポートが所有し、ここは運ぶだけである。
// `Clone` / `PartialEq` は実装しない — `Corrupt` の `source` (原因連鎖) が比較・複製不能で
// ある (裁定 6 で受容済み)。テストは `matches!` で判定する。
#[derive(Debug)]
pub enum ParkError {
    /// 実行の再構成・永続化の失敗（ポートからそのまま伝播）。
    Repository(RepositoryError<IntentExecutionId>),
    /// intent の取得の失敗（ポートからそのまま伝播）。
    IntentRepository(RepositoryError<IntentId>),
    /// 集約がコマンドを拒否した（そのまま伝播 — autonomous / Completed）。
    Command(CommandError),
}

impl fmt::Display for ParkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParkError::Repository(error) => write!(f, "repository: {error}"),
            ParkError::IntentRepository(error) => write!(f, "intent repository: {error}"),
            ParkError::Command(error) => write!(f, "command: {error}"),
        }
    }
}

impl std::error::Error for ParkError {
    /// 内包した失敗へ連鎖する。
    ///
    /// **封筒は連鎖を切ってはならない。** `RepositoryError::Corrupt` は「壊れていた」としか
    /// `Display` に書かず、実材料は `Error::source` の連鎖に載せる（裁定 6）。ここで `None` を
    /// 返すと、その材料はこの型で行き止まりになり、診断には分類だけが残る。
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ParkError::Repository(error) => Some(error),
            ParkError::IntentRepository(error) => Some(error),
            ParkError::Command(error) => Some(error),
        }
    }
}

impl From<RepositoryError<IntentExecutionId>> for ParkError {
    fn from(error: RepositoryError<IntentExecutionId>) -> ParkError {
        ParkError::Repository(error)
    }
}

impl From<RepositoryError<IntentId>> for ParkError {
    fn from(error: RepositoryError<IntentId>) -> ParkError {
        ParkError::IntentRepository(error)
    }
}

impl From<CommandError> for ParkError {
    fn from(error: CommandError) -> ParkError {
        ParkError::Command(error)
    }
}

#[cfg(test)]
mod tests {
    use super::super::park_error::ParkError;
    use super::super::port::RepositoryError;
    use core_command_domain::orchestration::{CommandError, IntentExecutionId, IntentId};

    #[test]
    fn a_repository_failure_is_carried_verbatim() {
        let execution_id =
            IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").expect("UUIDv7");
        let inner = RepositoryError::NotFound {
            id: execution_id.clone(),
        };
        let error = ParkError::from(inner);
        assert!(matches!(
            &error,
            ParkError::Repository(RepositoryError::NotFound { id }) if *id == execution_id
        ));
        assert_eq!(
            error.to_string(),
            format!("repository: not found: {execution_id}")
        );
    }

    #[test]
    fn a_refused_command_is_carried_verbatim() {
        let error = ParkError::from(CommandError::RefusedUnderAutonomy);
        assert!(matches!(
            error,
            ParkError::Command(CommandError::RefusedUnderAutonomy)
        ));
        assert!(error.to_string().starts_with("command: "));
    }

    #[test]
    fn the_envelope_chains_to_the_material_the_port_hid_in_its_source() {
        // `RepositoryError::Corrupt` は分類しか `Display` に書かない (裁定 6) — 実材料は
        // `source` の連鎖に載る。封筒がそこで連鎖を切ると、診断には分類しか残らない。
        let execution_id =
            IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").expect("UUIDv7");
        let error = ParkError::Repository(RepositoryError::Corrupt {
            id: execution_id,
            seq_nr: Some(3),
            source: Box::new(std::io::Error::other("undecodable payload")),
        });

        let port = std::error::Error::source(&error).expect("ポートの失敗へ連鎖する");
        assert_eq!(
            std::error::Error::source(port)
                .expect("ポートは原因へ連鎖する")
                .to_string(),
            "undecodable payload"
        );
    }

    #[test]
    fn the_intent_port_failure_keeps_its_own_face() {
        // 実行は読めたが計画が引けない場合。ID 型が違うので封筒の変種も別である —
        // 言い換えず、その面の失敗のまま運ぶ。
        let intent_id = IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6")
            .expect("フィクスチャの IntentId");
        let error = ParkError::from(RepositoryError::NotFound {
            id: intent_id.clone(),
        });

        assert!(matches!(
            &error,
            ParkError::IntentRepository(RepositoryError::NotFound { id }) if *id == intent_id
        ));
        assert_eq!(
            error.to_string(),
            format!("intent repository: not found: {intent_id}")
        );
        assert_eq!(
            std::error::Error::source(&error)
                .expect("ポートの失敗へ連鎖する")
                .to_string(),
            format!("not found: {intent_id}")
        );
    }

    #[test]
    fn a_refused_command_also_stays_on_the_source_chain() {
        // 集約の拒否も連鎖を切らない — 逐語を選ぶ側が材料そのものへ辿れる。
        let error = ParkError::Command(CommandError::NotRunning);

        assert_eq!(
            std::error::Error::source(&error)
                .expect("集約の拒否へ連鎖する")
                .to_string(),
            CommandError::NotRunning.to_string()
        );
    }

    #[test]
    fn the_failure_is_a_std_error() {
        let error: Box<dyn std::error::Error> =
            Box::new(ParkError::Command(CommandError::NotRunning));
        assert!(error.to_string().starts_with("command: "));
    }
}
