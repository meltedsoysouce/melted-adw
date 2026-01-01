# Provider モジュール詳細実装計画書（CLI版）

**作成日**: 2026-01-01
**更新日**: 2026-01-01 (フェーズ0完了済みを反映)
**タスク名**: provider-implementation
**目的**: CLIツール呼び出しベースのproviderモジュール実装

---

## エグゼクティブサマリー

本計画書は、**HTTPクライアントではなくCLIツールを呼び出す**設計でproviderモジュールを実装します。これにより：
- APIキーの管理が不要（CLIツールが管理）
- セキュリティの向上（認証情報をコードから分離）
- ユーザーフレンドリー（事前ログインで即座に使用可能）

**使用するCLIツール**:
- **Anthropic**: `claude` コマンド（Claude Code CLI）
- **OpenAI**: `codex` コマンド（Codex CLI）

**主要な要件**:
1. 各ファイルに責務と公開API/データ構造を記述
2. データ構造（構造体/列挙体）の作成
3. CLI呼び出しベースの実装
4. LLM/人間が理解しやすいドキュメンテーション

**進捗状況**:
- ✅ フェーズ0（依存関係追加）: 完了
- ⏳ フェーズ1-5: 実装待ち

**残り推定工数**: 9.5-14.5時間（1-2日相当）
**推定総工数**: 10-15時間
**推定総行数**: 800-1000行

---

## 目次

1. [アーキテクチャ変更の概要](#1-アーキテクチャ変更の概要)
2. [現状分析（As-Is）](#2-現状分析as-is)
3. [目標状態（To-Be）](#3-目標状態to-be)
4. [実装フェーズ](#4-実装フェーズ)
5. [成功基準](#5-成功基準)
6. [リスクと対策](#6-リスクと対策)

---

## 1. アーキテクチャ変更の概要

### 1.1 旧設計 vs 新設計

| 項目 | 旧設計（HTTP API） | 新設計（CLI） |
|------|------------------|-------------|
| **通信方法** | `reqwest` でHTTP API呼び出し | `tokio::process::Command` でCLI実行 |
| **認証** | APIキーをコードで管理 | CLIツールが管理（環境変数 or ログイン） |
| **依存関係** | tokio, reqwest, async-trait, serde_json | tokio, async-trait, serde_json |
| **エラー処理** | HTTPエラー、ステータスコード | プロセス実行エラー、CLI stderr |
| **APIキー引数** | `create_provider(api_key)` | 不要（CLIが自動取得） |
| **設定** | ベースURL等 | CLIコマンドパス |

### 1.2 メリット

1. **セキュリティ**: APIキーをコードやメモリに持たない
2. **ユーザビリティ**: `claude login` / `codex login` で事前認証
3. **依存関係削減**: `reqwest` 不要（約50の推移的依存削減）
4. **柔軟性**: CLIツールのアップデートに自動追従

### 1.3 CLIツール概要

#### Claude Code CLI

```bash
# インストール
npm install -g @anthropic-ai/claude-code

# 認証
claude  # 対話モードで /login

# 非対話実行
claude -p "Analyze this code" --output-format json

# 環境変数による認証
export ANTHROPIC_API_KEY="sk-ant-..."
claude -p "task"
```

**出力形式**:
```json
{
  "response": "生成されたテキスト",
  "metadata": {
    "model": "claude-sonnet-4-5",
    "tokens": {
      "input": 150,
      "output": 320
    }
  }
}
```

#### Codex CLI

```bash
# インストール
npm install -g @openai/codex

# 認証
codex login  # OAuth フロー

# 非対話実行
codex exec --json "Analyze this code"

# 環境変数による認証
export OPENAI_API_KEY="sk-..."
codex exec "task"
```

**出力形式（JSONL）**:
```json
{"type":"turn.started","timestamp":"..."}
{"type":"item.completed","item":{"role":"assistant","content":"生成されたテキスト"}}
{"type":"turn.completed","usage":{"input_tokens":150,"output_tokens":320}}
```

---

## 2. 現状分析（As-Is）

### 2.1 ファイル状態

すべてのproviderモジュールファイルが空（1行のみ）。

### 2.2 依存関係状態

**追加済み**: serde, thiserror, toml, clap, tracing系

**必要な追加**:
- tokio（`process` feature）
- serde_json
- async-trait

**不要**:
- ~~reqwest~~ （HTTPクライアント不要）

---

## 3. 目標状態（To-Be）

### 3.1 各ファイルの責務と公開API

#### 3.1.1 `src/provider.rs`

**責務**:
- providerモジュール全体の公開APIを定義
- サブモジュールの再エクスポート
- ファクトリー関数 `create_provider()` の提供（APIキー不要）

**公開API**:
```rust
// トレイト・型の再エクスポート
pub use traits::{ProviderClient, ProviderResponse, TokenUsage, StopReason};

// ファクトリー関数（APIキー引数なし）
pub fn create_provider(
    provider: &Provider,
) -> Result<Box<dyn ProviderClient>, ProviderError>
```

**変更点**: `api_key` 引数が削除され、CLIツールに任せる

---

#### 3.1.2 `src/provider/traits.rs`

**責務**: （変更なし）
- LLMプロバイダーの共通インターフェース定義
- プロバイダー非依存のレスポンス型定義

**公開API**:
```rust
#[async_trait]
pub trait ProviderClient: Send + Sync {
    async fn execute(
        &self,
        system_prompt: &str,
        user_input: &str,
        model_tier: &ModelTier,
    ) -> Result<ProviderResponse, ProviderError>;
}

pub struct ProviderResponse {
    pub content: String,
    pub token_usage: TokenUsage,
    pub stop_reason: StopReason,
    pub model: String,
}

pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

pub enum StopReason {
    EndTurn,
    MaxTokens,
    StopSequence,
    ContentFilter,
}
```

---

#### 3.1.3 `src/provider/model_tier.rs`

**責務**: （変更なし）
- `ModelTier` から実際のモデル名への変換

**公開API**:
```rust
pub fn resolve_model(
    provider: &Provider,
    tier: &ModelTier
) -> &'static str
```

**注意**: モデル名は以下のように調整
- Anthropic: Claude Code CLIが受け付けるモデル名
- OpenAI: Codex CLIが受け付けるモデル名

---

#### 3.1.4 `src/provider/anthropic.rs`

**責務**:
- Claude Code CLI との通信
- `ProviderClient` トレイトの実装
- JSON出力のパース

**公開API**:
```rust
pub struct AnthropicClient {
    cli_command: String,  // デフォルト: "claude"
}

impl AnthropicClient {
    pub fn new() -> Self;
    pub fn with_command(cli_command: String) -> Self;
}

#[async_trait]
impl ProviderClient for AnthropicClient {
    async fn execute(...) -> Result<ProviderResponse, ProviderError>;
}
```

**内部型**（非公開）:
```rust
// Claude Code CLI の JSON レスポンス
#[derive(Deserialize)]
struct ClaudeResponse {
    response: String,
    metadata: Metadata,
}

#[derive(Deserialize)]
struct Metadata {
    model: String,
    tokens: Tokens,
}

#[derive(Deserialize)]
struct Tokens {
    input: u32,
    output: u32,
}
```

**推定行数**: 150-200行

---

#### 3.1.5 `src/provider/openai.rs`

**責務**:
- Codex CLI との通信
- `ProviderClient` トレイトの実装
- JSONL出力のパース

**公開API**:
```rust
pub struct OpenAIClient {
    cli_command: String,  // デフォルト: "codex"
}

impl OpenAIClient {
    pub fn new() -> Self;
    pub fn with_command(cli_command: String) -> Self;
}

#[async_trait]
impl ProviderClient for OpenAIClient {
    async fn execute(...) -> Result<ProviderResponse, ProviderError>;
}
```

**内部型**（非公開）:
```rust
// Codex CLI の JSONL イベント
#[derive(Deserialize)]
struct CodexEvent {
    #[serde(rename = "type")]
    event_type: String,
    item: Option<CodexItem>,
    usage: Option<CodexUsage>,
}

#[derive(Deserialize)]
struct CodexItem {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct CodexUsage {
    input_tokens: u32,
    output_tokens: u32,
}
```

**推定行数**: 180-220行

---

#### 3.1.6 `src/error.rs`（拡張）

**責務**:
- プロバイダー関連のエラー型定義

**追加API**:
```rust
#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("CLIツールが見つかりません: {0}")]
    CliNotFound(String),

    #[error("認証に失敗しました: {0}")]
    AuthenticationError(String),

    #[error("CLIコマンド実行エラー: {0}")]
    CliExecutionError(String),

    #[error("不正なモデルティア: {0}")]
    InvalidModelTier(String),

    #[error("レート制限を超えました")]
    RateLimitExceeded,

    #[error("タイムアウトしました: {0}")]
    Timeout(String),

    #[error("CLIからの不正なレスポンス: {0}")]
    InvalidResponse(String),

    #[error("プロセス実行エラー: {0}")]
    ProcessError(#[from] std::io::Error),

    #[error("JSONパースエラー: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("UTF-8デコードエラー: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
}
```

**変更点**:
- `HttpError` 削除
- `CliNotFound`, `CliExecutionError`, `ProcessError`, `Utf8Error` 追加

---

### 3.2 データ構造の全体像

```
┌─────────────────────────────────────┐
│         provider.rs                 │
│  - モジュール定義                    │
│  - 公開API再エクスポート             │
│  - ファクトリー関数（APIキーなし）   │
└──────────────┬──────────────────────┘
               │
       ┌───────┴───────┬──────────┬──────────┐
       ↓               ↓          ↓          ↓
┌─────────────┐ ┌─────────┐ ┌──────────┐ ┌──────────┐
│  traits.rs  │ │model_   │ │anthropic │ │ openai   │
│             │ │tier.rs  │ │  .rs     │ │  .rs     │
├─────────────┤ ├─────────┤ ├──────────┤ ├──────────┤
│ProviderClient│ │resolve_ │ │Anthropic │ │OpenAI    │
│  (trait)    │ │model()  │ │Client    │ │Client    │
│             │ │         │ │          │ │          │
│Provider     │ │         │ │CLI呼出   │ │CLI呼出   │
│Response     │ │         │ │claude    │ │codex     │
│             │ │         │ │          │ │          │
│TokenUsage   │ │         │ │JSONパース│ │JSONLパース│
│             │ │         │ │          │ │          │
│StopReason   │ │         │ │          │ │          │
└─────────────┘ └─────────┘ └──────────┘ └──────────┘
       ↓
       使用
       ↓
┌─────────────────┐
│   error.rs      │
│  ProviderError  │
│  (CLI版)        │
└─────────────────┘
       ↑
       使用
       ↓
┌─────────────────┐
│ tokio::process  │
│  Command        │
└─────────────────┘
```

---

## 4. 実装フェーズ

### フェーズ0: 事前準備 ✅ **完了済み**

**目的**: 依存関係の追加とCLIツールの確認

**ステータス**: ✅ **完了** - 必要な依存関係は既に追加済み

**追加済みの依存関係**:
```toml
[dependencies]
tokio = { version = "1", features = ["process", "io-util", "rt"] }
serde_json = "1.0"
async-trait = "0.1"
```

**次のステップ**: フェーズ1（基盤実装）から開始

**注意**:
- CLIツール（`claude`, `codex`）のインストールは任意
- 実装時にCLIが見つからない場合、適切なエラーメッセージが表示される設計

---

### フェーズ1: 基盤実装

**目的**: エラー型、トレイト、モジュール定義の実装

**Context Window考慮**: 低（170-210行、3ファイル）

#### タスク1.1: `src/error.rs` に `ProviderError` を追加

**実装内容**:

```rust
//! エラー型の定義
//!
//! # 責務
//!
//! アプリケーション全体で使用されるエラー型を定義する。
//! - [ConfigError]: 設定ファイルの読み込み・パースエラー
//! - [ProviderError]: LLMプロバイダー通信エラー（CLI版）

use thiserror::Error;

/// 設定ファイル関連のエラー
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("ファイルの読み込みに失敗しました: {0}")]
    FileRead(#[from] std::io::Error),
    #[error("TOMLのデシリアライズに失敗しました: {0}")]
    TomlDeserialize(#[from] toml::de::Error),
    #[error("TOMLのシリアライズに失敗しました: {0}")]
    TomlSerialize(#[from] toml::ser::Error),
    #[error("バリデーションエラー: {0}")]
    Validation(String),
}

/// LLMプロバイダー通信関連のエラー（CLI版）
///
/// CLIツール（`claude`, `codex`）を呼び出す際のエラーを表現します。
#[derive(Debug, Error)]
pub enum ProviderError {
    /// CLIツールが見つからない（未インストール）
    #[error("CLIツールが見つかりません: {0}。インストールしてください: npm install -g {1}")]
    CliNotFound(String, String),  // (コマンド名, NPMパッケージ名)

    /// 認証エラー（ログインが必要）
    #[error("認証に失敗しました: {0}。'{1} login' を実行してください")]
    AuthenticationError(String, String),  // (エラー詳細, コマンド名)

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
```

---

#### タスク1.2: `src/provider/traits.rs` の実装

**実装内容**:

```rust
//! LLMプロバイダーの共通インターフェース定義
//!
//! # 責務
//!
//! - LLMプロバイダー（Anthropic, OpenAI等）の共通トレイト [ProviderClient] を定義
//! - プロバイダー非依存のレスポンス型 [ProviderResponse] を提供
//! - トークン使用量 [TokenUsage] と停止理由 [StopReason] の型を定義
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
    /// - [ProviderError::CliNotFound]: CLIツールが未インストール
    /// - [ProviderError::AuthenticationError]: 認証失敗（ログインが必要）
    /// - [ProviderError::CliExecutionError]: CLI実行エラー
    /// - [ProviderError::RateLimitExceeded]: レート制限超過
    /// - [ProviderError::Timeout]: タイムアウト
    /// - [ProviderError::InvalidResponse]: 不正なレスポンス
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
#[derive(Debug, Clone, Copy)]
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
```

---

#### タスク1.3: `src/provider.rs` の実装

**実装内容**:

```rust
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
//! - [traits]: 共通インターフェース（[ProviderClient]トレイト等）
//! - [model_tier]: モデルティアマッピング
//! - [anthropic]: Anthropic Claude Code CLI クライアント
//! - [openai]: OpenAI Codex CLI クライアント
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
/// - [ProviderError::CliNotFound]: CLIツールが未インストール
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
```

**推定行数合計**: 170-210行

**検証方法**:
```bash
cargo build
cargo clippy -- -D warnings
cargo doc --no-deps --open
```

**推定時間**: 2-3時間

---

### フェーズ2: モデルティアマッピング

**目的**: ModelTier から実際のモデル名への変換機能の実装

**Context Window考慮**: 極小（80-90行、1ファイル）

#### タスク2.1: `src/provider/model_tier.rs` の実装

**実装内容**:

```rust
//! モデルティアマッピング
//!
//! # 責務
//!
//! - [ModelTier] と [Provider] の組み合わせから、実際のモデル名を解決
//! - プロバイダー別のモデル名定数を管理
//!
//! # マッピング表
//!
//! | Tier   | Anthropic       | OpenAI      |
//! |--------|----------------|-------------|
//! | Heavy  | claude-opus-4  | o1          |
//! | Medium | claude-sonnet-4-5 | gpt-4o   |
//! | Light  | claude-haiku   | gpt-4o-mini |
//!
//! # 注意
//!
//! モデル名はCLIツールが受け付ける形式に合わせています。
//! CLIツールのバージョンやAPI仕様の変更により、モデル名が変わる可能性があります。
//!
//! # 使用例
//!
//! ```rust
//! use melted_adw::provider::model_tier::resolve_model;
//! use melted_adw::config::step::{Provider, ModelTier};
//!
//! let model = resolve_model(&Provider::Anthropic, &ModelTier::Medium);
//! assert_eq!(model, "claude-sonnet-4-5");
//! ```

use crate::config::step::{Provider, ModelTier};

// Anthropic モデル名定数
const ANTHROPIC_HEAVY: &str = "claude-opus-4";
const ANTHROPIC_MEDIUM: &str = "claude-sonnet-4-5";
const ANTHROPIC_LIGHT: &str = "claude-haiku";

// OpenAI モデル名定数
const OPENAI_HEAVY: &str = "o1";
const OPENAI_MEDIUM: &str = "gpt-4o";
const OPENAI_LIGHT: &str = "gpt-4o-mini";

/// モデルティアとプロバイダーから実際のモデル名を解決する
///
/// # 引数
///
/// - `provider`: プロバイダーの種類（Anthropic or OpenAI）
/// - `tier`: モデルティア（Heavy/Medium/Light）
///
/// # 戻り値
///
/// モデル名の文字列スライス（'static ライフタイム）
///
/// # 例
///
/// ```rust
/// use melted_adw::provider::model_tier::resolve_model;
/// use melted_adw::config::step::{Provider, ModelTier};
///
/// // Anthropic Medium -> claude-sonnet-4-5
/// assert_eq!(
///     resolve_model(&Provider::Anthropic, &ModelTier::Medium),
///     "claude-sonnet-4-5"
/// );
///
/// // OpenAI Heavy -> o1
/// assert_eq!(
///     resolve_model(&Provider::OpenAI, &ModelTier::Heavy),
///     "o1"
/// );
/// ```
pub fn resolve_model(provider: &Provider, tier: &ModelTier) -> &'static str {
    match (provider, tier) {
        // Anthropic
        (Provider::Anthropic, ModelTier::Heavy) => ANTHROPIC_HEAVY,
        (Provider::Anthropic, ModelTier::Medium) => ANTHROPIC_MEDIUM,
        (Provider::Anthropic, ModelTier::Light) => ANTHROPIC_LIGHT,

        // OpenAI
        (Provider::OpenAI, ModelTier::Heavy) => OPENAI_HEAVY,
        (Provider::OpenAI, ModelTier::Medium) => OPENAI_MEDIUM,
        (Provider::OpenAI, ModelTier::Light) => OPENAI_LIGHT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anthropic_models() {
        assert_eq!(resolve_model(&Provider::Anthropic, &ModelTier::Heavy), "claude-opus-4");
        assert_eq!(resolve_model(&Provider::Anthropic, &ModelTier::Medium), "claude-sonnet-4-5");
        assert_eq!(resolve_model(&Provider::Anthropic, &ModelTier::Light), "claude-haiku");
    }

    #[test]
    fn test_openai_models() {
        assert_eq!(resolve_model(&Provider::OpenAI, &ModelTier::Heavy), "o1");
        assert_eq!(resolve_model(&Provider::OpenAI, &ModelTier::Medium), "gpt-4o");
        assert_eq!(resolve_model(&Provider::OpenAI, &ModelTier::Light), "gpt-4o-mini");
    }
}
```

**検証方法**:
```bash
cargo build
cargo test model_tier
cargo clippy -- -D warnings
```

**推定時間**: 1-1.5時間

---

### フェーズ3: Anthropic実装

**目的**: Claude Code CLI クライアントの実装

**Context Window考慮**: 中（150-200行、1ファイル）

#### タスク3.1: `src/provider/anthropic.rs` の実装

**実装スケルトン**:

```rust
//! Anthropic Claude Code CLI クライアント
//!
//! # 責務
//!
//! - Claude Code CLI (`claude` コマンド) との通信を担当
//! - [ProviderClient] トレイトを実装し、統一インターフェースを提供
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
//! # CLI インターフェース
//!
//! ```bash
//! claude -p "prompt text" --output-format json
//! ```
//!
//! **出力形式**:
//! ```json
//! {
//!   "response": "生成されたテキスト",
//!   "metadata": {
//!     "model": "claude-sonnet-4-5",
//!     "tokens": {
//!       "input": 150,
//!       "output": 320
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
use crate::provider::model_tier::resolve_model;
use crate::provider::traits::{ProviderClient, ProviderResponse, StopReason, TokenUsage};

/// Anthropic Claude Code CLI クライアント
pub struct AnthropicClient {
    /// CLIコマンド名（デフォルト: "claude"）
    cli_command: String,
}

impl AnthropicClient {
    /// 新しいAnthropicクライアントを生成
    ///
    /// デフォルトのCLIコマンド名 `"claude"` を使用します。
    ///
    /// # 認証
    ///
    /// 以下のいずれかが必要です：
    /// - 環境変数 `ANTHROPIC_API_KEY` を設定
    /// - `claude` を起動して `/login` コマンドを実行済み
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
            cli_command: "claude".to_string(),
        }
    }

    /// カスタムCLIコマンド名でクライアントを生成
    ///
    /// テスト時やCLIツールのパスが特殊な場合に使用します。
    ///
    /// # 引数
    ///
    /// - `cli_command`: CLIコマンドのパス（例: "/usr/local/bin/claude"）
    ///
    /// # 例
    ///
    /// ```rust
    /// use melted_adw::provider::anthropic::AnthropicClient;
    ///
    /// let client = AnthropicClient::with_command("/opt/claude/bin/claude".to_string());
    /// ```
    pub fn with_command(cli_command: String) -> Self {
        Self { cli_command }
    }

    /// CLIツールが利用可能かチェック
    async fn check_cli_available(&self) -> Result<(), ProviderError> {
        let status = Command::new("which")
            .arg(&self.cli_command)
            .output()
            .await?
            .status;

        if !status.success() {
            return Err(ProviderError::CliNotFound(
                self.cli_command.clone(),
                "@anthropic-ai/claude-code".to_string(),
            ));
        }

        Ok(())
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
        // CLIツールが利用可能かチェック
        self.check_cli_available().await?;

        // モデル名を解決
        let model = resolve_model(&Provider::Anthropic, model_tier);

        // システムプロンプトとユーザー入力を結合
        let combined_prompt = format!("{}\n\n{}", system_prompt, user_input);

        // CLIコマンドを実行
        let output = Command::new(&self.cli_command)
            .arg("-p")
            .arg(&combined_prompt)
            .arg("--output-format")
            .arg("json")
            .arg("--model")
            .arg(model)
            .output()
            .await?;

        // 終了コードチェック
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);

            // 認証エラーの検出
            if stderr.contains("authentication") || stderr.contains("API key") {
                return Err(ProviderError::AuthenticationError(
                    stderr.to_string(),
                    self.cli_command.clone(),
                ));
            }

            // レート制限の検出
            if stderr.contains("rate limit") || stderr.contains("429") {
                return Err(ProviderError::RateLimitExceeded);
            }

            return Err(ProviderError::CliExecutionError(stderr.to_string()));
        }

        // JSONレスポンスをパース
        let stdout = String::from_utf8(output.stdout)?;
        let cli_response: ClaudeResponse = serde_json::from_str(&stdout)?;

        // 共通型に変換
        Ok(ProviderResponse {
            content: cli_response.response,
            token_usage: TokenUsage {
                input_tokens: cli_response.metadata.tokens.input,
                output_tokens: cli_response.metadata.tokens.output,
            },
            stop_reason: StopReason::EndTurn,  // Claude Code CLIは停止理由を返さない
            model: cli_response.metadata.model,
        })
    }
}

// 内部型定義（非公開）

/// Claude Code CLI の JSON レスポンス
#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    /// 生成されたテキスト
    response: String,
    /// メタデータ（モデル名、トークン数等）
    metadata: Metadata,
}

/// レスポンスのメタデータ
#[derive(Debug, Deserialize)]
struct Metadata {
    /// 使用されたモデル名
    model: String,
    /// トークン使用量
    tokens: Tokens,
}

/// トークン使用量
#[derive(Debug, Deserialize)]
struct Tokens {
    /// 入力トークン数
    input: u32,
    /// 出力トークン数
    output: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anthropic_client_creation() {
        let client = AnthropicClient::new();
        assert_eq!(client.cli_command, "claude");
    }

    #[test]
    fn test_anthropic_client_with_custom_command() {
        let client = AnthropicClient::with_command("/opt/claude".to_string());
        assert_eq!(client.cli_command, "/opt/claude");
    }

    // 注: 実際のCLI呼び出しテストは統合テストで実施
    // （モックCLIまたは実際のCLIツールを使用）
}
```

**検証方法**:
```bash
cargo build
cargo test anthropic
cargo clippy -- -D warnings
```

**推定時間**: 2-3時間

---

### フェーズ4: OpenAI実装

**目的**: Codex CLI クライアントの実装

**Context Window考慮**: 中（180-220行、1ファイル）

#### タスク4.1: `src/provider/openai.rs` の実装

**実装スケルトン**:

```rust
//! OpenAI Codex CLI クライアント
//!
//! # 責務
//!
//! - Codex CLI (`codex` コマンド) との通信を担当
//! - [ProviderClient] トレイトを実装し、統一インターフェースを提供
//! - Codex固有のJSONL出力形式と共通型の変換
//!
//! # CLIツール
//!
//! - **コマンド**: `codex`
//! - **インストール**: `npm install -g @openai/codex`
//! - **認証方法**:
//!   1. `codex login` を実行（OAuth フロー）
//!   2. 環境変数 `OPENAI_API_KEY` を設定
//!
//! # CLI インターフェース
//!
//! ```bash
//! codex exec --json "task description"
//! ```
//!
//! **出力形式（JSONL）**:
//! ```json
//! {"type":"turn.started","timestamp":"..."}
//! {"type":"item.completed","item":{"role":"assistant","content":"生成されたテキスト"}}
//! {"type":"turn.completed","usage":{"input_tokens":150,"output_tokens":320}}
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
//!     // 事前に `codex login` または環境変数設定が必要
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
use crate::provider::model_tier::resolve_model;
use crate::provider::traits::{ProviderClient, ProviderResponse, StopReason, TokenUsage};

/// OpenAI Codex CLI クライアント
pub struct OpenAIClient {
    /// CLIコマンド名（デフォルト: "codex"）
    cli_command: String,
}

impl OpenAIClient {
    /// 新しいOpenAIクライアントを生成
    ///
    /// デフォルトのCLIコマンド名 `"codex"` を使用します。
    ///
    /// # 認証
    ///
    /// 以下のいずれかが必要です：
    /// - `codex login` を実行済み
    /// - 環境変数 `OPENAI_API_KEY` を設定
    ///
    /// # 例
    ///
    /// ```rust
    /// use melted_adw::provider::openai::OpenAIClient;
    ///
    /// let client = OpenAIClient::new();
    /// ```
    pub fn new() -> Self {
        Self {
            cli_command: "codex".to_string(),
        }
    }

    /// カスタムCLIコマンド名でクライアントを生成
    ///
    /// テスト時やCLIツールのパスが特殊な場合に使用します。
    ///
    /// # 引数
    ///
    /// - `cli_command`: CLIコマンドのパス（例: "/usr/local/bin/codex"）
    ///
    /// # 例
    ///
    /// ```rust
    /// use melted_adw::provider::openai::OpenAIClient;
    ///
    /// let client = OpenAIClient::with_command("/opt/codex/bin/codex".to_string());
    /// ```
    pub fn with_command(cli_command: String) -> Self {
        Self { cli_command }
    }

    /// CLIツールが利用可能かチェック
    async fn check_cli_available(&self) -> Result<(), ProviderError> {
        let status = Command::new("which")
            .arg(&self.cli_command)
            .output()
            .await?
            .status;

        if !status.success() {
            return Err(ProviderError::CliNotFound(
                self.cli_command.clone(),
                "@openai/codex".to_string(),
            ));
        }

        Ok(())
    }

    /// JSONL出力から最終結果を抽出
    fn parse_jsonl_output(&self, output: &str) -> Result<(String, TokenUsage), ProviderError> {
        let mut content = String::new();
        let mut usage: Option<TokenUsage> = None;

        for line in output.lines() {
            let event: CodexEvent = serde_json::from_str(line)
                .map_err(|e| ProviderError::InvalidResponse(format!("JSONL parse error: {}", e)))?;

            match event.event_type.as_str() {
                "item.completed" => {
                    if let Some(item) = event.item {
                        if item.role == "assistant" {
                            content = item.content;
                        }
                    }
                }
                "turn.completed" => {
                    if let Some(u) = event.usage {
                        usage = Some(TokenUsage {
                            input_tokens: u.input_tokens,
                            output_tokens: u.output_tokens,
                        });
                    }
                }
                _ => {}
            }
        }

        let usage = usage.ok_or_else(|| {
            ProviderError::InvalidResponse("No usage information in response".to_string())
        })?;

        Ok((content, usage))
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
        // CLIツールが利用可能かチェック
        self.check_cli_available().await?;

        // モデル名を解決
        let model = resolve_model(&Provider::OpenAI, model_tier);

        // システムプロンプトとユーザー入力を結合
        let combined_prompt = format!("{}\n\n{}", system_prompt, user_input);

        // CLIコマンドを実行
        let output = Command::new(&self.cli_command)
            .arg("exec")
            .arg("--json")
            .arg("--model")
            .arg(model)
            .arg(&combined_prompt)
            .output()
            .await?;

        // 終了コードチェック
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);

            // 認証エラーの検出
            if stderr.contains("authentication") || stderr.contains("login") {
                return Err(ProviderError::AuthenticationError(
                    stderr.to_string(),
                    self.cli_command.clone(),
                ));
            }

            // レート制限の検出
            if stderr.contains("rate limit") || stderr.contains("429") {
                return Err(ProviderError::RateLimitExceeded);
            }

            return Err(ProviderError::CliExecutionError(stderr.to_string()));
        }

        // JSONL出力をパース
        let stdout = String::from_utf8(output.stdout)?;
        let (content, token_usage) = self.parse_jsonl_output(&stdout)?;

        // 共通型に変換
        Ok(ProviderResponse {
            content,
            token_usage,
            stop_reason: StopReason::EndTurn,  // Codex CLIからは取得不可
            model: model.to_string(),
        })
    }
}

// 内部型定義（非公開）

/// Codex CLI の JSONL イベント
#[derive(Debug, Deserialize)]
struct CodexEvent {
    /// イベント種別
    #[serde(rename = "type")]
    event_type: String,

    /// アイテム（item.completed イベント用）
    #[serde(skip_serializing_if = "Option::is_none")]
    item: Option<CodexItem>,

    /// トークン使用量（turn.completed イベント用）
    #[serde(skip_serializing_if = "Option::is_none")]
    usage: Option<CodexUsage>,
}

/// Codex のアイテム（メッセージ）
#[derive(Debug, Deserialize)]
struct CodexItem {
    /// 役割（"assistant", "user"等）
    role: String,
    /// 内容
    content: String,
}

/// Codex のトークン使用量
#[derive(Debug, Deserialize)]
struct CodexUsage {
    /// 入力トークン数
    input_tokens: u32,
    /// 出力トークン数
    output_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_client_creation() {
        let client = OpenAIClient::new();
        assert_eq!(client.cli_command, "codex");
    }

    #[test]
    fn test_openai_client_with_custom_command() {
        let client = OpenAIClient::with_command("/opt/codex".to_string());
        assert_eq!(client.cli_command, "/opt/codex");
    }

    #[test]
    fn test_parse_jsonl_output() {
        let client = OpenAIClient::new();
        let jsonl = r#"{"type":"turn.started","timestamp":"2026-01-01T10:00:00Z"}
{"type":"item.completed","item":{"role":"assistant","content":"Hello, world!"}}
{"type":"turn.completed","usage":{"input_tokens":10,"output_tokens":5}}"#;

        let (content, usage) = client.parse_jsonl_output(jsonl).unwrap();
        assert_eq!(content, "Hello, world!");
        assert_eq!(usage.input_tokens, 10);
        assert_eq!(usage.output_tokens, 5);
    }

    // 注: 実際のCLI呼び出しテストは統合テストで実施
}
```

**検証方法**:
```bash
cargo build
cargo test openai
cargo clippy -- -D warnings
```

**推定時間**: 2-3時間

---

### フェーズ5: 統合・検証

**目的**: 全体のビルド・テスト・ドキュメント生成の確認

**Context Window考慮**: 低（検証タスクのみ）

#### タスク5.1: lib.rsの更新

**ファイル**: `/home/metal/repos/melted-adw/src/lib.rs`

```rust
pub mod config;
pub mod error;
pub mod provider;  // 追加
```

#### タスク5.2: 全体ビルド・テスト

```bash
cargo clean
cargo build --release
cargo test --all
cargo clippy --all-targets -- -D warnings
cargo doc --no-deps --open
```

#### タスク5.3: README.md の更新（オプション）

CLIツールのインストールと認証手順を追加：

```markdown
## セットアップ

### CLIツールのインストール

```bash
# Anthropic Claude Code CLI
npm install -g @anthropic-ai/claude-code

# OpenAI Codex CLI
npm install -g @openai/codex
```

### 認証

**Anthropic**:
```bash
# 方法1: 対話的ログイン
claude
# 起動後、/login コマンドを実行

# 方法2: 環境変数
export ANTHROPIC_API_KEY="sk-ant-..."
```

**OpenAI**:
```bash
# 方法1: OAuth ログイン
codex login

# 方法2: 環境変数
export OPENAI_API_KEY="sk-..."
```
```

**推定時間**: 1-2時間

---

## 5. 成功基準

### 5.1 機能要件

- [ ] すべてのファイルに責務が記述されている
- [ ] すべてのファイルに公開API/データ構造が記述されている
- [ ] データ構造（構造体/列挙体）が完全実装されている
- [ ] ProviderClientトレイトが実装されている
- [ ] create_provider()ファクトリー関数が動作する（APIキー引数なし）
- [ ] モデルティアマッピングが全組み合わせで機能する
- [ ] CLIツールが正しく呼び出される
- [ ] JSON/JSONL出力が正しくパースされる

### 5.2 品質要件

- [ ] `cargo build --release` が警告なく成功
- [ ] `cargo test --all` が100%成功
- [ ] `cargo clippy --all-targets -- -D warnings` が警告ゼロ
- [ ] `cargo doc --no-deps` でドキュメントが生成される
- [ ] すべての公開APIにドキュメントコメントが存在
- [ ] 使用例が適切に記載されている

### 5.3 ドキュメント要件

- [ ] 各ファイルにファイルレベルドキュメント（`//!`）が存在
- [ ] 責務セクション（`# 責務`）が記述されている
- [ ] CLIツールの使用方法が記述されている
- [ ] 認証方法が説明されている
- [ ] 公開API（`pub`）にドキュメントコメント（`///`）が存在
- [ ] 引数・戻り値・エラーが説明されている
- [ ] 使用例（`# 例` または `# 使用例`）が記載されている

---

## 6. リスクと対策

### 6.1 依存関係追加の制約

**リスク**: CLAUDE.mdに「原則crateの追加は禁止」とある

**対策**:
- フェーズ0でユーザーに明示的に許可を取得
- 必要性を説明（非同期実行、JSONパース）
- reqwestが不要になることを強調（依存削減）

### 6.2 CLIツールの未インストール

**リスク**: ユーザー環境にCLIツールがインストールされていない

**対策**:
- `check_cli_available()` で事前チェック
- エラーメッセージにインストール手順を含める
- README.mdにセットアップ手順を記載

### 6.3 認証エラー

**リスク**: CLIツールが未ログイン、または環境変数未設定

**対策**:
- stderr出力から認証エラーを検出
- エラーメッセージに解決方法を含める（`codex login` 等）
- README.mdに認証手順を記載

### 6.4 CLI出力形式の変更

**リスク**: CLIツールのアップデートで出力形式が変わる可能性

**対策**:
- serde_jsonのエラーを適切にハンドリング
- バージョン情報をドキュメントに記載
- 柔軟なパース処理（オプショナルフィールド）

### 6.5 Context Window超過

**リスク**: フェーズ3/4で大量のコードを扱う

**対策**:
- フェーズを適切に分割（各フェーズ150-220行程度）
- 実装スケルトンを詳細に提供
- 参照ファイルを最小限にする

---

## 7. 実装順序のサマリー

```
フェーズ0: 事前準備 ✅ 完了済み
   └─ 依存関係追加（tokio, serde_json, async-trait）

----- 以下、実装が必要 -----

フェーズ1: 基盤実装（2-3時間）
   ├─ error.rs: ProviderError追加（CLI版）
   ├─ traits.rs: トレイト・型定義
   └─ provider.rs: モジュール定義（APIキーなし）

フェーズ2: モデルマッピング（1-1.5時間）
   └─ model_tier.rs: resolve_model()実装

フェーズ3: Anthropic実装（2-3時間）
   └─ anthropic.rs: Claude Code CLI呼び出し

フェーズ4: OpenAI実装（2-3時間）
   └─ openai.rs: Codex CLI呼び出し

フェーズ5: 統合・検証（1-2時間）
   ├─ ビルド・テスト確認
   ├─ Clippy警告チェック
   ├─ ドキュメント生成
   └─ lib.rs更新、README更新

残り推定時間: 9.5-14.5時間（1-2日相当）
総推定時間（全体）: 10-15時間
```

**旧設計との比較**:
- 旧: 13.5-20.5時間（HTTPクライアント実装）
- 新: 10-15時間（CLI呼び出し、うちフェーズ0完了済み）
- **削減**: 3.5-5.5時間（reqwest実装が不要）

---

## 8. 参考資料

### 8.1 既存ドキュメント

- `/home/metal/repos/melted-adw/specs/provider-implementation/analysis-report.md`: 詳細分析（HTTP API版）
- `/home/metal/repos/melted-adw/specs/provider-implementation/as-is-analysis.md`: 現状分析
- `/home/metal/repos/melted-adw/README.md`: プロジェクト概要

### 8.2 CLIツール公式ドキュメント

- [Claude Code CLI Reference](https://code.claude.com/docs/en/cli-reference)
- [Run Claude Code Programmatically](https://code.claude.com/docs/en/headless)
- [Codex CLI Command Line Options](https://developers.openai.com/codex/cli/reference/)
- [Codex CLI Features](https://developers.openai.com/codex/cli/features/)

### 8.3 参考実装

- `/home/metal/repos/melted-adw/src/config/step.rs`: DTO/ドメインモデル分離パターン

---

**計画書作成日**: 2026-01-01
**最終更新日**: 2026-01-01 (フェーズ0完了済みを反映)
**バージョン**: 2.1 (CLI版、フェーズ0完了)
