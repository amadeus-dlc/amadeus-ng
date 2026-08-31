//! ワークフロー定義リードモデル (3 入力 + ハーネス identity) の読取失敗。
//!
//! 逐語文言そのものは持たず、**文言を組み立てる材料**を運ぶ (`coding-rules/error-handling.md`
//! — 利用者向けの文言は出す側、すなわちユースケースの `wording` が組む)。旧名 `GraphReadError`
//! は `stage-graph.json` 由来で射程が狭かった (identity と scope identity 群も覆う) ため、
//! ポートへの移設に合わせて改名した (2026-08-31。旧名は残さない —
//! `coding-rules/no-backward-compatibility.md`)。
//!
//! **失敗態度の非対称は 12 §4 のとおり**であり、その非対称自体が観測可能な契約である:
//! グラフと identity は fatal、グリッドの欠損・不正は転置導出へフォールバックするので
//! ここには現れない。

use std::fmt;

/// 3 入力 + identity の読取・parse 失敗。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowDefinitionReadError {
    /// どの定義を読むべきかが決まらない — 呼び手がハーネスの定義を名指しできなかった。
    ///
    /// **読取の失敗ではなく、読取対象の不在**である。実 DAO 実装は構築時に読取対象
    /// (ハーネスの配置) を受け取るので構造的にこれを返さない — 対象を決められなかった
    /// 合成ルートが注入する DAO だけが返す。分類を残すのは、この観測だけが
    /// 「No workflow definition id was provided.」という別の逐語文言へ行くためである
    /// (b26 以前は `NextTurnInput` の `definition_id` 不在が同じ分岐を作っていた)。
    Unidentified,
    /// `stage-graph.json` が読めない (12 §4 #1)。
    ///
    /// `env_override` はパスが `AIDLC_STAGE_GRAPH` 由来かを表す。真のとき逐語文言の hint 節が
    /// 「unset して既定に戻せ」形へ切り替わる — **この分岐自体が観測可能な契約**。
    NotReadable {
        /// 読もうとした `stage-graph.json` の解決済みパス。
        path: String,
        /// 読取が失敗した理由 (OS 由来)。
        cause: String,
        /// `path` が `AIDLC_STAGE_GRAPH` 由来か。
        env_override: bool,
    },
    /// `stage-graph.json` が不正 JSON (12 §4 #2)。`NotReadable` とは別文言。
    InvalidJson {
        /// パースに失敗した `stage-graph.json` の解決済みパス。
        path: String,
        /// パースが失敗した理由 (JSON パーサ由来)。
        cause: String,
    },
    /// scope identity ファイルの読取・frontmatter 検証の失敗 (`name` 欠落・`skeleton` の
    /// 不正値など — 12 §3.3)。
    ScopeFile {
        /// 失敗の詳細。組み立て済みの逐語文言を運ぶ (拒否理由がパスと値に依存するため、
        /// 材料の分解形では組み直せない)。
        message: String,
    },
    /// JSON としては読めたがビュー型へ写せない (未知 `phase`、文法外 `slug` など)。
    ///
    /// upstream はロード時に検証しないが、serde による構造的パースは「ロード時無検証」からの
    /// 逸脱ではなく補強として扱う (12 §10) — dist の正規データに対しては観測差が生じない。
    Malformed {
        /// 写像に失敗した箇所の詳細。`ScopeFile` と同じく組み立て済みの文言を運ぶ。
        message: String,
    },
    /// ハーネス identity ファイル (`harness.json`) を読めない・不正 JSON・`name` が無い
    /// ないし id として不正 (ADR-008)。
    ///
    /// 定義 id の供給元が失われている状態であり、グラフと同じく **fatal**。読取対象が
    /// 名指されている点で [`WorkflowDefinitionReadError::Unidentified`] とは別の観測である。
    HarnessIdentity {
        /// 読もうとした `harness.json` の解決済みパス。
        path: String,
        /// 失敗の理由 (OS / JSON パーサ / id の形式検証のいずれか由来)。
        cause: String,
    },
}

impl fmt::Display for WorkflowDefinitionReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorkflowDefinitionReadError::Unidentified => {
                f.write_str("no workflow definition was named")
            }
            WorkflowDefinitionReadError::NotReadable { path, cause, .. }
            | WorkflowDefinitionReadError::InvalidJson { path, cause }
            | WorkflowDefinitionReadError::HarnessIdentity { path, cause } => {
                write!(f, "{path}: {cause}")
            }
            WorkflowDefinitionReadError::ScopeFile { message }
            | WorkflowDefinitionReadError::Malformed { message } => f.write_str(message),
        }
    }
}

impl std::error::Error for WorkflowDefinitionReadError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_variant_describes_itself_with_its_material() {
        assert_eq!(
            WorkflowDefinitionReadError::Unidentified.to_string(),
            "no workflow definition was named"
        );
        assert_eq!(
            WorkflowDefinitionReadError::NotReadable {
                path: "/d/stage-graph.json".to_string(),
                cause: "ENOENT".to_string(),
                env_override: true,
            }
            .to_string(),
            "/d/stage-graph.json: ENOENT"
        );
        assert_eq!(
            WorkflowDefinitionReadError::InvalidJson {
                path: "/d/stage-graph.json".to_string(),
                cause: "eof".to_string(),
            }
            .to_string(),
            "/d/stage-graph.json: eof"
        );
        assert_eq!(
            WorkflowDefinitionReadError::HarnessIdentity {
                path: "/d/harness.json".to_string(),
                cause: "ENOENT".to_string(),
            }
            .to_string(),
            "/d/harness.json: ENOENT"
        );
        assert_eq!(
            WorkflowDefinitionReadError::ScopeFile {
                message: "Scope file missing frontmatter: /s/aidlc-x.md".to_string(),
            }
            .to_string(),
            "Scope file missing frontmatter: /s/aidlc-x.md"
        );
        assert_eq!(
            WorkflowDefinitionReadError::Malformed {
                message: "unknown phase \"daydreaming\"".to_string(),
            }
            .to_string(),
            "unknown phase \"daydreaming\""
        );
    }
}
