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
//! - [`executor`][]: ワークフロー実行エンジン本体
//! - [`context`][]: ステップ実行コンテキスト（ステップ間データ受け渡し）
//! - [`result`][]: 実行結果型（ステップ&ワークフロー結果）
//!
//! # 使用例
//!
//! ```rust,no_run
//! use melted_adw::config::workflow::Workflow;
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

pub mod result;
pub mod context;
pub mod executor;

// 公開APIの再エクスポート
pub use result::{ExecutionError, ExecutionStatus, StepResult, StepStatus, WorkflowResult};
pub use context::{ExecutionContext, StepOutput};
pub use executor::WorkflowExecutor;
