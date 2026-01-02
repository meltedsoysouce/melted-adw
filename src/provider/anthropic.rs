//! Anthropic Claude Code CLI クライアント
//!
//! # 責務
//!
//! - Claude Code CLI (`claude` コマンド) との通信を担当
//! - [`ProviderClient`] トレイトを実装し、統一インターフェースを提供
//! - Claude固有のJSON出力形式と共通型の変換
//!
//! # CLIツール
//!
//! - **コマンド**: `claude`
//! - **インストール**: `npm install -g @anthropic-ai/claude-code`
//! - **認証方法**:
//!   1. 環境変数 `ANTHROPIC_API_KEY` を設定
//!   2. `claude` を起動して `/login` コマンドを実行
//!
//! # CLI出力形式
//!
//! JSON形式 (`--output-format json`):
//! ```json
//! {
//!   "response": "...",
//!   "metadata": {
//!     "model": "claude-sonnet-4-5",
//!     "tokens": {
//!       "input": 100,
//!       "output": 250
//!     }
//!   }
//! }
//! ```
//!
//! # 使用例
//!
//! ```rust,no_run
//! use melted_adw::provider::anthropic::AnthropicClient;
//! use melted_adw::provider::ProviderClient;
//! use melted_adw::config::step::ModelTier;
//!
//! #[tokio::main]
//! async fn main() {
//!     // 事前に環境変数設定またはログインが必要
//!     let client = AnthropicClient::new();
//!
//!     let response = client.execute(
//!         "You are a helpful assistant.",
//!         "Hello!",
//!         &ModelTier::Medium,
//!     ).await.unwrap();
//!
//!     println!("{}", response.content);
//! }
//! ```

use async_trait::async_trait;
use serde::Deserialize;
use tokio::process::Command;

use crate::config::step::{ModelTier, Provider};
use crate::error::ProviderError;
use super::model_tier::resolve_model;
use super::traits::{ProviderClient, ProviderResponse, StopReason, TokenUsage};

/// デフォルトのCLIコマンド名
const DEFAULT_COMMAND: &str = "claude";

/// NPMパッケージ名（エラーメッセージ用）
const NPM_PACKAGE: &str = "@anthropic-ai/claude-code";

/// Anthropic Claude Code CLI クライアント
///
/// Claude Code CLI (`claude` コマンド) を呼び出してLLMと通信します。
/// 認証は環境変数またはCLIツールの事前ログインに依存します。
pub struct AnthropicClient {
    /// 使用するCLIコマンド名（通常は "claude"）
    command: String,
}

impl AnthropicClient {
    /// 新しいAnthropicクライアントを生成
    ///
    /// CLIツール（`claude`）が利用可能である必要があります。
    /// 認証は環境変数またはCLIツールの事前ログインに依存します。
    ///
    /// # 例
    ///
    /// ```rust
    /// use melted_adw::provider::anthropic::AnthropicClient;
    ///
    /// let client = AnthropicClient::new();
    /// ```
    pub fn new() -> Self {
        Self {
            command: DEFAULT_COMMAND.to_string(),
        }
    }

    /// カスタムコマンド名を指定してクライアントを生成
    ///
    /// テストやカスタムインストール時に使用します。
    ///
    /// # 引数
    ///
    /// - `command`: CLIコマンド名（例: "claude-dev"）
    ///
    /// # 例
    ///
    /// ```rust
    /// use melted_adw::provider::anthropic::AnthropicClient;
    ///
    /// let client = AnthropicClient::with_command("claude-dev");
    /// ```
    pub fn with_command(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
        }
    }

    /// CLIツールが利用可能かチェック
    ///
    /// `which` コマンド（Unix系）または `where` コマンド（Windows）を使用して、
    /// CLIツールがインストールされているか確認します。
    ///
    /// # エラー
    ///
    /// - [`ProviderError::CliNotFound`] - CLIツールが見つからない
    async fn check_cli_available(&self) -> Result<(), ProviderError> {
        // Unix系では `which`、Windowsでは `where` を使用
        let check_command = if cfg!(target_os = "windows") {
            "where"
        } else {
            "which"
        };

        let output = Command::new(check_command)
            .arg(&self.command)
            .output()
            .await?;

        if output.status.success() {
            Ok(())
        } else {
            Err(ProviderError::CliNotFound(
                self.command.clone(),
                NPM_PACKAGE.to_string(),
            ))
        }
    }

    /// CLIコマンドを実行してレスポンスを取得
    ///
    /// # 引数
    ///
    /// - `prompt`: 完全なプロンプト（systemプロンプト + ユーザー入力）
    /// - `model`: モデル名
    ///
    /// # エラー
    ///
    /// - [`ProviderError::AuthenticationError`] - 認証エラー
    /// - [`ProviderError::RateLimitExceeded`] - レート制限超過
    /// - [`ProviderError::CliExecutionError`] - CLI実行エラー
    /// - [`ProviderError::InvalidResponse`] - 不正なレスポンス
    async fn execute_cli(
        &self,
        prompt: &str,
        model: &str,
    ) -> Result<ClaudeCliResponse, ProviderError> {
        let output = Command::new(&self.command)
            .arg("-p")
            .arg(prompt)
            .arg("--output-format")
            .arg("json")
            .arg("--model")
            .arg(model)
            .output()
            .await?;

        // 標準エラー出力をチェック（認証エラー等）
        let stderr = String::from_utf8_lossy(&output.stderr);

        // 終了コードが非0の場合はエラー
        if !output.status.success() {
            // 認証エラーを検出
            if stderr.contains("authentication") || stderr.contains("login") || stderr.contains("API key") {
                return Err(ProviderError::AuthenticationError(
                    stderr.to_string(),
                    self.command.clone(),
                ));
            }

            // レート制限を検出
            if stderr.contains("rate limit") || stderr.contains("429") {
                return Err(ProviderError::RateLimitExceeded);
            }

            // その他のエラー
            return Err(ProviderError::CliExecutionError(format!(
                "Command failed with exit code {}: {}",
                output.status.code().unwrap_or(-1),
                stderr
            )));
        }

        // 標準出力をパース
        let stdout = String::from_utf8(output.stdout)?;

        // JSONとしてパース
        let cli_response: ClaudeCliResponse = serde_json::from_str(&stdout)
            .map_err(|e| ProviderError::InvalidResponse(format!(
                "Failed to parse CLI JSON output: {}. Output was: {}",
                e,
                stdout
            )))?;

        Ok(cli_response)
    }
}

impl Default for AnthropicClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProviderClient for AnthropicClient {
    async fn execute(
        &self,
        system_prompt: &str,
        user_input: &str,
        model_tier: &ModelTier,
    ) -> Result<ProviderResponse, ProviderError> {
        // CLIツールの存在確認
        self.check_cli_available().await?;

        // モデル名を解決
        let model = resolve_model(&Provider::Anthropic, model_tier);

        // プロンプトを結合（システムプロンプト + ユーザー入力）
        let full_prompt = format!("{}\n\n{}", system_prompt, user_input);

        // CLIコマンドを実行
        let cli_response = self.execute_cli(&full_prompt, model).await?;

        // CLI形式のレスポンスを共通形式に変換
        Ok(ProviderResponse {
            content: cli_response.response,
            token_usage: TokenUsage {
                input_tokens: cli_response.metadata.tokens.input,
                output_tokens: cli_response.metadata.tokens.output,
            },
            stop_reason: StopReason::EndTurn, // CLIは停止理由を返さないためデフォルト値
            model: cli_response.metadata.model,
        })
    }
}

/// Claude CLI のJSON出力形式
///
/// `claude -p "..." --output-format json` の出力形式を表現します。
#[derive(Debug, Deserialize)]
struct ClaudeCliResponse {
    /// LLMが生成したレスポンステキスト
    response: String,

    /// メタデータ（モデル名、トークン情報等）
    metadata: ClaudeMetadata,
}

/// Claude CLI レスポンスのメタデータ
#[derive(Debug, Deserialize)]
struct ClaudeMetadata {
    /// 使用されたモデル名
    model: String,

    /// トークン使用情報
    tokens: ClaudeTokens,
}

/// Claude CLI のトークン情報
#[derive(Debug, Deserialize)]
struct ClaudeTokens {
    /// 入力トークン数
    input: u32,

    /// 出力トークン数
    output: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let client = AnthropicClient::new();
        assert_eq!(client.command, DEFAULT_COMMAND);
    }

    #[test]
    fn test_with_command() {
        let client = AnthropicClient::with_command("claude-dev");
        assert_eq!(client.command, "claude-dev");
    }

    #[test]
    fn test_default() {
        let client = AnthropicClient::default();
        assert_eq!(client.command, DEFAULT_COMMAND);
    }

    #[test]
    fn test_deserialize_cli_response() {
        let json = r#"{
            "response": "Hello! How can I help you?",
            "metadata": {
                "model": "claude-sonnet-4-5",
                "tokens": {
                    "input": 10,
                    "output": 20
                }
            }
        }"#;

        let response: ClaudeCliResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.response, "Hello! How can I help you?");
        assert_eq!(response.metadata.model, "claude-sonnet-4-5");
        assert_eq!(response.metadata.tokens.input, 10);
        assert_eq!(response.metadata.tokens.output, 20);
    }

    #[tokio::test]
    async fn test_check_cli_not_available() {
        // 存在しないコマンドでテスト
        let client = AnthropicClient::with_command("nonexistent-command-xyz123");
        let result = client.check_cli_available().await;

        assert!(result.is_err());
        match result {
            Err(ProviderError::CliNotFound(cmd, pkg)) => {
                assert_eq!(cmd, "nonexistent-command-xyz123");
                assert_eq!(pkg, NPM_PACKAGE);
            }
            _ => panic!("Expected CliNotFound error"),
        }
    }

    // 実際のCLI呼び出しテストは統合テストで実施
}
