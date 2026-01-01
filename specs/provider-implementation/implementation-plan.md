# Provider モジュール詳細実装計画書

## プロジェクト概要

**タスク名**: provider-implementation
**作成日**: 2026-01-01
**目的**: providerモジュールの詳細実装（各ファイルの責務明確化、データ構造作成、APIクライアント実装）

---

## 1. As-Is / To-Be 分析

### 1.1 As-Is（現状）

#### ファイル構成
- `src/provider.rs` - 空ファイル（モジュール定義のみ）
- `src/provider/traits.rs` - 空ファイル
- `src/provider/model_tier.rs` - 空ファイル
- `src/provider/anthropic.rs` - 空ファイル
- `src/provider/openai.rs` - 空ファイル

#### 既存の関連データ構造
`src/config/step.rs` に以下が定義されている：
```rust
pub enum Provider {
    Anthropic,
    OpenAI,
}

pub enum ModelTier {
    Heavy,   // 複雑な推論タスク
    Medium,  // 一般的なタスク
    Light,   // 簡単なタスク
}
```

#### 依存関係の状態
現在のCargo.tomlには以下が不足：
- `tokio` (非同期ランタイム)
- `reqwest` (HTTPクライアント)
- `async_trait` (非同期トレイト用)

**重要な制約**: CLAUDE.mdにより「原則crateの追加は禁止」→ユーザー許可が必要

### 1.2 To-Be（あるべき姿）

#### モジュール構成と責務

```
provider/
├── traits.rs        - Providerクライアントのトレイト定義
├── model_tier.rs    - ModelTierから具体的モデル名へのマッピング
├── anthropic.rs     - Anthropic APIクライアント実装
└── openai.rs        - OpenAI APIクライアント実装
```

#### 各ファイルの責務

**traits.rs**:
- `ProviderClient` トレイト定義
- `ProviderResponse` 構造体（API応答の統一形式）
- `StopReason` 列挙体（生成停止理由）

**model_tier.rs**:
- `ModelTier` を各プロバイダーの具体的モデル名にマッピングする機能
- プロバイダー固有のモデル定義

**anthropic.rs**:
- `AnthropicClient` 構造体
- Anthropic Messages API との通信実装
- APIキー管理、リクエスト構築、レスポンス解析

**openai.rs**:
- `OpenAIClient` 構造体
- OpenAI Chat Completions API との通信実装
- APIキー管理、リクエスト構築、レスポンス解析

#### 公開API設計

```rust
// traits.rs
#[async_trait]
pub trait ProviderClient {
    async fn execute(
        &self,
        system_prompt: &str,
        user_input: &str,
        model_tier: &ModelTier,
    ) -> Result<ProviderResponse, ProviderError>;
}

pub struct ProviderResponse {
    pub content: String,
    pub tokens_used: TokenUsage,
    pub stop_reason: StopReason,
    pub model_used: String,
}

pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

pub enum StopReason {
    EndTurn,
    MaxTokens,
    StopSequence,
    ToolUse,
}

// model_tier.rs
pub fn resolve_model(provider: &Provider, tier: &ModelTier) -> &'static str;

// anthropic.rs
pub struct AnthropicClient {
    api_key: String,
    http_client: reqwest::Client,
}

impl AnthropicClient {
    pub fn new(api_key: String) -> Result<Self, ProviderError>;
}

// openai.rs
pub struct OpenAIClient {
    api_key: String,
    http_client: reqwest::Client,
}

impl OpenAIClient {
    pub fn new(api_key: String) -> Result<Self, ProviderError>;
}

// provider.rs
pub fn create_provider(
    provider: &Provider,
    api_key: String,
) -> Result<Box<dyn ProviderClient>, ProviderError>;
```

#### エラー型の追加

`src/error.rs` に以下を追加：
```rust
#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("認証に失敗しました: {0}")]
    AuthenticationError(String),

    #[error("APIリクエストに失敗しました: {0}")]
    ApiError(String),

    #[error("不正なモデルティア: {0}")]
    InvalidModelTier(String),

    #[error("レート制限を超えました")]
    RateLimitExceeded,

    #[error("タイムアウトしました: {0}")]
    Timeout(String),

    #[error("プロバイダーからの不正なレスポンス: {0}")]
    InvalidResponse(String),

    #[error("HTTPリクエストエラー: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("JSONパースエラー: {0}")]
    JsonError(#[from] serde_json::Error),
}
```

---

## 2. ギャップ分析

### 2.1 実装が必要な項目

| カテゴリ | 項目 | 現状 | 必要な作業 |
|---------|------|------|-----------|
| **データ構造** | Provider/ModelTier列挙体 | config/step.rsに存在 | providerモジュールに移動・再エクスポート |
| **エラー型** | ProviderError | なし | error.rsに追加 |
| **トレイト** | ProviderClient | なし | traits.rsに実装 |
| **レスポンス型** | ProviderResponse, TokenUsage, StopReason | なし | traits.rsに実装 |
| **モデルマッピング** | ModelTier→モデル名変換 | なし | model_tier.rsに実装 |
| **Anthropicクライアント** | AnthropicClient構造体 | なし | anthropic.rsに実装 |
| **OpenAIクライアント** | OpenAIClient構造体 | なし | openai.rsに実装 |
| **ファクトリー** | create_provider関数 | なし | provider.rsに実装 |
| **依存関係** | tokio, reqwest, async_trait | なし | Cargo.tomlに追加（要許可） |

### 2.2 変更が必要なファイル

| ファイル | 変更内容 | 影響範囲 |
|---------|---------|---------|
| `src/error.rs` | `ProviderError`追加 | 新規追加のみ（既存に影響なし） |
| `src/config/step.rs` | `Provider`/`ModelTier`の再エクスポート化 | 既存の`use`文の更新が必要 |
| `src/provider.rs` | モジュール定義、ファクトリー関数 | 新規実装 |
| `src/provider/traits.rs` | トレイトと共通型の定義 | 新規実装 |
| `src/provider/model_tier.rs` | モデルマッピングロジック | 新規実装 |
| `src/provider/anthropic.rs` | Anthropicクライアント実装 | 新規実装 |
| `src/provider/openai.rs` | OpenAIクライアント実装 | 新規実装 |
| `Cargo.toml` | 非同期・HTTP依存関係追加 | **要ユーザー許可** |

---

## 3. 実装計画（フェーズ分割）

### フェーズ0: 事前準備（依存関係の追加）

**目的**: 必要なクレートを追加

**作業内容**:
1. ユーザーに依存関係追加の許可を取得
2. Cargo.tomlに以下を追加:
   ```toml
   tokio = { version = "1", features = ["full"] }
   reqwest = { version = "0.11", features = ["json"] }
   async-trait = "0.1"
   serde_json = "1.0"
   ```
3. `cargo check`で依存関係を確認

**成果物**:
- 更新されたCargo.toml
- ビルドが通ることの確認

**担当エージェント**: メインエージェント

---

### フェーズ1: エラー型とトレイト定義

**目的**: 基盤となる型定義とトレイトを実装

**対象ファイル**:
- `src/error.rs`
- `src/provider.rs`
- `src/provider/traits.rs`

**作業内容**:

#### 1.1 エラー型の追加 (`src/error.rs`)

```rust
/// プロバイダー関連のエラー
#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("認証に失敗しました: {0}")]
    AuthenticationError(String),

    #[error("APIリクエストに失敗しました: {0}")]
    ApiError(String),

    #[error("不正なモデルティア: {0}")]
    InvalidModelTier(String),

    #[error("レート制限を超えました")]
    RateLimitExceeded,

    #[error("タイムアウトしました: {0}")]
    Timeout(String),

    #[error("プロバイダーからの不正なレスポンス: {0}")]
    InvalidResponse(String),

    #[error("HTTPリクエストエラー: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("JSONパースエラー: {0}")]
    JsonError(#[from] serde_json::Error),
}
```

#### 1.2 トレイト定義 (`src/provider/traits.rs`)

```rust
//! Provider クライアントのトレイト定義
//!
//! # 責務
//!
//! このモジュールは、複数のAIプロバイダー（Anthropic、OpenAI等）を
//! 統一的に扱うための抽象化レイヤーを提供します。
//!
//! ## 主な機能
//!
//! - **統一インターフェース**: `ProviderClient` トレイトによる抽象化
//! - **レスポンス型**: プロバイダー固有のレスポンスを共通型に変換
//! - **トークン情報**: 入力・出力トークン数の追跡
//!
//! ## 使用例
//!
//! ```rust,ignore
//! let client: Box<dyn ProviderClient> = create_provider(&Provider::Anthropic, api_key)?;
//! let response = client.execute(
//!     "あなたは優秀なエンジニアです",
//!     "Rustでハローワールドを書いて",
//!     &ModelTier::Medium
//! ).await?;
//! println!("Response: {}", response.content);
//! println!("Tokens used: {}", response.tokens_used.total());
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::config::step::{ModelTier, Provider};
use crate::error::ProviderError;

/// AIプロバイダークライアントのトレイト
///
/// 各プロバイダー（Anthropic、OpenAI等）の実装がこのトレイトを実装することで、
/// 統一的なインターフェースでAI APIを呼び出せるようにします。
#[async_trait]
pub trait ProviderClient: Send + Sync {
    /// プロンプトを実行してレスポンスを取得
    ///
    /// # 引数
    ///
    /// * `system_prompt` - システムプロンプト（AIの役割定義）
    /// * `user_input` - ユーザー入力（実際のタスク指示）
    /// * `model_tier` - 使用するモデルのティア（Heavy/Medium/Light）
    ///
    /// # 戻り値
    ///
    /// * `Ok(ProviderResponse)` - 成功時、AIのレスポンス
    /// * `Err(ProviderError)` - 失敗時、エラー詳細
    ///
    /// # エラー
    ///
    /// - `ProviderError::AuthenticationError` - APIキーが無効
    /// - `ProviderError::ApiError` - API呼び出しに失敗
    /// - `ProviderError::Timeout` - タイムアウト
    /// - `ProviderError::RateLimitExceeded` - レート制限超過
    async fn execute(
        &self,
        system_prompt: &str,
        user_input: &str,
        model_tier: &ModelTier,
    ) -> Result<ProviderResponse, ProviderError>;

    /// プロバイダー名を取得
    fn provider_name(&self) -> &str;
}

/// プロバイダーからのレスポンス（共通形式）
///
/// 各プロバイダー固有のAPIレスポンスを、この共通形式に変換して扱います。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderResponse {
    /// 生成されたテキストコンテンツ
    pub content: String,

    /// 使用されたトークン数
    pub tokens_used: TokenUsage,

    /// 生成停止の理由
    pub stop_reason: StopReason,

    /// 実際に使用されたモデル名（例: "claude-sonnet-4", "gpt-4o"）
    pub model_used: String,
}

/// トークン使用量
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TokenUsage {
    /// 入力トークン数
    pub input_tokens: u32,

    /// 出力トークン数
    pub output_tokens: u32,
}

impl TokenUsage {
    /// 合計トークン数を取得
    pub fn total(&self) -> u32 {
        self.input_tokens + self.output_tokens
    }
}

/// 生成停止の理由
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// 自然な終了（モデルが完了と判断）
    EndTurn,

    /// 最大トークン数に到達
    MaxTokens,

    /// ストップシーケンスに到達
    StopSequence,

    /// ツール使用（関数呼び出し等）
    ToolUse,
}
```

#### 1.3 モジュール定義 (`src/provider.rs`)

```rust
//! AIプロバイダー抽象化レイヤー
//!
//! # 責務
//!
//! このモジュールは、複数のAIプロバイダー（Anthropic、OpenAI等）を
//! 統一的に扱うための機能を提供します。
//!
//! ## 主な機能
//!
//! - **プロバイダー抽象化**: `ProviderClient` トレイトによる統一API
//! - **モデルティアマッピング**: Heavy/Medium/Light を具体的なモデル名に変換
//! - **マルチプロバイダー対応**: Anthropic、OpenAI を透過的に扱う
//!
//! ## アーキテクチャ
//!
//! ```text
//! WorkflowStep (Provider + ModelTier)
//!      ↓
//! create_provider() - ファクトリー関数
//!      ↓
//! Box<dyn ProviderClient>
//!      ↓
//! AnthropicClient or OpenAIClient
//!      ↓
//! execute() - API呼び出し
//!      ↓
//! ProviderResponse
//! ```
//!
//! ## 使用例
//!
//! ```rust,ignore
//! use melted_adw::provider::create_provider;
//! use melted_adw::config::step::{Provider, ModelTier};
//!
//! let api_key = std::env::var("ANTHROPIC_API_KEY")?;
//! let client = create_provider(&Provider::Anthropic, api_key)?;
//!
//! let response = client.execute(
//!     "あなたは優秀なアシスタントです",
//!     "こんにちは",
//!     &ModelTier::Medium
//! ).await?;
//!
//! println!("Response: {}", response.content);
//! ```
//!
//! ## 関連モジュール
//!
//! - [`traits`]: `ProviderClient` トレイト定義
//! - [`model_tier`]: モデルティアのマッピングロジック
//! - [`anthropic`]: Anthropic API実装
//! - [`openai`]: OpenAI API実装

pub mod traits;
pub mod model_tier;
pub mod anthropic;
pub mod openai;

pub use traits::{ProviderClient, ProviderResponse, TokenUsage, StopReason};

use crate::config::step::Provider;
use crate::error::ProviderError;

/// プロバイダークライアントを生成するファクトリー関数
///
/// 指定されたプロバイダーに対応するクライアント実装を生成します。
///
/// # 引数
///
/// * `provider` - プロバイダーの種類（Anthropic または OpenAI）
/// * `api_key` - APIキー
///
/// # 戻り値
///
/// * `Ok(Box<dyn ProviderClient>)` - 生成されたクライアント
/// * `Err(ProviderError)` - 生成に失敗した場合（例: APIキーが無効）
///
/// # 使用例
///
/// ```rust,ignore
/// let api_key = std::env::var("ANTHROPIC_API_KEY")?;
/// let client = create_provider(&Provider::Anthropic, api_key)?;
/// ```
pub fn create_provider(
    provider: &Provider,
    api_key: String,
) -> Result<Box<dyn ProviderClient>, ProviderError> {
    match provider {
        Provider::Anthropic => {
            let client = anthropic::AnthropicClient::new(api_key)?;
            Ok(Box::new(client))
        }
        Provider::OpenAI => {
            let client = openai::OpenAIClient::new(api_key)?;
            Ok(Box::new(client))
        }
    }
}
```

**テスト要件**:
- ProviderError の各バリアントが正しく動作すること
- TokenUsage::total() が正しく計算されること

**成果物**:
- `ProviderError` 定義
- `ProviderClient` トレイト
- `ProviderResponse`, `TokenUsage`, `StopReason` 型
- モジュール定義とファクトリー関数

**推定行数**: 200-250行

**担当エージェント**: 実装エージェント1

---

### フェーズ2: モデルティアマッピング

**目的**: ModelTierから具体的なモデル名への変換ロジック実装

**対象ファイル**:
- `src/provider/model_tier.rs`

**作業内容**:

#### 2.1 モデルマッピングロジック (`src/provider/model_tier.rs`)

```rust
//! モデルティアマッピング
//!
//! # 責務
//!
//! このモジュールは、抽象的なモデルティア（Heavy/Medium/Light）を
//! 各プロバイダーの具体的なモデル名にマッピングする機能を提供します。
//!
//! ## 主な機能
//!
//! - **モデル解決**: `ModelTier` + `Provider` → 具体的なモデル名
//! - **一元管理**: モデル名の変更を一箇所で管理
//! - **拡張性**: 新しいモデルの追加が容易
//!
//! ## モデルマッピング表
//!
//! | Tier   | Anthropic          | OpenAI          |
//! |--------|--------------------|-----------------|
//! | Heavy  | claude-opus-4      | o1              |
//! | Medium | claude-sonnet-4    | gpt-4o          |
//! | Light  | claude-haiku-3-5   | gpt-4o-mini     |
//!
//! ## 使用例
//!
//! ```rust,ignore
//! use melted_adw::provider::model_tier::resolve_model;
//! use melted_adw::config::step::{Provider, ModelTier};
//!
//! let model_name = resolve_model(&Provider::Anthropic, &ModelTier::Heavy);
//! assert_eq!(model_name, "claude-opus-4");
//! ```

use crate::config::step::{ModelTier, Provider};

/// モデルティアを具体的なモデル名に解決
///
/// # 引数
///
/// * `provider` - プロバイダー（Anthropic or OpenAI）
/// * `tier` - モデルティア（Heavy/Medium/Light）
///
/// # 戻り値
///
/// 具体的なモデル名（例: "claude-sonnet-4", "gpt-4o"）
///
/// # 使用例
///
/// ```rust,ignore
/// let model = resolve_model(&Provider::Anthropic, &ModelTier::Medium);
/// assert_eq!(model, "claude-sonnet-4");
/// ```
pub fn resolve_model(provider: &Provider, tier: &ModelTier) -> &'static str {
    match (provider, tier) {
        // Anthropic のモデルマッピング
        (Provider::Anthropic, ModelTier::Heavy) => ANTHROPIC_HEAVY,
        (Provider::Anthropic, ModelTier::Medium) => ANTHROPIC_MEDIUM,
        (Provider::Anthropic, ModelTier::Light) => ANTHROPIC_LIGHT,

        // OpenAI のモデルマッピング
        (Provider::OpenAI, ModelTier::Heavy) => OPENAI_HEAVY,
        (Provider::OpenAI, ModelTier::Medium) => OPENAI_MEDIUM,
        (Provider::OpenAI, ModelTier::Light) => OPENAI_LIGHT,
    }
}

// Anthropic モデル定義
const ANTHROPIC_HEAVY: &str = "claude-opus-4";
const ANTHROPIC_MEDIUM: &str = "claude-sonnet-4";
const ANTHROPIC_LIGHT: &str = "claude-haiku-3-5";

// OpenAI モデル定義
const OPENAI_HEAVY: &str = "o1";
const OPENAI_MEDIUM: &str = "gpt-4o";
const OPENAI_LIGHT: &str = "gpt-4o-mini";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anthropic_model_mapping() {
        assert_eq!(
            resolve_model(&Provider::Anthropic, &ModelTier::Heavy),
            "claude-opus-4"
        );
        assert_eq!(
            resolve_model(&Provider::Anthropic, &ModelTier::Medium),
            "claude-sonnet-4"
        );
        assert_eq!(
            resolve_model(&Provider::Anthropic, &ModelTier::Light),
            "claude-haiku-3-5"
        );
    }

    #[test]
    fn test_openai_model_mapping() {
        assert_eq!(
            resolve_model(&Provider::OpenAI, &ModelTier::Heavy),
            "o1"
        );
        assert_eq!(
            resolve_model(&Provider::OpenAI, &ModelTier::Medium),
            "gpt-4o"
        );
        assert_eq!(
            resolve_model(&Provider::OpenAI, &ModelTier::Light),
            "gpt-4o-mini"
        );
    }

    #[test]
    fn test_all_combinations() {
        // すべての組み合わせが正しくマッピングされることを確認
        let providers = [Provider::Anthropic, Provider::OpenAI];
        let tiers = [ModelTier::Heavy, ModelTier::Medium, ModelTier::Light];

        for provider in &providers {
            for tier in &tiers {
                let model = resolve_model(provider, tier);
                assert!(!model.is_empty(), "Model name should not be empty");
            }
        }
    }
}
```

**テスト要件**:
- すべてのプロバイダー × ティアの組み合わせが正しくマッピングされること
- モデル名が空でないこと

**成果物**:
- `resolve_model` 関数
- モデル定義定数
- 包括的なテスト

**推定行数**: 100-120行

**担当エージェント**: 実装エージェント1（フェーズ1と同じエージェント）

---

### フェーズ3: Anthropic クライアント実装

**目的**: Anthropic Messages API クライアントの実装

**対象ファイル**:
- `src/provider/anthropic.rs`

**作業内容**:

#### 3.1 クライアント構造体とAPI実装

```rust
//! Anthropic API クライアント実装
//!
//! # 責務
//!
//! このモジュールは、Anthropic（Claude）のMessages APIとの通信を担当します。
//!
//! ## 主な機能
//!
//! - **API通信**: Anthropic Messages API への HTTP リクエスト送信
//! - **認証管理**: APIキーによる認証
//! - **レスポンス変換**: Anthropic固有のレスポンスを共通形式に変換
//! - **エラーハンドリング**: API エラーの適切な処理
//!
//! ## API仕様
//!
//! - エンドポイント: `https://api.anthropic.com/v1/messages`
//! - 認証: `x-api-key` ヘッダー
//! - APIバージョン: `2023-06-01`
//!
//! ## 使用例
//!
//! ```rust,ignore
//! let client = AnthropicClient::new("sk-ant-...".to_string())?;
//! let response = client.execute(
//!     "あなたは優秀なエンジニアです",
//!     "Rustでハローワールドを書いて",
//!     &ModelTier::Medium
//! ).await?;
//! ```

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::config::step::ModelTier;
use crate::error::ProviderError;
use super::model_tier::resolve_model;
use super::traits::{ProviderClient, ProviderResponse, TokenUsage, StopReason};
use crate::config::step::Provider;

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_API_VERSION: &str = "2023-06-01";

/// Anthropic APIクライアント
pub struct AnthropicClient {
    api_key: String,
    http_client: Client,
}

impl AnthropicClient {
    /// 新しいAnthropicクライアントを生成
    ///
    /// # 引数
    ///
    /// * `api_key` - Anthropic APIキー
    ///
    /// # 戻り値
    ///
    /// * `Ok(AnthropicClient)` - 生成されたクライアント
    /// * `Err(ProviderError)` - APIキーが無効な場合
    pub fn new(api_key: String) -> Result<Self, ProviderError> {
        if api_key.trim().is_empty() {
            return Err(ProviderError::AuthenticationError(
                "APIキーが空です".to_string()
            ));
        }

        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .map_err(|e| ProviderError::HttpError(e))?;

        Ok(Self {
            api_key,
            http_client,
        })
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
        let model = resolve_model(&Provider::Anthropic, model_tier);

        let request_body = AnthropicRequest {
            model: model.to_string(),
            max_tokens: 4096,
            system: system_prompt.to_string(),
            messages: vec![
                Message {
                    role: "user".to_string(),
                    content: user_input.to_string(),
                }
            ],
        };

        let response = self.http_client
            .post(ANTHROPIC_API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_API_VERSION)
            .header("content-type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| ProviderError::HttpError(e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();

            return Err(match status.as_u16() {
                401 => ProviderError::AuthenticationError(error_text),
                429 => ProviderError::RateLimitExceeded,
                _ => ProviderError::ApiError(format!("HTTP {}: {}", status, error_text)),
            });
        }

        let api_response: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;

        // Anthropic固有のレスポンスを共通形式に変換
        Ok(ProviderResponse {
            content: api_response.content
                .first()
                .map(|c| c.text.clone())
                .unwrap_or_default(),
            tokens_used: TokenUsage {
                input_tokens: api_response.usage.input_tokens,
                output_tokens: api_response.usage.output_tokens,
            },
            stop_reason: match api_response.stop_reason.as_str() {
                "end_turn" => StopReason::EndTurn,
                "max_tokens" => StopReason::MaxTokens,
                "stop_sequence" => StopReason::StopSequence,
                _ => StopReason::EndTurn,
            },
            model_used: api_response.model,
        })
    }

    fn provider_name(&self) -> &str {
        "anthropic"
    }
}

// Anthropic API リクエスト型
#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<Message>,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

// Anthropic API レスポンス型
#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    id: String,
    model: String,
    content: Vec<ContentBlock>,
    stop_reason: String,
    usage: Usage,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

#[derive(Debug, Deserialize)]
struct Usage {
    input_tokens: u32,
    output_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_with_valid_api_key() {
        let result = AnthropicClient::new("sk-ant-test123".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_new_with_empty_api_key() {
        let result = AnthropicClient::new("".to_string());
        assert!(result.is_err());
        match result {
            Err(ProviderError::AuthenticationError(_)) => {},
            _ => panic!("Expected AuthenticationError"),
        }
    }

    #[test]
    fn test_provider_name() {
        let client = AnthropicClient::new("test-key".to_string()).unwrap();
        assert_eq!(client.provider_name(), "anthropic");
    }

    // 実際のAPI呼び出しのテストは統合テストで実施
}
```

**テスト要件**:
- APIキーのバリデーション
- provider_name()の正しさ
- （統合テスト）実際のAPI呼び出し（モック使用）

**成果物**:
- `AnthropicClient` 構造体
- `ProviderClient` トレイト実装
- API リクエスト/レスポンス型
- 単体テスト

**推定行数**: 300-350行

**担当エージェント**: 実装エージェント2

---

### フェーズ4: OpenAI クライアント実装

**目的**: OpenAI Chat Completions API クライアントの実装

**対象ファイル**:
- `src/provider/openai.rs`

**作業内容**:

#### 4.1 クライアント構造体とAPI実装

```rust
//! OpenAI API クライアント実装
//!
//! # 責務
//!
//! このモジュールは、OpenAI（GPT）のChat Completions APIとの通信を担当します。
//!
//! ## 主な機能
//!
//! - **API通信**: OpenAI Chat Completions API への HTTP リクエスト送信
//! - **認証管理**: APIキーによる認証
//! - **レスポンス変換**: OpenAI固有のレスポンスを共通形式に変換
//! - **エラーハンドリング**: API エラーの適切な処理
//!
//! ## API仕様
//!
//! - エンドポイント: `https://api.openai.com/v1/chat/completions`
//! - 認証: `Authorization: Bearer <api_key>` ヘッダー
//!
//! ## 使用例
//!
//! ```rust,ignore
//! let client = OpenAIClient::new("sk-...".to_string())?;
//! let response = client.execute(
//!     "あなたは優秀なエンジニアです",
//!     "Rustでハローワールドを書いて",
//!     &ModelTier::Medium
//! ).await?;
//! ```

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::config::step::ModelTier;
use crate::error::ProviderError;
use super::model_tier::resolve_model;
use super::traits::{ProviderClient, ProviderResponse, TokenUsage, StopReason};
use crate::config::step::Provider;

const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";

/// OpenAI APIクライアント
pub struct OpenAIClient {
    api_key: String,
    http_client: Client,
}

impl OpenAIClient {
    /// 新しいOpenAIクライアントを生成
    ///
    /// # 引数
    ///
    /// * `api_key` - OpenAI APIキー
    ///
    /// # 戻り値
    ///
    /// * `Ok(OpenAIClient)` - 生成されたクライアント
    /// * `Err(ProviderError)` - APIキーが無効な場合
    pub fn new(api_key: String) -> Result<Self, ProviderError> {
        if api_key.trim().is_empty() {
            return Err(ProviderError::AuthenticationError(
                "APIキーが空です".to_string()
            ));
        }

        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .map_err(|e| ProviderError::HttpError(e))?;

        Ok(Self {
            api_key,
            http_client,
        })
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
        let model = resolve_model(&Provider::OpenAI, model_tier);

        let request_body = OpenAIRequest {
            model: model.to_string(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: user_input.to_string(),
                },
            ],
            max_tokens: Some(4096),
        };

        let response = self.http_client
            .post(OPENAI_API_URL)
            .header("Authorization", format!("Bearer {}", &self.api_key))
            .header("content-type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| ProviderError::HttpError(e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();

            return Err(match status.as_u16() {
                401 => ProviderError::AuthenticationError(error_text),
                429 => ProviderError::RateLimitExceeded,
                _ => ProviderError::ApiError(format!("HTTP {}: {}", status, error_text)),
            });
        }

        let api_response: OpenAIResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;

        // OpenAI固有のレスポンスを共通形式に変換
        let choice = api_response.choices
            .first()
            .ok_or_else(|| ProviderError::InvalidResponse(
                "レスポンスにchoicesが含まれていません".to_string()
            ))?;

        Ok(ProviderResponse {
            content: choice.message.content.clone(),
            tokens_used: TokenUsage {
                input_tokens: api_response.usage.prompt_tokens,
                output_tokens: api_response.usage.completion_tokens,
            },
            stop_reason: match choice.finish_reason.as_str() {
                "stop" => StopReason::EndTurn,
                "length" => StopReason::MaxTokens,
                "tool_calls" => StopReason::ToolUse,
                _ => StopReason::EndTurn,
            },
            model_used: api_response.model,
        })
    }

    fn provider_name(&self) -> &str {
        "openai"
    }
}

// OpenAI API リクエスト型
#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: Option<u32>,
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

// OpenAI API レスポンス型
#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    id: String,
    model: String,
    choices: Vec<Choice>,
    usage: OpenAIUsage,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ChatMessage,
    finish_reason: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_with_valid_api_key() {
        let result = OpenAIClient::new("sk-test123".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_new_with_empty_api_key() {
        let result = OpenAIClient::new("".to_string());
        assert!(result.is_err());
        match result {
            Err(ProviderError::AuthenticationError(_)) => {},
            _ => panic!("Expected AuthenticationError"),
        }
    }

    #[test]
    fn test_provider_name() {
        let client = OpenAIClient::new("test-key".to_string()).unwrap();
        assert_eq!(client.provider_name(), "openai");
    }

    // 実際のAPI呼び出しのテストは統合テストで実施
}
```

**テスト要件**:
- APIキーのバリデーション
- provider_name()の正しさ
- （統合テスト）実際のAPI呼び出し（モック使用）

**成果物**:
- `OpenAIClient` 構造体
- `ProviderClient` トレイト実装
- API リクエスト/レスポンス型
- 単体テスト

**推定行数**: 300-350行

**担当エージェント**: 実装エージェント3

---

### フェーズ5: 統合とテスト

**目的**: 全体統合、エクスポート調整、包括的なテスト

**対象ファイル**:
- `src/provider.rs`
- `src/lib.rs`
- `src/config/step.rs`

**作業内容**:

#### 5.1 Provider/ModelTierの再エクスポート調整

現在、`Provider` と `ModelTier` は `config::step` に定義されているが、
論理的には `provider` モジュールに属すべき。以下の方針で調整：

**オプションA: 現状維持（推奨）**
- `config::step` にそのまま残す
- `provider` モジュールから `pub use` で再エクスポート
- 既存コードへの影響を最小化

```rust
// src/provider.rs に追加
pub use crate::config::step::{Provider, ModelTier};
```

**オプションB: 移動（影響大）**
- `Provider`, `ModelTier` を `provider::model_tier` に移動
- `config::step` から `pub use provider::model_tier::{Provider, ModelTier};`
- 既存の使用箇所を全て更新

**決定事項**: オプションAを採用（既存コードへの影響を最小化）

#### 5.2 lib.rsへのエクスポート追加

```rust
// src/lib.rs に追加
pub mod provider;
```

#### 5.3 統合テストの追加

`tests/integration/provider_test.rs` を作成：

```rust
//! Providerモジュールの統合テスト

use melted_adw::provider::{create_provider, ProviderClient};
use melted_adw::config::step::{Provider, ModelTier};

#[tokio::test]
async fn test_provider_factory_anthropic() {
    // モックAPIキーでクライアント生成テスト
    let result = create_provider(&Provider::Anthropic, "test-key".to_string());
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_provider_factory_openai() {
    let result = create_provider(&Provider::OpenAI, "test-key".to_string());
    assert!(result.is_ok());
}

// 実際のAPI呼び出しテスト（環境変数でAPIキー提供時のみ実行）
#[tokio::test]
#[ignore] // デフォルトでは無視、`cargo test -- --ignored` で実行
async fn test_real_anthropic_api_call() {
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .expect("ANTHROPIC_API_KEY environment variable not set");

    let client = create_provider(&Provider::Anthropic, api_key).unwrap();
    let response = client.execute(
        "あなたは優秀なアシスタントです",
        "こんにちは",
        &ModelTier::Light
    ).await;

    assert!(response.is_ok());
    let response = response.unwrap();
    assert!(!response.content.is_empty());
    assert!(response.tokens_used.total() > 0);
}
```

**テスト要件**:
- ファクトリー関数が正しく動作すること
- 各プロバイダーのクライアントが生成できること
- （オプション）実際のAPI呼び出しが成功すること

**成果物**:
- 統合されたproviderモジュール
- 統合テスト
- ビルド・テスト成功確認

**推定行数**: 100-150行（テストコード含む）

**担当エージェント**: 実装エージェント1（レビュー兼務）

---

## 4. 成果物チェックリスト

### ドキュメント
- [ ] 各ファイルにモジュールレベルドキュメント（`//!`）を記述
- [ ] 公開関数/構造体にdocコメント記述
- [ ] 使用例を含むドキュメント

### コード品質
- [ ] `cargo check` が成功
- [ ] `cargo clippy` で警告なし
- [ ] `cargo fmt` でフォーマット済み
- [ ] すべてのテストが成功（`cargo test`）

### 機能要件
- [ ] `ProviderClient` トレイト定義
- [ ] `AnthropicClient` 実装
- [ ] `OpenAIClient` 実装
- [ ] `create_provider` ファクトリー関数
- [ ] モデルティアマッピング
- [ ] エラー型定義

### テスト
- [ ] 単体テストカバレッジ 70%以上
- [ ] モデルマッピングの全組み合わせテスト
- [ ] エラーケースのテスト
- [ ] 統合テスト

---

## 5. リスクと対応策

| リスク | 発生確率 | 影響度 | 対応策 |
|--------|---------|-------|--------|
| クレート追加の許可が得られない | 中 | 高 | 標準ライブラリのみでの実装方法を検討 |
| APIの仕様変更 | 低 | 中 | APIバージョンを固定、変更監視 |
| 非同期処理の複雑性 | 中 | 中 | async_traitの使用、十分なテスト |
| モデル名の変更 | 低 | 低 | model_tier.rsで一元管理 |

---

## 6. 実装順序とエージェント割り当て

### 推奨実装順序

```
フェーズ0（依存関係）
    ↓
フェーズ1（エラー型・トレイト）
    ↓
フェーズ2（モデルマッピング）
    ↓
    ├─ フェーズ3（Anthropic）
    └─ フェーズ4（OpenAI）   ← 並行実装可能
    ↓
フェーズ5（統合・テスト）
```

### エージェント割り当て案

- **メインエージェント**: フェーズ0（依存関係調整）
- **実装エージェント1**: フェーズ1（基盤）+ フェーズ2（マッピング）+ フェーズ5（統合）
- **実装エージェント2**: フェーズ3（Anthropic実装）
- **実装エージェント3**: フェーズ4（OpenAI実装）

**並行実装のポイント**:
- フェーズ3とフェーズ4は独立しているため、並行実装可能
- フェーズ1-2完了後に両方を同時に開始することで効率化

---

## 7. 推定工数

| フェーズ | 推定行数 | 複雑度 | 推定時間 |
|---------|---------|-------|---------|
| フェーズ0: 依存関係 | - | 低 | 0.5時間 |
| フェーズ1: エラー型・トレイト | 200-250 | 中 | 2-3時間 |
| フェーズ2: モデルマッピング | 100-120 | 低 | 1-2時間 |
| フェーズ3: Anthropic実装 | 300-350 | 高 | 4-6時間 |
| フェーズ4: OpenAI実装 | 300-350 | 高 | 4-6時間 |
| フェーズ5: 統合・テスト | 100-150 | 中 | 2-3時間 |
| **合計** | **1000-1220** | **中-高** | **13.5-20.5時間** |

---

## 8. レビューポイント

### コードレビュー時のチェック項目

1. **アーキテクチャ**
   - トレイトベースの設計が適切か
   - 各モジュールの責務が明確か
   - 依存関係が適切か

2. **エラーハンドリング**
   - すべてのエラーケースが処理されているか
   - エラーメッセージが明確か
   - ProviderErrorの使い分けが適切か

3. **非同期処理**
   - async/awaitが正しく使用されているか
   - タイムアウトが設定されているか
   - エラー時の処理が適切か

4. **テスト**
   - 十分なテストカバレッジがあるか
   - エッジケースがテストされているか
   - モックを適切に使用しているか

5. **ドキュメント**
   - モジュールレベルのドキュメントが充実しているか
   - 公開APIにdocコメントがあるか
   - 使用例が含まれているか

---

## 9. 完了条件

以下の条件がすべて満たされた時、本タスクを完了とする：

1. ✅ すべてのフェーズの実装が完了
2. ✅ `cargo build` が成功
3. ✅ `cargo test` が成功（カバレッジ70%以上）
4. ✅ `cargo clippy` で警告なし
5. ✅ すべてのファイルに適切なドキュメントコメント
6. ✅ 統合テストが成功
7. ✅ create_provider()で両プロバイダーのクライアントが生成できる
8. ✅ モデルティアマッピングが全組み合わせで動作
9. ✅ コードレビュー完了

---

## 10. 補足資料

### 参考リンク

- [Anthropic API Documentation](https://docs.anthropic.com/claude/reference/messages_post)
- [OpenAI API Documentation](https://platform.openai.com/docs/api-reference/chat/create)
- [async-trait crate](https://docs.rs/async-trait/)
- [reqwest crate](https://docs.rs/reqwest/)

### 既存コードの参考箇所

- `src/config/workflow.rs` - DTO/ドメインモデルパターンの参考
- `src/config/step.rs` - Provider/ModelTier列挙体の定義
- `src/error.rs` - エラー型定義のパターン

---

**計画書作成完了**

本計画書に基づいて、各フェーズを順次実装してください。
各フェーズ完了時には、必ずテストを実行し、品質を確認してから次のフェーズに進んでください。
