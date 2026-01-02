//! OpenAI Codex CLI クライアント実装
//!
//! # 責務
//!
//! - Codex CLI (`codex` コマンド) との通信を担当
//! - [`ProviderClient`] トレイトを実装し、統一インターフェースを提供
//! - JSONL形式（複数行のJSONイベント）の出力をパース
//! - OpenAI固有のレスポンス形式と共通型の変換
//!
//! # CLIツール
//!
//! - **コマンド**: `codex exec --json --model <model> "prompt"`
//! - **インストール**: `npm install -g @openai/codex`
//! - **認証方法**:
//!   1. 環境変数 `OPENAI_API_KEY` を設定
//!   2. `codex login` コマンドを実行
//!
//! # 出力形式
//!
//! Codex CLIはJSONL（JSON Lines）形式で出力します。
//! 各行が独立したJSONイベントで、複数のイベントタイプがあります：
//!
//! - `turn.started` - LLM実行開始
//! - `item.completed` - 出力アイテム完了（コンテンツを含む）
//! - `turn.completed` - LLM実行完了（トークン使用量を含む）
//!
//! ## 出力例
//!
//! ```json
//! {"type":"turn.started","model":"gpt-4o"}
//! {"type":"item.completed","item":{"type":"text","text":"Hello, world!"}}
//! {"type":"turn.completed","usage":{"input_tokens":10,"output_tokens":5},"stop_reason":"end_turn"}
//! ```
//!
//! # 使用例
//!
//! ```rust,no_run
//! use melted_adw::provider::openai::OpenAIClient;
//! use melted_adw::provider::ProviderClient;
//! use melted_adw::config::step::ModelTier;
//!
//! #[tokio::main]
//! async fn main() {
//!     // 事前に環境変数設定またはログインが必要
//!     let client = OpenAIClient::new();
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

/// Codex CLIのデフォルトコマンド名
const DEFAULT_COMMAND: &str = "codex";

/// Codex CLIのNPMパッケージ名（エラーメッセージ用）
const NPM_PACKAGE: &str = "@openai/codex";

/// OpenAI Codex CLI クライアント
///
/// Codex CLI (`codex exec`) を呼び出してOpenAI LLMと通信します。
///
/// # 認証
///
/// 認証は以下の方法で行われます（CLIツールに委譲）：
/// 1. 環境変数 `OPENAI_API_KEY`
/// 2. `codex login` による事前ログイン
pub struct OpenAIClient {
    /// CLIコマンド名（デフォルト: "codex"）
    command: String,
}

impl OpenAIClient {
    /// 新しいOpenAIクライアントを生成
    ///
    /// CLIツール（`codex`）が利用可能である必要があります。
    /// 認証は環境変数またはCLIツールの事前ログインに依存します。
    pub fn new() -> Self {
        Self {
            command: DEFAULT_COMMAND.to_string(),
        }
    }

    /// カスタムコマンド名でクライアントを生成
    ///
    /// テスト時やカスタムパスの `codex` コマンドを使用する場合に有用です。
    ///
    /// # 引数
    ///
    /// - `command`: CLIコマンド名またはパス（例: "/usr/local/bin/codex"）
    pub fn with_command(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
        }
    }

    /// CLIツールの存在確認
    ///
    /// `codex --version` を実行してツールの存在を確認します。
    ///
    /// # 戻り値
    ///
    /// - `Ok(())` - ツールが利用可能
    /// - `Err(ProviderError)` - ツールが未インストールまたは実行不可
    async fn check_cli_available(&self) -> Result<(), ProviderError> {
        let output = Command::new(&self.command)
            .arg("--version")
            .output()
            .await;

        match output {
            Ok(output) if output.status.success() => Ok(()),
            Ok(_) => Err(ProviderError::CliNotFound(
                self.command.clone(),
                NPM_PACKAGE.to_string(),
            )),
            Err(_) => Err(ProviderError::CliNotFound(
                self.command.clone(),
                NPM_PACKAGE.to_string(),
            )),
        }
    }

    /// JSONL出力をパースしてProviderResponseに変換
    ///
    /// Codex CLIの出力は複数行のJSON（JSONL形式）です。
    /// 各行をパースし、必要な情報を抽出します。
    ///
    /// # 引数
    ///
    /// - `stdout`: Codex CLIの標準出力
    ///
    /// # 戻り値
    ///
    /// - `Ok(ProviderResponse)` - パース成功
    /// - `Err(ProviderError)` - パース失敗または不正な出力
    fn parse_jsonl_output(&self, stdout: &str) -> Result<ProviderResponse, ProviderError> {
        let mut content = String::new();
        let mut model = String::new();
        let mut token_usage = TokenUsage {
            input_tokens: 0,
            output_tokens: 0,
        };
        let mut stop_reason = StopReason::Unknown;

        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let event: JsonLEvent = serde_json::from_str(line)
                .map_err(|e| ProviderError::InvalidResponse(
                    format!("JSONL parse error: {}: {}", e, line)
                ))?;

            match event.event_type.as_str() {
                "turn.started" => {
                    if let Some(m) = event.model {
                        model = m;
                    }
                }
                "item.completed" => {
                    if let Some(item) = event.item
                        && item.item_type == "text"
                        && let Some(text) = item.text
                    {
                        content.push_str(&text);
                    }
                }
                "turn.completed" => {
                    if let Some(usage) = event.usage {
                        token_usage = TokenUsage {
                            input_tokens: usage.input_tokens.unwrap_or(0),
                            output_tokens: usage.output_tokens.unwrap_or(0),
                        };
                    }
                    if let Some(reason) = event.stop_reason {
                        stop_reason = match reason.as_str() {
                            "end_turn" => StopReason::EndTurn,
                            "max_tokens" => StopReason::MaxTokens,
                            "stop_sequence" => StopReason::StopSequence,
                            "content_filter" => StopReason::ContentFilter,
                            _ => StopReason::Unknown,
                        };
                    }
                }
                _ => {
                    // 未知のイベントタイプは無視
                }
            }
        }

        if content.is_empty() {
            return Err(ProviderError::InvalidResponse(
                "No content in response".to_string()
            ));
        }

        Ok(ProviderResponse {
            content,
            token_usage,
            stop_reason,
            model: if model.is_empty() { "unknown".to_string() } else { model },
        })
    }

    /// stderrから認証エラーやレート制限を検出
    ///
    /// # 引数
    ///
    /// - `stderr`: CLIの標準エラー出力
    ///
    /// # 戻り値
    ///
    /// - `Ok(())` - エラーなし
    /// - `Err(ProviderError)` - 検出されたエラー
    fn detect_error_from_stderr(&self, stderr: &str) -> Result<(), ProviderError> {
        let stderr_lower = stderr.to_lowercase();

        if stderr_lower.contains("authentication")
            || stderr_lower.contains("unauthorized")
            || stderr_lower.contains("invalid api key") {
            return Err(ProviderError::AuthenticationError(
                stderr.to_string(),
                self.command.clone(),
            ));
        }

        if stderr_lower.contains("rate limit")
            || stderr_lower.contains("too many requests") {
            return Err(ProviderError::RateLimitExceeded);
        }

        if stderr_lower.contains("timeout") {
            return Err(ProviderError::Timeout(stderr.to_string()));
        }

        if !stderr.trim().is_empty() {
            return Err(ProviderError::CliExecutionError(stderr.to_string()));
        }

        Ok(())
    }
}

impl Default for OpenAIClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProviderClient for OpenAIClient {
    async fn execute(
        &self,
        system_prompt: &str,
        user_input: &str,
        model_tier: &ModelTier,
    ) -> Result<ProviderResponse, ProviderError> {
        // CLIツールの存在確認
        self.check_cli_available().await?;

        // モデル名を解決
        let model = resolve_model(&Provider::OpenAI, model_tier);

        // プロンプトを結合（システムプロンプト + ユーザー入力）
        let combined_prompt = format!("{}\n\n{}", system_prompt, user_input);

        // Codex CLIを実行
        let output = Command::new(&self.command)
            .arg("exec")
            .arg("--json")
            .arg("--model")
            .arg(model)
            .arg(&combined_prompt)
            .output()
            .await?;

        // stderrをチェック
        let stderr = String::from_utf8_lossy(&output.stderr);
        self.detect_error_from_stderr(&stderr)?;

        // 終了コードをチェック
        if !output.status.success() {
            let exit_code = output.status.code().unwrap_or(-1);
            return Err(ProviderError::CliExecutionError(
                format!("codex exited with code {}: {}", exit_code, stderr)
            ));
        }

        // stdoutをパース
        let stdout = String::from_utf8(output.stdout)?;
        self.parse_jsonl_output(&stdout)
    }
}

// JSONL イベント型定義

/// JSONL イベント（全イベントタイプの共通構造）
#[derive(Debug, Deserialize)]
struct JsonLEvent {
    #[serde(rename = "type")]
    event_type: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    item: Option<JsonLItem>,

    #[serde(skip_serializing_if = "Option::is_none")]
    usage: Option<JsonLUsage>,

    #[serde(skip_serializing_if = "Option::is_none")]
    stop_reason: Option<String>,
}

/// JSONL アイテム（item.completed イベント用）
#[derive(Debug, Deserialize)]
struct JsonLItem {
    #[serde(rename = "type")]
    item_type: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
}

/// JSONL 使用量（turn.completed イベント用）
#[derive(Debug, Deserialize)]
struct JsonLUsage {
    #[serde(skip_serializing_if = "Option::is_none")]
    input_tokens: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    output_tokens: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let client = OpenAIClient::new();
        assert_eq!(client.command, "codex");
    }

    #[test]
    fn test_with_command() {
        let client = OpenAIClient::with_command("/custom/path/codex");
        assert_eq!(client.command, "/custom/path/codex");
    }

    #[test]
    fn test_parse_jsonl_output_success() {
        let client = OpenAIClient::new();
        let jsonl = r#"{"type":"turn.started","model":"gpt-4o"}
{"type":"item.completed","item":{"type":"text","text":"Hello, "}}
{"type":"item.completed","item":{"type":"text","text":"world!"}}
{"type":"turn.completed","usage":{"input_tokens":10,"output_tokens":5},"stop_reason":"end_turn"}"#;

        let result = client.parse_jsonl_output(jsonl);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.content, "Hello, world!");
        assert_eq!(response.model, "gpt-4o");
        assert_eq!(response.token_usage.input_tokens, 10);
        assert_eq!(response.token_usage.output_tokens, 5);
        assert_eq!(response.stop_reason, StopReason::EndTurn);
    }

    #[test]
    fn test_parse_jsonl_output_max_tokens() {
        let client = OpenAIClient::new();
        let jsonl = r#"{"type":"turn.started","model":"gpt-4o"}
{"type":"item.completed","item":{"type":"text","text":"Truncated"}}
{"type":"turn.completed","usage":{"input_tokens":5,"output_tokens":100},"stop_reason":"max_tokens"}"#;

        let result = client.parse_jsonl_output(jsonl);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.stop_reason, StopReason::MaxTokens);
    }

    #[test]
    fn test_parse_jsonl_output_empty_content() {
        let client = OpenAIClient::new();
        let jsonl = r#"{"type":"turn.started","model":"gpt-4o"}
{"type":"turn.completed","usage":{"input_tokens":5,"output_tokens":0},"stop_reason":"end_turn"}"#;

        let result = client.parse_jsonl_output(jsonl);
        assert!(result.is_err());
        assert!(matches!(result, Err(ProviderError::InvalidResponse(_))));
    }

    #[test]
    fn test_parse_jsonl_output_invalid_json() {
        let client = OpenAIClient::new();
        let jsonl = "not valid json";

        let result = client.parse_jsonl_output(jsonl);
        assert!(result.is_err());
        assert!(matches!(result, Err(ProviderError::InvalidResponse(_))));
    }

    #[test]
    fn test_detect_error_from_stderr_auth_error() {
        let client = OpenAIClient::new();
        let stderr = "Error: Authentication failed. Invalid API key.";

        let result = client.detect_error_from_stderr(stderr);
        assert!(result.is_err());
        assert!(matches!(result, Err(ProviderError::AuthenticationError(_, _))));
    }

    #[test]
    fn test_detect_error_from_stderr_rate_limit() {
        let client = OpenAIClient::new();
        let stderr = "Error: Rate limit exceeded. Please try again later.";

        let result = client.detect_error_from_stderr(stderr);
        assert!(result.is_err());
        assert!(matches!(result, Err(ProviderError::RateLimitExceeded)));
    }

    #[test]
    fn test_detect_error_from_stderr_timeout() {
        let client = OpenAIClient::new();
        let stderr = "Error: Request timeout after 30 seconds.";

        let result = client.detect_error_from_stderr(stderr);
        assert!(result.is_err());
        assert!(matches!(result, Err(ProviderError::Timeout(_))));
    }

    #[test]
    fn test_detect_error_from_stderr_no_error() {
        let client = OpenAIClient::new();
        let stderr = "";

        let result = client.detect_error_from_stderr(stderr);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_jsonl_multiple_text_items() {
        let client = OpenAIClient::new();
        let jsonl = r#"{"type":"turn.started","model":"gpt-4o"}
{"type":"item.completed","item":{"type":"text","text":"Part 1 "}}
{"type":"item.completed","item":{"type":"text","text":"Part 2 "}}
{"type":"item.completed","item":{"type":"text","text":"Part 3"}}
{"type":"turn.completed","usage":{"input_tokens":10,"output_tokens":15},"stop_reason":"end_turn"}"#;

        let result = client.parse_jsonl_output(jsonl);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.content, "Part 1 Part 2 Part 3");
        assert_eq!(response.token_usage.total(), 25);
    }

    #[test]
    fn test_parse_jsonl_content_filter() {
        let client = OpenAIClient::new();
        let jsonl = r#"{"type":"turn.started","model":"gpt-4o"}
{"type":"item.completed","item":{"type":"text","text":"Filtered content"}}
{"type":"turn.completed","usage":{"input_tokens":5,"output_tokens":2},"stop_reason":"content_filter"}"#;

        let result = client.parse_jsonl_output(jsonl);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.stop_reason, StopReason::ContentFilter);
    }
}
