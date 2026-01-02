//! ワークフロー実行エンジン
//!
//! # 責務
//!
//! このモジュールは、ワークフローの実行を制御する `WorkflowExecutor` を提供します。
//! Workflow 定義を受け取り、各ステップを順次実行し、ステップ間でデータを受け渡します。
//!
//! # 主要な型
//!
//! - [`WorkflowExecutor`][]: ワークフロー実行の中核となる構造体
//!
//! # 実行フロー
//!
//! 1. ワークフロー定義を受け取る
//! 2. 初期入力を設定（オプション）
//! 3. 各ステップを順次実行
//!    - プロバイダークライアントを生成
//!    - LLM を実行
//!    - 結果を記録
//!    - 次のステップへ出力を引き継ぐ
//! 4. 最終結果を返す
//!
//! # 使用例
//!
//! ```rust,no_run
//! use melted_adw::config::workflow::Workflow;
//! use melted_adw::engine::executor::WorkflowExecutor;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let workflow = Workflow::from_file("workflow.toml")?;
//!     let executor = WorkflowExecutor::new(workflow)
//!         .with_initial_input("タスク: 新機能の実装".to_string());
//!
//!     let result = executor.execute().await?;
//!
//!     if result.is_success() {
//!         println!("ワークフロー成功!");
//!         println!("総トークン使用量: {}", result.total_tokens_used);
//!     }
//!
//!     Ok(())
//! }
//! ```

use crate::config::workflow::Workflow;
use crate::config::step::WorkflowStep;
use crate::engine::context::{ExecutionContext, StepOutput};
use crate::engine::result::{WorkflowResult, StepResult, ExecutionStatus, StepStatus, ExecutionError};
use std::time::{SystemTime, Duration};

/// ワークフロー実行エンジン
///
/// Workflow 定義を受け取り、各ステップを順次実行します。
/// ステップ間のデータ受け渡しを自動的に処理し、実行結果を記録します。
///
/// # フィールド
///
/// - `workflow`: 実行するワークフロー定義
/// - `initial_input`: 最初のステップへの初期入力（オプション）
///
/// # 例
///
/// ```rust,no_run
/// use melted_adw::config::workflow::Workflow;
/// use melted_adw::engine::executor::WorkflowExecutor;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let workflow = Workflow::from_file("workflow.toml")?;
/// let executor = WorkflowExecutor::new(workflow);
/// let result = executor.execute().await?;
/// # Ok(())
/// # }
/// ```
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
    /// use melted_adw::config::workflow::Workflow;
    /// use melted_adw::engine::executor::WorkflowExecutor;
    ///
    /// let workflow = Workflow::from_file("workflow.toml").unwrap();
    /// let executor = WorkflowExecutor::new(workflow);
    /// ```
    pub fn new(workflow: Workflow) -> Self {
        Self {
            workflow,
            initial_input: None,
        }
    }

    /// 初期入力を設定
    ///
    /// 最初のステップに渡す入力を設定します。
    /// 設定しない場合、最初のステップにはシステムプロンプトのみが渡されます。
    ///
    /// # 引数
    ///
    /// - `input`: 初期入力文字列
    ///
    /// # 例
    ///
    /// ```rust,no_run
    /// use melted_adw::config::workflow::Workflow;
    /// use melted_adw::engine::executor::WorkflowExecutor;
    ///
    /// let workflow = Workflow::from_file("workflow.toml").unwrap();
    /// let executor = WorkflowExecutor::new(workflow)
    ///     .with_initial_input("初期入力データ".to_string());
    /// ```
    pub fn with_initial_input(mut self, input: String) -> Self {
        self.initial_input = Some(input);
        self
    }

    /// ワークフローを実行
    ///
    /// ワークフロー内の全ステップを順次実行し、結果を返します。
    /// 各ステップの出力は次のステップの入力として自動的に渡されます。
    ///
    /// # 戻り値
    ///
    /// - `Ok(WorkflowResult)`: 実行成功時、結果を返す
    /// - `Err(ExecutionError)`: 実行失敗時、エラーを返す
    ///
    /// # 例
    ///
    /// ```rust,no_run
    /// # use melted_adw::config::workflow::Workflow;
    /// # use melted_adw::engine::executor::WorkflowExecutor;
    /// # async fn example() {
    /// let workflow = Workflow::from_file("workflow.toml").unwrap();
    /// let executor = WorkflowExecutor::new(workflow);
    /// let result = executor.execute().await.unwrap();
    ///
    /// println!("Workflow completed: {}", result.is_success());
    /// # }
    /// ```
    pub async fn execute(&self) -> Result<WorkflowResult, ExecutionError> {
        let mut context = ExecutionContext::new(self.workflow.name().to_string());
        let mut step_results = Vec::new();
        let start_time = SystemTime::now();

        // 初期入力の設定
        let mut current_input = self.initial_input
            .clone()
            .unwrap_or_default();

        // 各ステップを順次実行（リトライ機能付き）
        for (index, step) in self.workflow.steps().iter().enumerate() {
            context.start_step(step.name());

            let step_result = self.execute_step_with_retry(
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

    /// 単一ステップを実行（プライベートメソッド）
    ///
    /// 指定されたステップを実行し、結果を返します。
    ///
    /// # 引数
    ///
    /// - `step`: 実行するステップ
    /// - `step_index`: ステップのインデックス（0始まり）
    /// - `user_input`: ステップへの入力
    /// - `context`: 実行コンテキスト
    ///
    /// # 戻り値
    ///
    /// - `Ok(StepResult)`: ステップ実行成功
    /// - `Err(ExecutionError)`: ステップ実行失敗
    async fn execute_step(
        &self,
        step: &WorkflowStep,
        step_index: usize,
        user_input: &str,
        context: &mut ExecutionContext,
    ) -> Result<StepResult, ExecutionError> {
        let step_start = SystemTime::now();

        // LLMを実行（タイムアウト付き）
        let response = self.execute_with_timeout(step, user_input).await?;

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

    /// リトライ機能付きでステップを実行（プライベートメソッド）
    ///
    /// ステップ設定に基づいて、失敗時に自動的にリトライします。
    /// リトライ間には1秒の待機時間を設けます。
    ///
    /// # 引数
    ///
    /// - `step`: 実行するステップ
    /// - `step_index`: ステップのインデックス（0始まり）
    /// - `user_input`: ステップへの入力
    /// - `context`: 実行コンテキスト
    ///
    /// # 戻り値
    ///
    /// - `Ok(StepResult)`: ステップ実行成功（リトライ後の成功も含む）
    /// - `Err(ExecutionError)`: すべてのリトライが失敗した場合
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

    /// タイムアウト付きでLLMを実行（プライベートメソッド）
    ///
    /// ステップにタイムアウト設定がある場合、指定時間内に完了しない場合はタイムアウトエラーを返します。
    ///
    /// # 引数
    ///
    /// - `step`: 実行するステップ
    /// - `user_input`: ステップへの入力
    ///
    /// # 戻り値
    ///
    /// - `Ok(ProviderResponse)`: LLM実行成功
    /// - `Err(ExecutionError)`: LLM実行失敗またはタイムアウト
    async fn execute_with_timeout(
        &self,
        step: &WorkflowStep,
        user_input: &str,
    ) -> Result<crate::provider::ProviderResponse, ExecutionError> {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::step::ModelTier;
    use crate::provider::{ProviderClient, ProviderResponse, TokenUsage};
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};

    /// モックプロバイダークライアント
    ///
    /// テスト用のモック実装。
    /// 実際のLLM APIを呼び出さずに、決められた応答を返します。
    #[allow(dead_code)]
    struct MockProviderClient {
        responses: Arc<Mutex<Vec<String>>>,
    }

    #[allow(dead_code)]
    impl MockProviderClient {
        fn new(responses: Vec<String>) -> Self {
            Self {
                responses: Arc::new(Mutex::new(responses)),
            }
        }
    }

    #[async_trait]
    impl ProviderClient for MockProviderClient {
        async fn execute(
            &self,
            _system_prompt: &str,
            user_input: &str,
            _model_tier: &ModelTier,
        ) -> Result<ProviderResponse, crate::error::ProviderError> {
            let mut responses = self.responses.lock().unwrap();
            let response_text = if !responses.is_empty() {
                responses.remove(0)
            } else {
                format!("Mock response for: {}", user_input)
            };

            Ok(ProviderResponse {
                content: response_text,
                token_usage: TokenUsage {
                    input_tokens: 100,
                    output_tokens: 50,
                },
                stop_reason: crate::provider::StopReason::EndTurn,
                model: "mock-model".to_string(),
            })
        }
    }

    /// テスト用のワークフローを作成するヘルパー関数
    fn create_test_workflow(step_count: usize) -> Workflow {
        // Create a minimal TOML string for testing
        let mut toml = String::from(
            "[workflow]\n\
             name = \"test_workflow\"\n\
             description = \"Test workflow\"\n\
             version = \"1.0.0\"\n\n"
        );

        for i in 0..step_count {
            toml.push_str(&format!(
                "[[steps]]\n\
                 name = \"step{}\"\n\
                 system_prompt = \"System prompt for step {}\"\n\
                 provider = \"anthropic\"\n\
                 model_tier = \"medium\"\n\n",
                i + 1, i + 1
            ));
        }

        Workflow::from_toml(&toml).unwrap()
    }

    #[tokio::test]
    async fn test_workflow_executor_single_step() {
        let workflow = create_test_workflow(1);
        let executor = WorkflowExecutor::new(workflow);

        // Note: This test will fail without mocking the provider factory
        // In a real implementation, we would need dependency injection
        // or a test-specific provider factory

        // For now, we're testing the structure and compilation
        assert_eq!(executor.workflow.name(), "test_workflow");
        assert_eq!(executor.workflow.steps().len(), 1);
        assert!(executor.initial_input.is_none());
    }

    #[tokio::test]
    async fn test_workflow_executor_with_initial_input() {
        let workflow = create_test_workflow(2);
        let executor = WorkflowExecutor::new(workflow)
            .with_initial_input("Initial input".to_string());

        assert_eq!(executor.initial_input, Some("Initial input".to_string()));
        assert_eq!(executor.workflow.steps().len(), 2);
    }

    #[test]
    fn test_workflow_executor_new() {
        let workflow = create_test_workflow(3);
        let executor = WorkflowExecutor::new(workflow);

        assert_eq!(executor.workflow.name(), "test_workflow");
        assert_eq!(executor.workflow.steps().len(), 3);
        assert!(executor.initial_input.is_none());
    }

    #[test]
    fn test_workflow_executor_builder_pattern() {
        let workflow = create_test_workflow(1);
        let executor = WorkflowExecutor::new(workflow)
            .with_initial_input("Test input".to_string());

        assert_eq!(executor.initial_input, Some("Test input".to_string()));
    }

    // Note: Full integration tests with mocked providers would require
    // additional infrastructure for dependency injection. These would test:
    // - Single step workflow execution
    // - Multiple step workflow execution
    // - Step-to-step data propagation
    // - Provider error handling
    //
    // Example structure for future implementation:
    //
    // #[tokio::test]
    // async fn test_execute_single_step_workflow() {
    //     let workflow = create_test_workflow(1);
    //     let executor = WorkflowExecutor::new(workflow)
    //         .with_provider_factory(mock_provider_factory);
    //
    //     let result = executor.execute().await.unwrap();
    //
    //     assert!(result.is_success());
    //     assert_eq!(result.steps.len(), 1);
    //     assert_eq!(result.workflow_name, "test_workflow");
    // }

    /// テスト用のリトライ設定付きワークフローを作成
    fn create_test_workflow_with_retry(retry_count: u32) -> Workflow {
        let toml = format!(
            "[workflow]\n\
             name = \"test_workflow_retry\"\n\
             description = \"Test workflow with retry\"\n\
             version = \"1.0.0\"\n\n\
             [[steps]]\n\
             name = \"step1\"\n\
             system_prompt = \"Test prompt\"\n\
             provider = \"anthropic\"\n\
             model_tier = \"medium\"\n\
             retry_count = {}\n",
            retry_count
        );

        Workflow::from_toml(&toml).unwrap()
    }

    /// テスト用のタイムアウト設定付きワークフローを作成
    fn create_test_workflow_with_timeout(timeout_secs: u64) -> Workflow {
        let toml = format!(
            "[workflow]\n\
             name = \"test_workflow_timeout\"\n\
             description = \"Test workflow with timeout\"\n\
             version = \"1.0.0\"\n\n\
             [[steps]]\n\
             name = \"step1\"\n\
             system_prompt = \"Test prompt\"\n\
             provider = \"anthropic\"\n\
             model_tier = \"medium\"\n\
             timeout = {}\n",
            timeout_secs
        );

        Workflow::from_toml(&toml).unwrap()
    }

    /// リトライ機能のテスト - ワークフローの構造確認
    #[test]
    fn test_workflow_with_retry_structure() {
        let workflow = create_test_workflow_with_retry(3);

        assert_eq!(workflow.name(), "test_workflow_retry");
        assert_eq!(workflow.steps().len(), 1);
        assert_eq!(workflow.steps()[0].retry_count(), Some(3));
    }

    /// タイムアウト機能のテスト - ワークフローの構造確認
    #[test]
    fn test_workflow_with_timeout_structure() {
        let workflow = create_test_workflow_with_timeout(60);

        assert_eq!(workflow.name(), "test_workflow_timeout");
        assert_eq!(workflow.steps().len(), 1);
        assert_eq!(workflow.steps()[0].timeout(), Some(60));
    }

    /// リトライとタイムアウトの組み合わせワークフローのテスト
    #[test]
    fn test_workflow_with_retry_and_timeout() {
        let toml =
            "[workflow]\n\
             name = \"test_workflow_combined\"\n\
             description = \"Test workflow with retry and timeout\"\n\
             version = \"1.0.0\"\n\n\
             [[steps]]\n\
             name = \"step1\"\n\
             system_prompt = \"Test prompt\"\n\
             provider = \"anthropic\"\n\
             model_tier = \"medium\"\n\
             retry_count = 2\n\
             timeout = 30\n";

        let workflow = Workflow::from_toml(toml).unwrap();

        assert_eq!(workflow.steps()[0].retry_count(), Some(2));
        assert_eq!(workflow.steps()[0].timeout(), Some(30));
    }

    // Note: The following tests demonstrate the expected behavior of retry and timeout features.
    // Actual integration tests with mocked providers that fail/timeout would require
    // dependency injection or a test-specific provider factory.
    //
    // Expected behaviors:
    //
    // #[tokio::test]
    // async fn test_retry_succeeds_after_failures() {
    //     // ワークフローは retry_count = 2 で設定
    //     // モックプロバイダーは最初の2回失敗、3回目に成功を返す
    //     // 結果: StepStatus::Retried { attempts: 2 }, retry_count = 2
    // }
    //
    // #[tokio::test]
    // async fn test_retry_fails_after_max_attempts() {
    //     // ワークフローは retry_count = 2 で設定
    //     // モックプロバイダーは常に失敗を返す
    //     // 結果: ExecutionError が返される（合計3回試行後）
    // }
    //
    // #[tokio::test]
    // async fn test_timeout_triggers() {
    //     // ワークフローは timeout = 2 で設定
    //     // モックプロバイダーは5秒待機してから応答
    //     // 結果: ExecutionError::TimeoutError
    // }
    //
    // #[tokio::test]
    // async fn test_timeout_does_not_trigger_for_fast_execution() {
    //     // ワークフローは timeout = 10 で設定
    //     // モックプロバイダーは即座に応答
    //     // 結果: StepStatus::Success, 正常完了
    // }
}
