//! フック観測が属する、初期化前のspaceまたは既存recordの識別。
use super::{IntentDirName, SpaceName};
/// ルーティングの値。実行集約や絶対パスを保持せず、TS由来のrecordにも対応する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookHealthTarget {
    space: SpaceName,
    record: Option<IntentDirName>,
}
impl HookHealthTarget {
    /// 検証済みのspaceと、存在するときだけrecord識別を受ける完全コンストラクタ。
    #[must_use]
    pub const fn new(space: SpaceName, record: Option<IntentDirName>) -> Self {
        Self { space, record }
    }
    /// 保存された相対領域を検査して復元する。
    /// # Errors
    /// 名前空間、space、recordのいずれかが不正な場合。
    pub fn parse(value: &str) -> Result<Self, super::HookHealthError> {
        let mut parts = value.split('/');
        if parts.next() != Some("spaces") {
            return Err(super::HookHealthError::InvalidIdentity);
        }
        let space = SpaceName::parse(parts.next().unwrap_or_default())
            .map_err(|_| super::HookHealthError::InvalidIdentity)?;
        if parts.next() != Some("intents") {
            return Err(super::HookHealthError::InvalidIdentity);
        }
        let record = parts
            .next()
            .map(|name| {
                IntentDirName::parse(name).map_err(|_| super::HookHealthError::InvalidIdentity)
            })
            .transpose()?;
        if parts.next().is_some() {
            return Err(super::HookHealthError::InvalidIdentity);
        }
        Ok(Self::new(space, record))
    }

    /// ワークスペース根からの観測領域。実際のファイル操作はRMUが行う。
    #[must_use]
    pub fn relative_directory(&self) -> String {
        let directory = format!("spaces/{}/intents", self.space.as_str());
        match &self.record {
            Some(record) => format!("{directory}/{}", record.as_str()),
            None => directory,
        }
    }
    /// 観測が属するspace。
    #[must_use]
    pub const fn space(&self) -> &SpaceName {
        &self.space
    }
    /// 初期化後のrecord識別。初期化前はNone。
    #[must_use]
    pub const fn record(&self) -> Option<&IntentDirName> {
        self.record.as_ref()
    }
}
