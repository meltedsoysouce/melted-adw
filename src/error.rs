//! エラー型の定義
//!
//! このモジュールは、Melted ADW 全体で使用されるエラー型を定義します。

use thiserror::Error;

/// 設定関連のエラー
#[derive(Debug, Error)]
pub enum ConfigError {
    /// ファイルの読み込みに失敗
    #[error("設定ファイルの読み込みに失敗しました: {0}")]
    FileRead(#[from] std::io::Error),

    /// TOML のデシリアライズに失敗
    #[error("TOML のデシリアライズに失敗しました: {0}")]
    TomlDeserialize(#[from] toml::de::Error),

    /// TOML のシリアライズに失敗
    #[error("TOML のシリアライズに失敗しました: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    /// バリデーションエラー
    #[error("設定のバリデーションに失敗しました: {0}")]
    Validation(String),
}
