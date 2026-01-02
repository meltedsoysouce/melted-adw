//! LLMプロバイダー抽象化レイヤー（CLI版）
//!
//! # 責務
//!
//! - 複数のLLMプロバイダー（Anthropic, OpenAI等）を統一的に扱うインターフェースを提供
//! - プロバイダーの種類に応じた適切なクライアントを生成するファクトリー機能
//! - モデルティア（Heavy/Medium/Light）から実際のモデル名へのマッピング
//!
//! # アーキテクチャ
//!
//! このモジュールは **CLIツール呼び出しベース** で設計されています。
//! APIキーの管理や認証はCLIツールに委譲し、コード内では扱いません。
//!
//! ## 使用するCLIツール
//!
//! - **Anthropic**: `claude` コマンド（Claude Code CLI）
//!   - インストール: `npm install -g @anthropic-ai/claude-code`
//!   - 認証: `claude` を起動し `/login` コマンド、または環境変数 `ANTHROPIC_API_KEY`
//!
//! - **OpenAI**: `codex` コマンド（Codex CLI）
//!   - インストール: `npm install -g @openai/codex`
//!   - 認証: `codex login`、または環境変数 `OPENAI_API_KEY`
//!
//! # モジュール構成
//!
//! - `traits` - 共通インターフェース（[`ProviderClient`]トレイト等）
//! - `model_tier` - モデルティアマッピング
//! - `anthropic` - Anthropic Claude Code CLI クライアント
//! - `openai` - OpenAI Codex CLI クライアント
//!
//! # 使用例
//!
//! ```rust,no_run
//! use melted_adw::provider::{create_provider, ProviderClient};
//! use melted_adw::config::step::{Provider, ModelTier};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // 事前に `claude login` または環境変数設定が必要
//!
//!     // プロバイダークライアントを生成（APIキー不要）
//!     let client = create_provider(&Provider::Anthropic)?;
//!
//!     // LLMを実行
//!     let response = client.execute(
//!         "You are a helpful assistant.",
//!         "Explain Rust ownership in one sentence.",
//!         &ModelTier::Medium,
//!     ).await?;
//!
//!     println!("Response: {}", response.content);
//!     Ok(())
//! }
//! ```

pub mod traits;
pub mod model_tier;
pub mod anthropic;
pub mod openai;

// 公開APIの再エクスポート
pub use traits::{ProviderClient, ProviderResponse, TokenUsage, StopReason};

use crate::config::step::Provider;
use crate::error::ProviderError;

/// プロバイダークライアントを生成するファクトリー関数
///
/// 指定されたプロバイダーの種類に応じて、適切なCLIベースのクライアントを生成します。
///
/// # 認証について
///
/// この関数はAPIキーを引数に取りません。認証は以下の方法でCLIツールに委譲されます：
///
/// ## Anthropic（Claude Code CLI）
///
/// 1. **環境変数**: `ANTHROPIC_API_KEY` が設定されている場合
/// 2. **事前ログイン**: `claude` を起動して `/login` コマンドを実行済みの場合
///
/// ## OpenAI（Codex CLI）
///
/// 1. **環境変数**: `OPENAI_API_KEY` が設定されている場合
/// 2. **事前ログイン**: `codex login` を実行済みの場合
///
/// 認証エラーが発生した場合、[ProviderError::AuthenticationError] が返されます。
///
/// # 引数
///
/// - `provider`: プロバイダーの種類（[Provider::Anthropic] または [Provider::OpenAI]）
///
/// # 戻り値
///
/// - `Ok(Box<dyn ProviderClient>)`: 成功時、プロバイダークライアント
/// - `Err(ProviderError)`: 失敗時、エラー詳細
///
/// # エラー
///
/// - [`ProviderError::CliNotFound`] - CLIツールが未インストール
///
/// # 例
///
/// ```rust,no_run
/// use melted_adw::provider::create_provider;
/// use melted_adw::config::step::Provider;
///
/// // 事前に環境変数設定またはログインが必要:
/// // export ANTHROPIC_API_KEY="sk-ant-..."
/// // または: claude (起動後 /login)
///
/// let client = create_provider(&Provider::Anthropic).unwrap();
/// ```
pub fn create_provider(
    provider: &Provider,
) -> Result<Box<dyn ProviderClient>, ProviderError> {
    match provider {
        Provider::Anthropic => Ok(Box::new(anthropic::AnthropicClient::new())),
        Provider::OpenAI => Ok(Box::new(openai::OpenAIClient::new())),
    }
}
