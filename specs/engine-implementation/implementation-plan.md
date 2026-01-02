# Engine モジュール詳細実装計画書

**作成日**: 2026-01-02
**更新日**: 2026-01-02
**タスク名**: engine-implementation
**目的**: ワークフロー実行エンジンの実装

---

## エグゼクティブサマリー

本計画書は、configとproviderモジュールを統合し、ワークフローを実行するengineモジュールの実装を定義します。これにより：
- TOML形式で定義されたワークフローの自動実行
- ステップ間のデータ受け渡しによる連鎖実行
- プロバイダー（Anthropic/OpenAI）の抽象的な利用
- テレメトリー収集による実行分析

**主要な要件**:
1. 各ファイルにモジュール/ファイルの責務をrustdocで記述
2. 各ファイルにテストを記述し、デグレを防止
3. README.mdに記述されたengineモジュールの責務を満たす
4. LLM/人間が理解しやすいドキュメンテーション

**進捗状況**:
- ⏳ すべてのフェーズが実装待ち

**推定総工数**: 12-18時間（2-3日相当）
**推定総行数**: 900-1200行

---

## 目次

1. [アーキテクチャ概要](#1-アーキテクチャ概要)
2. [現状分析（As-Is）](#2-現状分析as-is)
3. [目標状態（To-Be）](#3-目標状態to-be)
4. [実装フェーズ](#4-実装フェーズ)
5. [成功基準](#5-成功基準)
6. [リスクと対策](#6-リスクと対策)

---

## 1. アーキテクチャ概要

### 1.1 全体アーキテクチャにおけるengineの位置づけ

```
┌─────────────────────────────────────────────────────────────────┐
│                         CLI / API                                │
├─────────────────────────────────────────────────────────────────┤
│                    Workflow Engine (engine)                      │
│  ┌───────────┐  ┌───────────┐  ┌───────────┐  ┌───────────┐    │
│  │  Step 1   │→ │  Step 2   │→ │  Step 3   │→ │  Step N   │    │
│  │ (計画)    │  │ (実装)    │  │ (レビュー) │  │   ...     │    │
│  └───────────┘  └───────────┘  └───────────┘  └───────────┘    │
├─────────────────────────────────────────────────────────────────┤
│                   Provider Abstraction Layer                     │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │   Anthropic     │  │     OpenAI      │  │    Future...    │  │
│  │  (Claude Code)  │  │    (Codex)      │  │                 │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### 1.2 モジュール間の連携フロー

```
1. Workflow読み込み
   CLI → config::Workflow::from_file("workflow.toml")

2. Engine初期化
   CLI → engine::WorkflowExecutor::new(workflow)

3. ワークフロー実行
   Engine → executor.execute()
     ├─ Step 1実行
     │   ├─ provider::create_provider(step.provider())
     │   ├─ client.execute(step.system_prompt(), user_input, step.model_tier())
     │   └─ context.record_step_result(result)
     ├─ Step 2実行 (Step 1の出力を入力として使用)
     │   └─ ...
     └─ Step N実行

4. 結果返却
   Engine → WorkflowResult
     ├─ telemetry.collect(summary)
     └─ CLI → 結果表示
```

### 1.3 engineモジュールの責務

**Workflow Execution**:
- Workflowオブジェクトを受け取り、各Stepを順序立てて実行
- タイムアウトとリトライの制御

**Data Propagation**:
- 前のステップの出力を次のステップの入力として自動受け渡し
- ステップ実行履歴の保持

**Provider Integration**:
- 各Stepの定義（provider, model_tier）に基づいてproviderモジュールを呼び出し
- プロバイダーエラーのハンドリング

**Telemetry Collection**:
- 各ステップのトークン使用量・実行時間を記録
- ワークフロー全体のサマリーを生成

---

## 2. 現状分析（As-Is）

### 2.1 ファイル状態

すべてのengineモジュールファイルが空（1行のみ）：

- `/home/metal/repos/melted-adw/src/engine.rs` - 空
- `/home/metal/repos/melted-adw/src/engine/executor.rs` - 空
- `/home/metal/repos/melted-adw/src/engine/context.rs` - 空
- `/home/metal/repos/melted-adw/src/engine/result.rs` - 空

### 2.2 依存モジュールの状態

**完了済み**:
- ✅ `config` モジュール - Workflow/WorkflowStep定義、TOML読み込み
- ✅ `provider` モジュール - ProviderClient trait、Anthropic/OpenAI実装
- ✅ `error` モジュール - ConfigError/ProviderError定義

**未完了**:
- ❌ `telemetry` モジュール - 実装未着手（engine実装後に着手予定）
- ❌ `cli` モジュール - 実装未着手（engine実装後に着手予定）

### 2.3 既存依存関係

**利用可能な依存関係**:
```toml
[dependencies]
tokio = { version = "1", features = ["process", "io-util", "rt", "time", "macros"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
async-trait = "0.1"
thiserror = "1.0"
```

**新規追加が不要な理由**:
- 非同期処理: `tokio` で対応
- エラー型: `thiserror` で対応
- JSONシリアライズ: `serde_json` で対応（テレメトリー用）

---

## 3. 目標状態（To-Be）

### 3.1 各ファイルの責務と公開API

#### 3.1.1 `src/engine.rs`

**責務**:
- engineモジュール全体の公開APIを定義
- サブモジュールの再エクスポート

**公開API**:
```rust
// モジュール定義
pub mod executor;
pub mod context;
pub mod result;

// 公開APIの再エクスポート
pub use executor::WorkflowExecutor;
pub use context::{ExecutionContext, StepOutput};
pub use result::{
    WorkflowResult, StepResult,
    ExecutionStatus, StepStatus,
    ExecutionError,
};
```

**推定行数**: 40-60行（ドキュメント含む）

---

#### 3.1.2 `src/engine/result.rs`

**責務**:
- ステップ実行結果の型定義
- ワークフロー実行結果の型定義
- 実行ステータスとエラーの型定義

**公開API**:
```rust
/// ワークフロー実行結果
#[derive(Debug, Clone, Serialize)]
pub struct WorkflowResult {
    pub workflow_name: String,
    pub status: ExecutionStatus,
    pub steps: Vec<StepResult>,
    pub start_time: SystemTime,
    pub end_time: SystemTime,
    pub total_duration: Duration,
    pub total_tokens_used: u32,
    pub error: Option<String>,  // ExecutionErrorのメッセージ
}

impl WorkflowResult {
    /// 結果をJSON形式でシリアライズ
    pub fn to_json(&self) -> Result<String, serde_json::Error>;

    /// 成功したかどうか
    pub fn is_success(&self) -> bool;

    /// 完了したステップ数
    pub fn completed_steps(&self) -> usize;
}

/// ステップ実行結果
#[derive(Debug, Clone, Serialize)]
pub struct StepResult {
    pub step_name: String,
    pub index: usize,
    pub status: StepStatus,
    pub output: Option<String>,
    pub token_usage: TokenUsage,
    pub duration: Duration,
    pub retry_count: u32,
    pub error: Option<String>,
}

/// ワークフロー実行ステータス
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ExecutionStatus {
    Success,
    PartialSuccess { completed: usize, total: usize },
    Failed,
}

/// ステップ実行ステータス
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum StepStatus {
    Success,
    Failed,
    Retried { attempts: u32 },
    Skipped,
}

/// 実行エラー
#[derive(Debug, Error)]
pub enum ExecutionError {
    #[error("設定エラー: {0}")]
    ConfigError(#[from] ConfigError),

    #[error("プロバイダーエラー: {0}")]
    ProviderError(#[from] ProviderError),

    #[error("タイムアウト: ステップ '{step_name}' が {timeout_secs}秒以内に完了しませんでした")]
    TimeoutError { step_name: String, timeout_secs: u64 },

    #[error("バリデーションエラー: {0}")]
    ValidationError(String),

    #[error("コンテキストエラー: {0}")]
    ContextError(String),
}
```

**推定行数**: 180-220行

---

#### 3.1.3 `src/engine/context.rs`

**責務**:
- ステップ実行の進行状況を追跡
- 各ステップの入力と出力を保持
- テレメトリー情報の累積

**公開API**:
```rust
/// ステップ実行コンテキスト
#[derive(Debug)]
pub struct ExecutionContext {
    workflow_name: String,
    start_time: SystemTime,

    // ステップ実行履歴
    steps_executed: Vec<String>,
    current_step: Option<String>,

    // ステップ間データ受け渡し
    step_outputs: Vec<StepOutput>,

    // テレメトリー情報
    total_tokens_used: u32,
    execution_times: Vec<Duration>,
    retry_counts: HashMap<String, u32>,
}

impl ExecutionContext {
    /// 新しいコンテキストを生成
    pub fn new(workflow_name: String) -> Self;

    /// ステップ実行開始
    pub fn start_step(&mut self, step_name: &str);

    /// ステップ完了と結果記録
    pub fn record_step_result(&mut self, output: StepOutput);

    /// 最後のステップの出力を取得
    pub fn get_last_output(&self) -> Option<&StepOutput>;

    /// 特定のステップ出力を取得
    pub fn get_step_output(&self, step_name: &str) -> Option<&StepOutput>;

    /// リトライカウントを増加
    pub fn increment_retry(&mut self, step_name: &str);

    /// リトライカウントを取得
    pub fn get_retry_count(&self, step_name: &str) -> u32;

    /// 総トークン使用量を取得
    pub fn total_tokens(&self) -> u32;

    /// 総実行時間を取得
    pub fn total_duration(&self) -> Duration;
}

/// ステップ出力
#[derive(Debug, Clone)]
pub struct StepOutput {
    pub step_name: String,
    pub content: String,
    pub token_usage: TokenUsage,
    pub execution_time: Duration,
}

impl StepOutput {
    /// 新しいステップ出力を生成
    pub fn new(
        step_name: String,
        content: String,
        token_usage: TokenUsage,
        execution_time: Duration,
    ) -> Self;
}
```

**推定行数**: 220-280行

---

#### 3.1.4 `src/engine/executor.rs`

**責務**:
- Workflowオブジェクトを受け取り、各Stepを順序立てて実行
- 各Stepの定義に基づいてproviderを呼び出し
- タイムアウトとリトライの制御
- 実行過程をExecutionContextに記録

**公開API**:
```rust
/// ワークフロー実行エンジン
pub struct WorkflowExecutor {
    workflow: Workflow,
    initial_input: Option<String>,
}

impl WorkflowExecutor {
    /// 新しいエグゼキューターを生成
    ///
    /// # 引数
    ///
    /// - `workflow`: 実行するワークフロー定義
    ///
    /// # 例
    ///
    /// ```rust,no_run
    /// use melted_adw::config::Workflow;
    /// use melted_adw::engine::WorkflowExecutor;
    ///
    /// let workflow = Workflow::from_file("workflow.toml").unwrap();
    /// let executor = WorkflowExecutor::new(workflow);
    /// ```
    pub fn new(workflow: Workflow) -> Self;

    /// 初期入力を設定
    ///
    /// 最初のステップに渡す入力を設定します。
    /// 設定しない場合、最初のステップにはシステムプロンプトのみが渡されます。
    ///
    /// # 引数
    ///
    /// - `input`: 初期入力文字列
    pub fn with_initial_input(mut self, input: String) -> Self;

    /// ワークフローを実行
    ///
    /// # 戻り値
    ///
    /// - `Ok(WorkflowResult)`: 実行成功時、結果を返す
    /// - `Err(ExecutionError)`: 実行失敗時、エラーを返す
    ///
    /// # 例
    ///
    /// ```rust,no_run
    /// # use melted_adw::config::Workflow;
    /// # use melted_adw::engine::WorkflowExecutor;
    /// # async fn example() {
    /// let workflow = Workflow::from_file("workflow.toml").unwrap();
    /// let executor = WorkflowExecutor::new(workflow);
    /// let result = executor.execute().await.unwrap();
    ///
    /// println!("Workflow completed: {}", result.is_success());
    /// # }
    /// ```
    pub async fn execute(&self) -> Result<WorkflowResult, ExecutionError>;
}

// 内部実装（非公開）
impl WorkflowExecutor {
    /// 単一ステップを実行
    async fn execute_step(
        &self,
        step: &WorkflowStep,
        step_index: usize,
        user_input: &str,
        context: &mut ExecutionContext,
    ) -> Result<StepResult, ExecutionError>;

    /// リトライロジックを持つステップ実行
    async fn execute_step_with_retry(
        &self,
        step: &WorkflowStep,
        step_index: usize,
        user_input: &str,
        context: &mut ExecutionContext,
    ) -> Result<StepResult, ExecutionError>;

    /// タイムアウト制御付きステップ実行
    async fn execute_with_timeout(
        &self,
        step: &WorkflowStep,
        user_input: &str,
    ) -> Result<ProviderResponse, ExecutionError>;
}
```

**推定行数**: 350-450行

---

### 3.2 データ構造の全体像

```
┌─────────────────────────────────────┐
│         engine.rs                   │
│  - モジュール定義                    │
│  - 公開API再エクスポート             │
└──────────────┬──────────────────────┘
               │
       ┌───────┴───────┬──────────┐
       ↓               ↓          ↓
┌─────────────┐ ┌─────────────┐ ┌──────────┐
│  result.rs  │ │ context.rs  │ │executor  │
│             │ │             │ │  .rs     │
├─────────────┤ ├─────────────┤ ├──────────┤
│WorkflowResult│ │Execution   │ │Workflow  │
│             │ │Context     │ │Executor  │
│StepResult   │ │             │ │          │
│             │ │StepOutput   │ │          │
│Execution    │ │             │ │          │
│Status       │ │             │ │          │
│             │ │             │ │          │
│StepStatus   │ │             │ │          │
│             │ │             │ │          │
│Execution    │ │             │ │          │
│Error        │ │             │ │          │
└─────────────┘ └─────────────┘ └──────────┘
       ↓               ↓          ↓
       使用            使用        使用
       ↓               ↓          ↓
┌─────────────────┐ ┌──────────┐
│   config        │ │ provider │
│   (Workflow)    │ │ (Client) │
└─────────────────┘ └──────────┘
```

---

## 4. 実装フェーズ

### フェーズ1: 型定義とエラーハンドリング

**目的**: result.rsの実装（基盤となる型定義）

**Context Window考慮**: 低（180-220行、1ファイル）

#### タスク1.1: `src/engine/result.rs` の実装

**実装内容**:
- WorkflowResult, StepResult 構造体
- ExecutionStatus, StepStatus 列挙型
- ExecutionError エラー型
- Serialize実装（テレメトリー用）
- 単体テスト

**依存関係**:
- `use crate::error::{ConfigError, ProviderError};`
- `use crate::provider::TokenUsage;`
- `use serde::Serialize;`
- `use std::time::{Duration, SystemTime};`

**テストケース**:
- WorkflowResult生成とJSON変換
- ExecutionStatusの判定ロジック
- エラーメッセージのフォーマット

**検証方法**:
```bash
cargo build
cargo test result
cargo clippy -- -D warnings
```

**推定時間**: 2-3時間

---

### フェーズ2: コンテキスト管理

**目的**: context.rsの実装（ステップ間データ受け渡し）

**Context Window考慮**: 中（220-280行、1ファイル）

#### タスク2.1: `src/engine/context.rs` の実装

**実装内容**:
- ExecutionContext 構造体
- StepOutput 構造体
- ステップ履歴管理
- テレメトリー情報累積
- 単体テスト

**依存関係**:
- `use crate::provider::TokenUsage;`
- `use std::time::{Duration, SystemTime};`
- `use std::collections::HashMap;`

**主要ロジック**:
```rust
// 最後のステップの出力を取得（次のステップの入力として使用）
pub fn get_last_output(&self) -> Option<&StepOutput> {
    self.step_outputs.last()
}

// リトライカウントの管理
pub fn increment_retry(&mut self, step_name: &str) {
    *self.retry_counts.entry(step_name.to_string()).or_insert(0) += 1;
}
```

**テストケース**:
- ステップ出力の記録と取得
- リトライカウントの増加
- 総トークン/総時間の集計
- 複数ステップの履歴管理

**検証方法**:
```bash
cargo build
cargo test context
cargo clippy -- -D warnings
```

**推定時間**: 3-4時間

---

### フェーズ3: エグゼキューター実装（基本）

**目的**: executor.rsの基本実装（リトライ・タイムアウトなし）

**Context Window考慮**: 高（200-250行、1ファイル - シンプル版）

#### タスク3.1: `src/engine/executor.rs` の基本実装

**実装内容**:
- WorkflowExecutor 構造体
- new(), with_initial_input()
- execute() - ステップを順次実行
- execute_step() - 単一ステップ実行
- 単体テスト（モックプロバイダー使用）

**主要ロジック**:
```rust
pub async fn execute(&self) -> Result<WorkflowResult, ExecutionError> {
    let mut context = ExecutionContext::new(self.workflow.name().to_string());
    let mut step_results = Vec::new();
    let start_time = SystemTime::now();

    // 初期入力の設定
    let mut current_input = self.initial_input
        .clone()
        .unwrap_or_else(|| String::new());

    // 各ステップを順次実行
    for (index, step) in self.workflow.steps().iter().enumerate() {
        context.start_step(step.name());

        let step_result = self.execute_step(
            step,
            index,
            &current_input,
            &mut context,
        ).await?;

        // 次のステップの入力として設定
        if let Some(output) = &step_result.output {
            current_input = output.clone();
        }

        step_results.push(step_result);
    }

    // 結果をまとめる
    let end_time = SystemTime::now();
    let total_duration = end_time.duration_since(start_time)
        .unwrap_or(Duration::from_secs(0));

    Ok(WorkflowResult {
        workflow_name: self.workflow.name().to_string(),
        status: ExecutionStatus::Success,
        steps: step_results,
        start_time,
        end_time,
        total_duration,
        total_tokens_used: context.total_tokens(),
        error: None,
    })
}

async fn execute_step(
    &self,
    step: &WorkflowStep,
    step_index: usize,
    user_input: &str,
    context: &mut ExecutionContext,
) -> Result<StepResult, ExecutionError> {
    let step_start = SystemTime::now();

    // プロバイダークライアントを生成
    let client = crate::provider::create_provider(step.provider())?;

    // LLMを実行
    let response = client.execute(
        step.system_prompt(),
        user_input,
        step.model_tier(),
    ).await?;

    let step_end = SystemTime::now();
    let duration = step_end.duration_since(step_start)
        .unwrap_or(Duration::from_secs(0));

    // コンテキストに記録
    context.record_step_result(StepOutput {
        step_name: step.name().to_string(),
        content: response.content.clone(),
        token_usage: response.token_usage,
        execution_time: duration,
    });

    Ok(StepResult {
        step_name: step.name().to_string(),
        index: step_index,
        status: StepStatus::Success,
        output: Some(response.content),
        token_usage: response.token_usage,
        duration,
        retry_count: 0,
        error: None,
    })
}
```

**テストケース**:
- 単一ステップのワークフロー実行
- 複数ステップのワークフロー実行
- ステップ間データ受け渡しの確認
- プロバイダーエラーのハンドリング

**検証方法**:
```bash
cargo build
cargo test executor::tests::basic
cargo clippy -- -D warnings
```

**推定時間**: 3-4時間

---

### フェーズ4: エグゼキューター実装（拡張）

**目的**: リトライ・タイムアウト機能の追加

**Context Window考慮**: 中（150-200行追加、1ファイル）

#### タスク4.1: リトライ機能の実装

**実装内容**:
```rust
async fn execute_step_with_retry(
    &self,
    step: &WorkflowStep,
    step_index: usize,
    user_input: &str,
    context: &mut ExecutionContext,
) -> Result<StepResult, ExecutionError> {
    let max_retries = step.retry_count().unwrap_or(0);
    let mut last_error = None;

    for attempt in 0..=max_retries {
        if attempt > 0 {
            context.increment_retry(step.name());
        }

        match self.execute_step(step, step_index, user_input, context).await {
            Ok(mut result) => {
                if attempt > 0 {
                    result.status = StepStatus::Retried { attempts: attempt };
                    result.retry_count = attempt;
                }
                return Ok(result);
            }
            Err(e) => {
                last_error = Some(e);
                // リトライ前に少し待機
                if attempt < max_retries {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }

    // すべてのリトライが失敗
    Err(last_error.unwrap())
}
```

#### タスク4.2: タイムアウト機能の実装

**実装内容**:
```rust
async fn execute_with_timeout(
    &self,
    step: &WorkflowStep,
    user_input: &str,
) -> Result<ProviderResponse, ExecutionError> {
    let client = crate::provider::create_provider(step.provider())?;

    if let Some(timeout_secs) = step.timeout() {
        // タイムアウト付き実行
        let timeout_duration = Duration::from_secs(timeout_secs);

        match tokio::time::timeout(
            timeout_duration,
            client.execute(step.system_prompt(), user_input, step.model_tier())
        ).await {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(e)) => Err(ExecutionError::ProviderError(e)),
            Err(_) => Err(ExecutionError::TimeoutError {
                step_name: step.name().to_string(),
                timeout_secs,
            }),
        }
    } else {
        // タイムアウトなし実行
        client.execute(step.system_prompt(), user_input, step.model_tier())
            .await
            .map_err(ExecutionError::ProviderError)
    }
}
```

#### タスク4.3: execute()の更新

execute_step_with_retry()を使用するように変更し、タイムアウトを統合。

**テストケース**:
- リトライが成功するケース
- リトライが最大回数まで失敗するケース
- タイムアウトが発動するケース
- タイムアウトなしで正常完了するケース

**検証方法**:
```bash
cargo build
cargo test executor
cargo clippy -- -D warnings
```

**推定時間**: 3-4時間

---

### フェーズ5: モジュール統合と検証

**目的**: engine.rsの実装と全体統合

**Context Window考慮**: 低（40-60行、1ファイル + 統合テスト）

#### タスク5.1: `src/engine.rs` の実装

**実装内容**:
```rust
//! ワークフロー実行エンジン
//!
//! # 責務
//!
//! - Workflowオブジェクトを受け取り、各Stepを順序立てて実行
//! - ステップ間のデータ受け渡しによる連鎖実行
//! - プロバイダー（Anthropic/OpenAI）の抽象的な利用
//! - タイムアウトとリトライの制御
//! - テレメトリー収集のためのデータ記録
//!
//! # モジュール構成
//!
//! - [`executor`]: ワークフロー実行エンジン本体
//! - [`context`]: ステップ実行コンテキスト（ステップ間データ受け渡し）
//! - [`result`]: 実行結果型（ステップ&ワークフロー結果）
//!
//! # 使用例
//!
//! ```rust,no_run
//! use melted_adw::config::Workflow;
//! use melted_adw::engine::WorkflowExecutor;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // 1. Workflowを読み込む
//!     let workflow = Workflow::from_file("workflows/example.toml")?;
//!
//!     // 2. Executorを生成
//!     let executor = WorkflowExecutor::new(workflow)
//!         .with_initial_input("新しい認証機能を実装してください".to_string());
//!
//!     // 3. ワークフローを実行
//!     let result = executor.execute().await?;
//!
//!     // 4. 結果を出力
//!     println!("Workflow: {}", result.workflow_name);
//!     println!("Status: {:?}", result.status);
//!     println!("Total tokens: {}", result.total_tokens_used);
//!     println!("Duration: {:?}", result.total_duration);
//!
//!     for step_result in &result.steps {
//!         println!("  Step {}: {:?}", step_result.step_name, step_result.status);
//!     }
//!
//!     Ok(())
//! }
//! ```

pub mod executor;
pub mod context;
pub mod result;

// 公開APIの再エクスポート
pub use executor::WorkflowExecutor;
pub use context::{ExecutionContext, StepOutput};
pub use result::{
    WorkflowResult, StepResult,
    ExecutionStatus, StepStatus,
    ExecutionError,
};
```

#### タスク5.2: `src/lib.rs` の更新

```rust
pub mod config;
pub mod error;
pub mod provider;
pub mod engine;  // 追加
```

#### タスク5.3: 統合テスト

**テストファイル**: `tests/integration/workflow_execution.rs`

**テストケース**:
- 実際のワークフローファイルを読み込んで実行
- 3ステップのワークフロー（plan → implement → review）
- エラーハンドリング（プロバイダーエラー、タイムアウト）
- リトライの動作確認

**検証方法**:
```bash
cargo clean
cargo build --release
cargo test --all
cargo clippy --all-targets -- -D warnings
cargo doc --no-deps --open
```

**推定時間**: 2-3時間

---

## 5. 成功基準

### 5.1 機能要件

- [ ] すべてのファイルにモジュール/ファイルの責務がrustdocで記述されている
- [ ] すべてのファイルにテストが記述されている
- [ ] README.mdに記述されたengineモジュールの責務が満たせている
  - [ ] ステップ実行ロジック（executor.rs）
  - [ ] 実行コンテキスト・ステップ間データ受け渡し（context.rs）
  - [ ] 実行結果（result.rs）
- [ ] Workflowオブジェクトから各Stepを順次実行できる
- [ ] ステップ間でデータが正しく受け渡される
- [ ] タイムアウト機能が動作する
- [ ] リトライ機能が動作する
- [ ] プロバイダーエラーが適切にハンドリングされる

### 5.2 品質要件

- [ ] `cargo build --release` が警告なく成功
- [ ] `cargo test --all` が100%成功
- [ ] `cargo clippy --all-targets -- -D warnings` が警告ゼロ
- [ ] `cargo doc --no-deps` でドキュメントが生成される
- [ ] すべての公開APIにドキュメントコメントが存在
- [ ] 各モジュールに使用例が記載されている

### 5.3 ドキュメント要件

- [ ] 各ファイルにファイルレベルドキュメント（`//!`）が存在
- [ ] 責務セクション（`# 責務`）が記述されている
- [ ] 主要機能の説明が記述されている
- [ ] 公開API（`pub`）にドキュメントコメント（`///`）が存在
- [ ] 引数・戻り値・エラーが説明されている
- [ ] 使用例（`# 例`）が記載されている

---

## 6. リスクと対策

### 6.1 依存関係追加の制約

**リスク**: CLAUDE.mdに「原則crateの追加は禁止」とある

**対策**:
- 既存の依存関係（tokio, serde_json, thiserror）のみで実装可能
- 新規依存関係の追加は不要

### 6.2 テレメトリーモジュールの未実装

**リスク**: telemetryモジュールがまだ実装されていない

**対策**:
- ExecutionContextとWorkflowResultにテレメトリーデータを保持
- telemetryモジュールの実装時に簡単に統合できる設計
- 現時点ではJSON出力のみで対応可能

### 6.3 CLIモジュールの未実装

**リスク**: CLIからの呼び出し方法が未確定

**対策**:
- WorkflowExecutorの公開APIを汎用的に設計
- CLIだけでなくライブラリとしても使用可能
- ドキュメントに使用例を明記

### 6.4 プロバイダーのモック化

**リスク**: テスト時に実際のLLMを呼び出すとコストと時間がかかる

**対策**:
- ProviderClientトレイトを利用してモック実装を作成
- 単体テストではモックプロバイダーを使用
- 統合テストは実際のCLIツールを使用（オプショナル）

### 6.5 エラーハンドリングの複雑化

**リスク**: ConfigError, ProviderError, ExecutionErrorの変換が複雑

**対策**:
- ExecutionErrorに`#[from]`属性を使用して自動変換
- 各エラー型に明確なメッセージを含める
- ドキュメントにエラーの種類を明記

### 6.6 Context Window超過

**リスク**: フェーズ3/4で大量のコードを扱う

**対策**:
- フェーズを適切に分割（基本実装と拡張実装を分離）
- 実装の詳細をこの計画書に記載
- 必要最小限のファイルのみを参照

---

## 7. 実装順序のサマリー

```
フェーズ1: 型定義 (2-3時間)
   └─ result.rs: WorkflowResult, StepResult, エラー型

フェーズ2: コンテキスト管理 (3-4時間)
   └─ context.rs: ExecutionContext, StepOutput

フェーズ3: 基本実装 (3-4時間)
   └─ executor.rs: WorkflowExecutor基本機能

フェーズ4: 拡張機能 (3-4時間)
   └─ executor.rs: リトライ・タイムアウト

フェーズ5: 統合・検証 (2-3時間)
   ├─ engine.rs: モジュール定義
   ├─ lib.rs更新
   ├─ 統合テスト
   └─ ドキュメント確認

推定総時間: 12-18時間（2-3日相当）
推定総行数: 900-1200行
```

---

## 8. 参考資料

### 8.1 既存ドキュメント

- `/home/metal/repos/melted-adw/README.md`: プロジェクト概要、アーキテクチャ図
- `/home/metal/repos/melted-adw/specs/provider-implementation/detailed-implementation-plan.md`: provider実装計画（参考パターン）
- `/home/metal/repos/melted-adw/specs/config-implementation/implementation-plan.md`: config実装計画（参考パターン）

### 8.2 実装済みモジュール

- `/home/metal/repos/melted-adw/src/config/workflow.rs`: Workflow構造体、DTO/ドメインモデル分離パターン
- `/home/metal/repos/melted-adw/src/config/step.rs`: WorkflowStep構造体、Provider/ModelTier定義
- `/home/metal/repos/melted-adw/src/provider/traits.rs`: ProviderClientトレイト、ProviderResponse
- `/home/metal/repos/melted-adw/src/provider.rs`: create_provider()ファクトリー関数

### 8.3 サンプルワークフロー

- `/home/metal/repos/melted-adw/workflows/`: ワークフロー定義例（実装時に参照）

---

**計画書作成日**: 2026-01-02
**最終更新日**: 2026-01-02
**バージョン**: 1.0
