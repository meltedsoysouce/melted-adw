//! LLMプロバイダーの共通インターフェース定義
//!
//! # 責務
//!
//! - LLMプロバイダー（Anthropic, OpenAI等）の共通トレイト [`ProviderClient`] を定義
//! - プロバイダー非依存のレスポンス型 [`ProviderResponse`] を提供
//! - トークン使用量 [`TokenUsage`] と停止理由 [`StopReason`] の型を定義
//!
//! # 実装方式
//!
//! このモジュールは **CLIツール呼び出しベース** で設計されています。
//! - Anthropic: `claude` コマンド（Claude Code CLI）
//! - OpenAI: `codex` コマンド（Codex CLI）
//!
//! APIキーの管理はCLIツールに委譲し、コード内では扱いません。
//!
//! # 使用例
//!
//! ```rust,no_run
//! use melted_adw::provider::{ProviderClient, ProviderResponse};
//! use melted_adw::config::step::ModelTier;
//!
//! async fn example(client: Box<dyn ProviderClient>) {
//!     let response = client.execute(
//!         "You are a helpful assistant.",
//!         "Hello!",
//!         &ModelTier::Medium,
//!     ).await.unwrap();
//!
//!     println!("Response: {}", response.content);
//!     println!("Tokens: {} in, {} out",
//!         response.token_usage.input_tokens,
//!         response.token_usage.output_tokens
//!     );
//! }
//! ```

use async_trait::async_trait;
use crate::config::step::ModelTier;
use crate::error::ProviderError;

/// LLMプロバイダーの共通インターフェース
///
/// このトレイトを実装することで、任意のLLMプロバイダーを
/// アプリケーションに統合できます。
///
/// # 実装要件
///
/// - `Send + Sync`: マルチスレッド環境で安全に使用可能
/// - 非同期実行対応（`async_trait`を使用）
///
/// # 実装パターン
///
/// CLIツールを呼び出す場合:
/// ```rust,ignore
/// use tokio::process::Command;
///
/// async fn execute(...) -> Result<ProviderResponse, ProviderError> {
///     let output = Command::new("claude")
///         .arg("-p").arg(user_input)
///         .arg("--output-format").arg("json")
///         .output()
///         .await?;
///
///     // JSONをパースしてProviderResponseに変換
///     // ...
/// }
/// ```
#[async_trait]
pub trait ProviderClient: Send + Sync {
    /// LLMに対してプロンプトを実行し、レスポンスを取得する
    ///
    /// # 引数
    ///
    /// - `system_prompt`: システムプロンプト（LLMの役割・制約を定義）
    /// - `user_input`: ユーザー入力（処理対象のテキスト）
    /// - `model_tier`: モデルティア（Heavy/Medium/Light）
    ///
    /// # 戻り値
    ///
    /// - `Ok(ProviderResponse)`: 成功時、LLMのレスポンス
    /// - `Err(ProviderError)`: 失敗時、エラー詳細
    ///
    /// # エラー
    ///
    /// - [`ProviderError::CliNotFound`] - CLIツールが未インストール
    /// - [`ProviderError::AuthenticationError`] - 認証失敗（ログインが必要）
    /// - [`ProviderError::CliExecutionError`] - CLI実行エラー
    /// - [`ProviderError::RateLimitExceeded`] - レート制限超過
    /// - [`ProviderError::Timeout`] - タイムアウト
    /// - [`ProviderError::InvalidResponse`] - 不正なレスポンス
    async fn execute(
        &self,
        system_prompt: &str,
        user_input: &str,
        model_tier: &ModelTier,
    ) -> Result<ProviderResponse, ProviderError>;
}

/// LLMプロバイダーからのレスポンス
///
/// プロバイダー固有のレスポンス形式（CLI出力）を共通の型に変換したもの。
#[derive(Debug, Clone)]
pub struct ProviderResponse {
    /// LLMが生成したテキスト
    pub content: String,

    /// トークン使用量
    pub token_usage: TokenUsage,

    /// 生成停止理由
    pub stop_reason: StopReason,

    /// 使用されたモデル名（例: "claude-sonnet-4-5", "gpt-4o"）
    pub model: String,
}

/// トークン使用量
#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct TokenUsage {
    /// 入力トークン数（プロンプト）
    pub input_tokens: u32,

    /// 出力トークン数（LLM生成テキスト）
    pub output_tokens: u32,
}

impl TokenUsage {
    /// 総トークン数を計算
    pub fn total(&self) -> u32 {
        self.input_tokens + self.output_tokens
    }
}

/// LLMの生成停止理由
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    /// 自然な終了（LLMが完了を判断）
    EndTurn,

    /// 最大トークン数到達
    MaxTokens,

    /// 停止シーケンス検出
    StopSequence,

    /// コンテンツフィルター発動
    ContentFilter,

    /// 不明な理由
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_usage_total() {
        let usage = TokenUsage {
            input_tokens: 100,
            output_tokens: 250,
        };
        assert_eq!(usage.total(), 350);
    }

    #[test]
    fn test_stop_reason_equality() {
        assert_eq!(StopReason::EndTurn, StopReason::EndTurn);
        assert_ne!(StopReason::EndTurn, StopReason::MaxTokens);
    }
}
