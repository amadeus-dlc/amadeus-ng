//! `AutonomyMode` — 2 境界型 (10 §2.2、PR #2 レビューで確定)。
//! 状態読取側: 正確な `"autonomous"` のみ autonomous、それ以外 (未設定・空・未知値) は
//! すべて gated の fail-closed **リーダ** (初期化は失敗しない)。
//! CLI 引数側: `AutonomyModeArg` の 2 値厳密パースで、不正値は upstream 逐語で拒否
//! (1 型に畳むと本家の拒否文言が発生不能になる)。

/// 状態フィールド `Construction Autonomy Mode` の 2 値。Bolt バッチごとのゲートを出すか
/// どうかを決める唯一の値。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AutonomyMode {
    /// 無人実行。バッチゲートを提示しない。この昇格だけが human presence を要し (I11)、
    /// 再開する人間がいないため park / recompose は拒否される。
    Autonomous,
    /// 有人実行。バッチゲートを提示する。「付与されていない」側の吸収先でもあり、
    /// 未設定・空・未知値はすべてここへ落ちる (降格に presence は不要)。
    Gated,
}

impl AutonomyMode {
    /// 状態フィールドの値から作る fail-closed な変換 — 「Autonomy is NEVER inferred」の読み側。
    ///
    /// I/O はしない（読み終わった値を写像するだけ）。`from_<源の名前>` は
    /// coding-rules/factory-naming.md の変換の綴りである。
    #[must_use]
    pub fn from_state_field(field_value: Option<&str>) -> AutonomyMode {
        match field_value {
            Some("autonomous") => AutonomyMode::Autonomous,
            _ => AutonomyMode::Gated,
        }
    }

    /// 状態ファイルの `- **Construction Autonomy Mode**:` に書く綴り。
    ///
    /// [`AutonomyMode::from_state_field`] の逆写像である（往復忠実:
    /// `from_state_field(Some(m.as_state_field())) == m`）。読む綴りと書く綴りが割れると、
    /// 一度書いた値を自分で読み戻せなくなる。
    #[must_use]
    pub const fn as_state_field(self) -> &'static str {
        match self {
            AutonomyMode::Autonomous => "autonomous",
            AutonomyMode::Gated => "gated",
        }
    }

    /// 無人実行中か。バッチゲートの省略可否と、park / recompose の拒否ガードの判定に使う。
    #[must_use]
    pub fn is_autonomous(self) -> bool {
        self == AutonomyMode::Autonomous
    }

    /// CLI `--mode` 引数の厳密パース (Controller 規約: 値オブジェクト初期化がバリデーション)。
    ///
    /// 状態読取側の [`AutonomyMode::from_state_field`] とは別の境界である — あちらは
    /// fail-closed で落とし、こちらは拒否の**材料**（与えられた値）を呼出側へ返す。
    ///
    /// # Errors
    ///
    /// 2 値 (`autonomous` / `gated`) 以外は `InvalidModeArg` で拒否する。利用者向けの逐語文言
    /// (`Invalid --mode: …`) は境界（アダプタ層）が材料から組む —
    /// ドメインは文言を運ばない (`coding-rules/error-handling.md`。2026-08-29 是正:
    /// 文言カタログへの依存は「純粋部品だから」を免罪符にした依存方向違反だった)。
    pub fn parse(s: &str) -> Result<AutonomyMode, InvalidModeArg> {
        match s {
            "autonomous" => Ok(AutonomyMode::Autonomous),
            "gated" => Ok(AutonomyMode::Gated),
            other => Err(InvalidModeArg::new(other)),
        }
    }
}

/// CLI `--mode` の 2 値厳密パースの拒否 — **材料のみ**（与えられた不正値）を運ぶ。
/// 利用者向け文言はアダプタ層が組む (2026-08-29 是正 — 従来は完成文言を運んでいた)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidModeArg {
    /// 与えられた不正な `--mode` 値（そのまま）。
    given: String,
}

impl InvalidModeArg {
    /// 拒否された値を包む（整形も加工もしない — 文言はここの責務ではない）。
    #[must_use]
    pub fn new(given: impl Into<String>) -> InvalidModeArg {
        InvalidModeArg {
            given: given.into(),
        }
    }

    /// 与えられた不正な `--mode` 値。
    #[must_use]
    pub fn given(&self) -> &str {
        &self.given
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_reader_is_fail_closed_and_never_fails() {
        assert_eq!(
            AutonomyMode::from_state_field(Some("autonomous")),
            AutonomyMode::Autonomous
        );
        assert_eq!(
            AutonomyMode::from_state_field(Some("Autonomous")),
            AutonomyMode::Gated
        );
        assert_eq!(
            AutonomyMode::from_state_field(Some("")),
            AutonomyMode::Gated
        );
        assert_eq!(
            AutonomyMode::from_state_field(Some("turbo")),
            AutonomyMode::Gated
        );
        assert_eq!(AutonomyMode::from_state_field(None), AutonomyMode::Gated);
    }

    #[test]
    fn cli_arg_parse_is_strict_with_the_verbatim_rejection() {
        assert_eq!(
            AutonomyMode::parse("autonomous"),
            Ok(AutonomyMode::Autonomous)
        );
        assert_eq!(AutonomyMode::parse("gated"), Ok(AutonomyMode::Gated));
        let err = AutonomyMode::parse("turbo").unwrap_err();
        assert_eq!(err.given(), "turbo");
    }
}
