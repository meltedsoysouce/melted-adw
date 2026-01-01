# Provider モジュール分析レポート

**作成日**: 2026-01-01
**分析対象**: src/provider/ モジュール
**分析レベル**: Medium

---

## 1. 現状分析（As-Is）

### 1.1 ファイル構造

Provider モジュールは以下のファイルで構成されている：

```
src/provider/
├── provider.rs          - モジュール定義（空）
├── traits.rs           - トレイト定義（空）
├── model_tier.rs       - モデルティアマッピング（空）
├── anthropic.rs        - Anthropic実装（空）
└── openai.rs           - OpenAI実装（空）
```

**実装状況**: 全ファイルが空（0-1行）

### 1.2 既存の関連データ構造

`src/config/step.rs` に以下の列挙体が定義されている：

```rust
/// AI プロバイダー
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    /// Anthropic (Claude Code)
    Anthropic,
    /// OpenAI (Codex)
    OpenAI,
}

/// モデルのティア（Heavy/Medium/Light）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelTier {
    /// 複雑な推論タスク用（例: Claude Opus, GPT-4o）
    Heavy,
    /// 一般的なタスク用（例: Claude Sonnet, GPT-4）
    Medium,
    /// 簡単なタスク用（例: Claude Haiku, GPT-3.5）
    Light,
}
```

これらは `WorkflowStep` 構造体で使用されている：

```rust
pub struct WorkflowStep {
    provider: Provider,
    model_tier: ModelTier,
    // ...
}
```

### 1.3 エラーハンドリング基盤

`src/error.rs` には `ConfigError` のみ定義されており、Provider固有のエラー型は存在しない。

```rust
pub enum ConfigError {
    FileRead(#[from] std::io::Error),
    TomlDeserialize(#[from] toml::de::Error),
    TomlSerialize(#[from] toml::ser::Error),
    Validation(String),
}
```

### 1.4 依存関係の状態

現在の `Cargo.toml`:
```toml
[dependencies]
clap = "4.5.53"
serde = "1.0.228"
thiserror = "2.0.9"
toml = "0.9.10"
tracing = "0.1.44"
tracing-appender = "0.2.4"
tracing-subscriber = "0.3.22"
```

**不足している依存関係**:
- `tokio` - 非同期ランタイム
- `reqwest` - HTTPクライアント
- `async-trait` - 非同期トレイト用
- `serde_json` - JSONシリアライゼーション

**制約**: CLAUDE.md により「原則crateの追加は禁止」→ ユーザー許可が必要

---

## 2. 要求仕様（To-Be）

### 2.1 アーキテクチャ上の役割

Provider モジュールは、README.md に記載されているアーキテクチャ図において、以下の位置を占める：

```
Workflow Engine
    ↓
Provider Abstraction Layer  ← このレイヤーを実装
    ↓
Anthropic / OpenAI API
```

**責務**:
1. 複数のAIプロバイダーを統一インターフェースで扱う抽象化
2. ModelTier（Heavy/Medium/Light）から具体的なモデル名へのマッピング
3. API認証とリクエスト処理
4. レスポンスの統一形式への変換
5. エラーハンドリングとリトライ（オプション）
6. テレメトリーデータの提供

### 2.2 データフロー

```
WorkflowStep
  ├─ provider: Provider
  └─ model_tier: ModelTier
       ↓
create_provider(provider, api_key)
       ↓
Box<dyn ProviderClient>
       ↓
execute(system_prompt, user_input, model_tier)
       ↓
Resolve ModelTier → 具体的なモデル名
  - Anthropic::Heavy → "claude-opus-4"
  - OpenAI::Medium → "gpt-4o"
       ↓
API呼び出し（HTTP POST）
       ↓
ProviderResponse {
  content: String,
  tokens_used: TokenUsage,
  stop_reason: StopReason,
  model_used: String,
}
```

### 2.3 必要なトレイトとデータ構造

#### ProviderClient トレイト
```rust
#[async_trait]
pub trait ProviderClient: Send + Sync {
    async fn execute(
        &self,
        system_prompt: &str,
        user_input: &str,
        model_tier: &ModelTier,
    ) -> Result<ProviderResponse, ProviderError>;

    fn provider_name(&self) -> &str;
}
```

#### 共通レスポンス型
```rust
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
```

#### エラー型
```rust
pub enum ProviderError {
    AuthenticationError(String),
    ApiError(String),
    InvalidModelTier(String),
    RateLimitExceeded,
    Timeout(String),
    InvalidResponse(String),
    HttpError(#[from] reqwest::Error),
    JsonError(#[from] serde_json::Error),
}
```

### 2.4 モデルマッピング仕様

| ModelTier | Anthropic | OpenAI |
|-----------|-----------|--------|
| Heavy | claude-opus-4 | o1 |
| Medium | claude-sonnet-4 | gpt-4o |
| Light | claude-haiku-3-5 | gpt-4o-mini |

---

## 3. ギャップ分析

### 3.1 実装が必要な項目

| コンポーネント | 現状 | To-Be | ギャップ |
|--------------|------|-------|---------|
| **ProviderClient トレイト** | なし | async トレイト定義 | 新規実装 |
| **ProviderResponse 型** | なし | 共通レスポンス型 | 新規実装 |
| **ProviderError 型** | なし | エラー列挙体 | error.rsに追加 |
| **ModelTier マッピング** | なし | resolve_model関数 | 新規実装 |
| **AnthropicClient** | なし | ProviderClient実装 | 新規実装 |
| **OpenAIClient** | なし | ProviderClient実装 | 新規実装 |
| **create_provider** | なし | ファクトリー関数 | 新規実装 |
| **非同期依存** | なし | tokio, async-trait | Cargo.toml追加 |
| **HTTP依存** | なし | reqwest | Cargo.toml追加 |

### 3.2 影響を受けるファイル

| ファイル | 変更内容 | 影響度 |
|---------|---------|-------|
| `Cargo.toml` | 依存関係追加 | 中（要許可） |
| `src/error.rs` | ProviderError追加 | 低（新規追加のみ） |
| `src/provider.rs` | モジュール定義・ファクトリー | 中（新規実装） |
| `src/provider/traits.rs` | トレイト・型定義 | 中（新規実装） |
| `src/provider/model_tier.rs` | マッピングロジック | 低（新規実装） |
| `src/provider/anthropic.rs` | クライアント実装 | 高（新規実装） |
| `src/provider/openai.rs` | クライアント実装 | 高（新規実装） |
| `src/lib.rs` | pub mod追加 | 低 |
| `src/config/step.rs` | （オプション）再エクスポート調整 | 低 |

---

## 4. 実装パターンの参考

### 4.1 Config モジュールから学ぶべきパターン

Config モジュール（特に `config/workflow.rs`）は、本プロジェクトの実装品質のベンチマークとなる。

#### パターン1: DTO / ドメインモデル分離

```rust
// DTO（TOMLデシリアライズ用）
pub struct WorkflowDto { /* ... */ }

// ドメインモデル（バリデーション済み）
pub struct Workflow { /* ... */ }

// TryFromでバリデーション
impl TryFrom<WorkflowDto> for Workflow {
    fn try_from(dto: WorkflowDto) -> Result<Self, ConfigError> {
        // バリデーション処理
    }
}
```

**Provider への適用**:
- API レスポンス DTO（`AnthropicResponse`, `OpenAIResponse`）
- ドメインモデル（`ProviderResponse`）
- 変換時にバリデーション

#### パターン2: 充実したドキュメント

```rust
//! モジュールレベルのドキュメント
//!
//! # 責務
//! - 箇条書きで明確に
//!
//! ## 主な機能
//! - 機能リスト
//!
//! ## 使用例
//! ```rust,ignore
//! // コード例
//! ```
```

**Provider への適用**:
- 各ファイルに詳細なモジュールレベルドキュメント
- 使用例を含む
- 責務を明確に記述

#### パターン3: 包括的なテスト

```rust
#[cfg(test)]
mod tests {
    // 正常系テスト
    #[test]
    fn test_validation_success() { /* ... */ }

    // 異常系テスト
    #[test]
    fn test_validation_empty_workflow_name() { /* ... */ }

    // ラウンドトリップテスト
    #[test]
    fn test_round_trip_conversion() { /* ... */ }
}
```

**Provider への適用**:
- 各クライアントの正常系・異常系テスト
- モデルマッピングの全組み合わせテスト
- 統合テスト

---

## 5. API仕様調査

### 5.1 Anthropic Messages API

**エンドポイント**: `https://api.anthropic.com/v1/messages`

**リクエストヘッダー**:
```
x-api-key: {API_KEY}
anthropic-version: 2023-06-01
content-type: application/json
```

**リクエストボディ**:
```json
{
  "model": "claude-sonnet-4",
  "max_tokens": 4096,
  "system": "あなたは優秀なエンジニアです",
  "messages": [
    {
      "role": "user",
      "content": "Rustでハローワールドを書いて"
    }
  ]
}
```

**レスポンス**:
```json
{
  "id": "msg_xxx",
  "model": "claude-sonnet-4",
  "content": [
    {
      "type": "text",
      "text": "以下のようにハローワールドを書けます..."
    }
  ],
  "stop_reason": "end_turn",
  "usage": {
    "input_tokens": 50,
    "output_tokens": 100
  }
}
```

### 5.2 OpenAI Chat Completions API

**エンドポイント**: `https://api.openai.com/v1/chat/completions`

**リクエストヘッダー**:
```
Authorization: Bearer {API_KEY}
content-type: application/json
```

**リクエストボディ**:
```json
{
  "model": "gpt-4o",
  "messages": [
    {
      "role": "system",
      "content": "あなたは優秀なエンジニアです"
    },
    {
      "role": "user",
      "content": "Rustでハローワールドを書いて"
    }
  ],
  "max_tokens": 4096
}
```

**レスポンス**:
```json
{
  "id": "chatcmpl-xxx",
  "model": "gpt-4o",
  "choices": [
    {
      "message": {
        "role": "assistant",
        "content": "以下のようにハローワールドを書けます..."
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 50,
    "completion_tokens": 100,
    "total_tokens": 150
  }
}
```

---

## 6. 技術的課題と解決策

### 6.1 非同期処理の扱い

**課題**: Rustの非同期処理は複雑で、トレイトメソッドを非同期にするには工夫が必要

**解決策**: `async-trait` クレートを使用

```rust
use async_trait::async_trait;

#[async_trait]
pub trait ProviderClient {
    async fn execute(...) -> Result<...>;
}
```

### 6.2 HTTPタイムアウト処理

**課題**: APIリクエストが無限にハングする可能性

**解決策**: `reqwest::Client` でタイムアウトを設定

```rust
let client = Client::builder()
    .timeout(std::time::Duration::from_secs(300))
    .build()?;
```

### 6.3 エラー型の変換

**課題**: `reqwest::Error` と `serde_json::Error` を `ProviderError` に変換

**解決策**: `thiserror` の `#[from]` 属性を使用

```rust
#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("HTTPリクエストエラー: {0}")]
    HttpError(#[from] reqwest::Error),
}
```

### 6.4 トレイトオブジェクトのライフタイム

**課題**: `Box<dyn ProviderClient>` で返す場合、Send + Sync が必要

**解決策**: トレイトに `Send + Sync` を追加

```rust
pub trait ProviderClient: Send + Sync {
    // ...
}
```

---

## 7. テスト戦略

### 7.1 単体テスト

**対象**:
- モデルマッピング関数
- エラー型の変換
- クライアント生成（APIキーバリデーション）

**ツール**:
- 標準の `#[test]`
- `tokio::test` マクロ（非同期テスト用）

### 7.2 統合テスト

**対象**:
- ファクトリー関数でのクライアント生成
- モックAPIサーバーを使った実際のHTTPリクエスト

**ツール**:
- `mockito` または `wiremock`（HTTPモック用）
- `tests/integration/` ディレクトリ

### 7.3 E2Eテスト（オプション）

**対象**:
- 実際のAPI呼び出し（環境変数でAPIキー提供時のみ）

**実装**:
```rust
#[tokio::test]
#[ignore] // デフォルトでは無視
async fn test_real_api_call() {
    let api_key = std::env::var("ANTHROPIC_API_KEY").ok();
    if api_key.is_none() {
        return; // APIキーがなければスキップ
    }
    // 実際のAPI呼び出しテスト
}
```

---

## 8. リスク分析

| リスク | 確率 | 影響 | 軽減策 |
|--------|------|------|--------|
| クレート追加の許可が得られない | 中 | 高 | 標準ライブラリのみでの実装を検討 |
| API仕様変更 | 低 | 中 | APIバージョンを固定、定期的な確認 |
| 非同期処理の複雑性 | 中 | 中 | async-traitの使用、十分なテスト |
| レート制限への対応 | 中 | 中 | エクスポネンシャルバックオフの実装 |
| タイムアウトの調整 | 中 | 低 | WorkflowStepのtimeout設定を使用 |
| トークンカウントの精度 | 低 | 低 | プロバイダーの公式値を使用 |

---

## 9. パフォーマンス考察

### 9.1 ボトルネック予測

1. **ネットワークI/O**: API呼び出しが最大のボトルネック
2. **JSONパース**: レスポンスのデシリアライズ
3. **文字列操作**: プロンプトの構築

### 9.2 最適化ポイント

- HTTPコネクションの再利用（reqwest::Clientの再利用）
- 並行リクエスト処理（tokioのタスク並行実行）
- レスポンスサイズの制限（max_tokensの適切な設定）

---

## 10. セキュリティ考察

### 10.1 APIキー管理

**現状の問題点**:
- APIキーをどこに保存するか未定義

**推奨方法**:
- 環境変数から読み込み（`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`）
- APIキーをログに出力しない
- APIキーをバリデーション時に即座に確認

### 10.2 プロンプトインジェクション対策

**リスク**: ユーザー入力がシステムプロンプトを上書きする可能性

**対策**:
- システムプロンプトとユーザー入力を明確に分離
- Anthropic/OpenAIのメッセージ形式を正しく使用

---

## 11. 将来の拡張性

### 11.1 新しいプロバイダーの追加

**設計上の考慮**:
- `ProviderClient` トレイトを実装すれば、任意のプロバイダーを追加可能
- `Provider` 列挙体に新しいバリアントを追加
- `model_tier.rs` にマッピングを追加

**候補**:
- Google Vertex AI（Gemini）
- Cohere
- ローカルLLM（Ollama等）

### 11.2 高度な機能

**ストリーミング**:
- レスポンスをストリーミングで受け取る
- `async_stream` クレートの使用

**リトライロジック**:
- エクスポネンシャルバックオフ
- レート制限のハンドリング

**キャッシング**:
- 同じプロンプトへのレスポンスをキャッシュ
- コスト削減

---

## 12. 結論

### 12.1 実装の優先順位

1. **高優先度**: トレイト定義、エラー型、モデルマッピング
2. **中優先度**: Anthropic実装、OpenAI実装
3. **低優先度**: 高度なリトライロジック、ストリーミング

### 12.2 成功基準

- [ ] 両プロバイダーのクライアントが動作する
- [ ] モデルティアマッピングが全組み合わせで機能
- [ ] エラーハンドリングが適切
- [ ] テストカバレッジ70%以上
- [ ] ドキュメントが充実
- [ ] `cargo clippy` で警告なし

### 12.3 推定工数

**合計**: 13.5-20.5時間（1.5-2.5日相当）

**内訳**:
- 基盤実装（トレイト・エラー型）: 2-3時間
- モデルマッピング: 1-2時間
- Anthropic実装: 4-6時間
- OpenAI実装: 4-6時間
- 統合・テスト: 2-3時間

---

**分析完了**

本レポートは、providerモジュールの詳細実装に必要な全ての情報を提供しています。
実装計画書（implementation-plan.md）と併せて参照してください。
