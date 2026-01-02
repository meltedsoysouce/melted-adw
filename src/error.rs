//! エラー型の定義
//!
//! # 責務
//!
//! アプリケーション全体で使用されるエラー型を定義する。
//! - [`ConfigError`] - 設定ファイルの読み込み・パースエラー
//! - [`ProviderError`] - LLMプロバイダー通信エラー（CLI版）

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

/// LLMプロバイダー通信関連のエラー（CLI版）
///
/// CLIツール（`claude`, `codex`）を呼び出す際のエラーを表現します。
#[derive(Debug, Error)]
pub enum ProviderError {
    /// CLIツールが見つからない（未インストール）
    #[error("CLIツールが見つかりません: {0}。インストールしてください: npm install -g {1}")]
    CliNotFound(String, String), // (コマンド名, NPMパッケージ名)

    /// 認証エラー（ログインが必要）
    #[error("認証に失敗しました: {0}。'{1} login' を実行してください")]
    AuthenticationError(String, String), // (エラー詳細, コマンド名)

    /// CLIコマンド実行エラー（終了コードが非0）
    #[error("CLIコマンド実行エラー: {0}")]
    CliExecutionError(String),

    /// 不正なモデルティア指定
    #[error("不正なモデルティア: {0}")]
    InvalidModelTier(String),

    /// レート制限超過
    #[error("レート制限を超えました")]
    RateLimitExceeded,

    /// タイムアウト
    #[error("タイムアウトしました: {0}")]
    Timeout(String),

    /// CLIからの不正なレスポンス（JSONパース失敗等）
    #[error("CLIからの不正なレスポンス: {0}")]
    InvalidResponse(String),

    /// プロセス実行エラー（spawn失敗等）
    #[error("プロセス実行エラー: {0}")]
    ProcessError(#[from] std::io::Error),

    /// JSONパースエラー
    #[error("JSONパースエラー: {0}")]
    JsonError(#[from] serde_json::Error),

    /// UTF-8デコードエラー
    #[error("UTF-8デコードエラー: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
}
