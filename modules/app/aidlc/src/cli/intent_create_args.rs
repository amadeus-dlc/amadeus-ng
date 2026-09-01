//! `intent-create` が運ぶ引数（upstream `aidlc-utility.ts:5989` の usage）。

/// `intent-create` が運ぶ引数（upstream `aidlc-utility.ts:5989` の usage）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IntentCreateArgs {
    scope: Option<String>,
    arguments: Option<String>,
    label: Option<String>,
    depth: Option<String>,
    test_strategy: Option<String>,
    review: Option<String>,
}

impl IntentCreateArgs {
    /// 鋳造する scope（`--scope`）。
    #[must_use]
    pub fn scope(&self) -> Option<&str> {
        self.scope.as_deref()
    }
    /// 自由記述（`--arguments`）。
    #[must_use]
    pub fn arguments(&self) -> Option<&str> {
        self.arguments.as_deref()
    }
    /// 記録ディレクトリ名の短いラベル（`--label`）。
    #[must_use]
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }
    /// 深さの上書き（`--depth`）。
    #[must_use]
    pub fn depth(&self) -> Option<&str> {
        self.depth.as_deref()
    }
    /// テスト戦略の上書き（`--test-strategy`）。
    #[must_use]
    pub fn test_strategy(&self) -> Option<&str> {
        self.test_strategy.as_deref()
    }
    /// レビュー上限の上書き（`--review`）。
    #[must_use]
    pub fn review(&self) -> Option<&str> {
        self.review.as_deref()
    }
}

/// `intent-create` のフラグを [`IntentCreateArgs`] へ畳む。
///
/// `IntentCreateArgs` のフィールドはすべて private なので、フィールド単位で組み立てる
/// 本関数は同じファイル（同じモジュール）に置く（`coding-rules/field-visibility.md`）。
/// `cli::request` の `parse` から呼ばれるため `pub(super)`（`cli` とその子孫まで可視）に上げる。
pub(super) fn parse_intent_create(args: &[String]) -> IntentCreateArgs {
    let mut flags = IntentCreateArgs::default();
    let mut index = 0;
    while let Some(arg) = args.get(index) {
        // upstream は `--arguments=<value>` の等号形も出す (`createPrintDirective`)。
        let (name, inline) = arg
            .split_once('=')
            .map_or((arg.as_str(), None), |(name, value)| {
                (name, Some(value.to_string()))
            });
        let value = inline.clone().or_else(|| args.get(index + 1).cloned());
        let mut took_value = inline.is_none();
        match name {
            "--scope" => flags.scope = value,
            "--arguments" => flags.arguments = value,
            "--label" => flags.label = value,
            "--depth" => flags.depth = value,
            "--test-strategy" => flags.test_strategy = value,
            "--review" => flags.review = value,
            _ => took_value = false,
        }
        if took_value {
            index += 1;
        }
        index += 1;
    }
    flags
}
