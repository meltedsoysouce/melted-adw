//! ワークフロー実行結果の型定義
//!
//! # 責務
//!
//! - ステップ実行結果 [`StepResult`] の型定義
//! - ワークフロー実行結果 [`WorkflowResult`] の型定義
//! - 実行ステータス [`ExecutionStatus`] と [`StepStatus`] の型定義
//! - 実行エラー [`ExecutionError`] の型定義
//!
//! # 主要な型
//!
//! - [`WorkflowResult`][]: ワークフロー全体の実行結果（成功/失敗、各ステップの結果、トークン使用量等）
//! - [`StepResult`][]: 個別ステップの実行結果（出力、トークン使用量、リトライ回数等）
//! - [`ExecutionStatus`][]: ワークフロー全体の実行ステータス（成功/部分成功/失敗）
//! - [`StepStatus`][]: 個別ステップの実行ステータス（成功/失敗/リトライ/スキップ）
//! - [`ExecutionError`][]: ワークフロー実行時のエラー型
//!
//! # 使用例
//!
//! ```rust,no_run
//! use melted_adw::engine::result::{WorkflowResult, ExecutionStatus};
//!
//! fn handle_result(result: WorkflowResult) {
//!     if result.is_success() {
//!         println!("ワークフロー成功: {}", result.workflow_name);
//!         println!("完了ステップ数: {}/{}", result.completed_steps(), result.steps.len());
//!         println!("総トークン使用量: {}", result.total_tokens_used);
//!         println!("実行時間: {:?}", result.total_duration);
//!     } else {
//!         println!("ワークフロー失敗: {:?}", result.error);
//!     }
//!
//!     // JSON形式で出力
//!     if let Ok(json) = result.to_json() {
//!         println!("JSON: {}", json);
//!     }
//! }
//! ```

use crate::error::{ConfigError, ProviderError};
use crate::provider::TokenUsage;
use serde::Serialize;
use std::time::{Duration, SystemTime};
use thiserror::Error;

/// ワークフロー実行結果
///
/// ワークフロー全体の実行結果を表す型です。
/// 各ステップの実行結果、実行時間、トークン使用量などを含みます。
///
/// # 例
///
/// ```rust,no_run
/// use melted_adw::engine::result::WorkflowResult;
///
/// fn analyze_result(result: WorkflowResult) {
///     println!("ワークフロー: {}", result.workflow_name);
///     println!("ステータス: {:?}", result.status);
///
///     for step_result in &result.steps {
///         println!("  ステップ {}: {:?}", step_result.step_name, step_result.status);
///     }
/// }
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct WorkflowResult {
    /// ワークフロー名
    pub workflow_name: String,

    /// 実行ステータス
    pub status: ExecutionStatus,

    /// 各ステップの実行結果
    pub steps: Vec<StepResult>,

    /// 実行開始時刻
    pub start_time: SystemTime,

    /// 実行終了時刻
    pub end_time: SystemTime,

    /// 総実行時間
    pub total_duration: Duration,

    /// 総トークン使用量
    pub total_tokens_used: u32,

    /// エラーメッセージ（失敗時のみ）
    pub error: Option<String>,
}

impl WorkflowResult {
    /// 結果をJSON形式でシリアライズ
    ///
    /// # 戻り値
    ///
    /// - `Ok(String)`: JSON文字列
    /// - `Err(serde_json::Error)`: シリアライズ失敗
    ///
    /// # 例
    ///
    /// ```rust,no_run
    /// # use melted_adw::engine::result::{WorkflowResult, ExecutionStatus};
    /// # use std::time::{SystemTime, Duration};
    /// # let result = WorkflowResult {
    /// #     workflow_name: "test".to_string(),
    /// #     status: ExecutionStatus::Success,
    /// #     steps: vec![],
    /// #     start_time: SystemTime::now(),
    /// #     end_time: SystemTime::now(),
    /// #     total_duration: Duration::from_secs(1),
    /// #     total_tokens_used: 100,
    /// #     error: None,
    /// # };
    /// let json = result.to_json().unwrap();
    /// println!("結果: {}", json);
    /// ```
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// 成功したかどうか
    ///
    /// # 戻り値
    ///
    /// - `true`: ステータスが [`ExecutionStatus::Success`]
    /// - `false`: それ以外
    pub fn is_success(&self) -> bool {
        matches!(self.status, ExecutionStatus::Success)
    }

    /// 完了したステップ数
    ///
    /// # 戻り値
    ///
    /// [`StepStatus::Success`] または [`StepStatus::Retried`] のステップ数
    ///
    /// # 例
    ///
    /// ```rust,no_run
    /// # use melted_adw::engine::result::{WorkflowResult, ExecutionStatus};
    /// # use std::time::{SystemTime, Duration};
    /// # let result = WorkflowResult {
    /// #     workflow_name: "test".to_string(),
    /// #     status: ExecutionStatus::Success,
    /// #     steps: vec![],
    /// #     start_time: SystemTime::now(),
    /// #     end_time: SystemTime::now(),
    /// #     total_duration: Duration::from_secs(1),
    /// #     total_tokens_used: 100,
    /// #     error: None,
    /// # };
    /// println!("完了ステップ: {}/{}", result.completed_steps(), result.steps.len());
    /// ```
    pub fn completed_steps(&self) -> usize {
        self.steps
            .iter()
            .filter(|step| {
                matches!(step.status, StepStatus::Success | StepStatus::Retried { .. })
            })
            .count()
    }
}

/// ステップ実行結果
///
/// 個別のステップの実行結果を表す型です。
/// 出力、トークン使用量、実行時間、リトライ回数などを含みます。
#[derive(Debug, Clone, Serialize)]
pub struct StepResult {
    /// ステップ名
    pub step_name: String,

    /// ステップインデックス（0始まり）
    pub index: usize,

    /// 実行ステータス
    pub status: StepStatus,

    /// LLMの出力（成功時のみ）
    pub output: Option<String>,

    /// トークン使用量
    pub token_usage: TokenUsage,

    /// 実行時間
    pub duration: Duration,

    /// リトライ回数
    pub retry_count: u32,

    /// エラーメッセージ（失敗時のみ）
    pub error: Option<String>,
}

/// ワークフロー実行ステータス
///
/// ワークフロー全体の実行結果を表します。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ExecutionStatus {
    /// すべてのステップが成功
    Success,

    /// 一部のステップのみ成功
    PartialSuccess {
        /// 完了したステップ数
        completed: usize,
        /// 総ステップ数
        total: usize,
    },

    /// ワークフロー失敗
    Failed,
}

/// ステップ実行ステータス
///
/// 個別のステップの実行結果を表します。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum StepStatus {
    /// 成功（リトライなし）
    Success,

    /// 失敗
    Failed,

    /// リトライ後に成功
    Retried {
        /// リトライ回数
        attempts: u32,
    },

    /// スキップ（前ステップの失敗により未実行）
    Skipped,
}

/// 実行エラー
///
/// ワークフロー実行時に発生する可能性のあるエラーを表します。
///
/// # エラー種別
///
/// - [`ExecutionError::ConfigError`] - 設定エラー（ワークフロー定義の不備等）
/// - [`ExecutionError::ProviderError`] - プロバイダーエラー（LLM通信失敗等）
/// - [`ExecutionError::TimeoutError`] - タイムアウト（ステップが時間内に完了しない）
/// - [`ExecutionError::ValidationError`] - バリデーションエラー（入力値の不備等）
/// - [`ExecutionError::ContextError`] - コンテキストエラー（ステップ間データ受け渡しの失敗等）
#[derive(Debug, Error)]
#[allow(clippy::enum_variant_names)]
pub enum ExecutionError {
    /// 設定エラー
    #[error("設定エラー: {0}")]
    ConfigError(#[from] ConfigError),

    /// プロバイダーエラー
    #[error("プロバイダーエラー: {0}")]
    ProviderError(#[from] ProviderError),

    /// タイムアウト
    #[error("タイムアウト: ステップ '{step_name}' が {timeout_secs}秒以内に完了しませんでした")]
    TimeoutError {
        /// タイムアウトしたステップ名
        step_name: String,
        /// タイムアウト時間（秒）
        timeout_secs: u64,
    },

    /// バリデーションエラー
    #[error("バリデーションエラー: {0}")]
    ValidationError(String),

    /// コンテキストエラー
    #[error("コンテキストエラー: {0}")]
    ContextError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_result_is_success() {
        let result = WorkflowResult {
            workflow_name: "test_workflow".to_string(),
            status: ExecutionStatus::Success,
            steps: vec![],
            start_time: SystemTime::now(),
            end_time: SystemTime::now(),
            total_duration: Duration::from_secs(10),
            total_tokens_used: 1000,
            error: None,
        };

        assert!(result.is_success());
    }

    #[test]
    fn test_workflow_result_is_not_success_when_failed() {
        let result = WorkflowResult {
            workflow_name: "test_workflow".to_string(),
            status: ExecutionStatus::Failed,
            steps: vec![],
            start_time: SystemTime::now(),
            end_time: SystemTime::now(),
            total_duration: Duration::from_secs(5),
            total_tokens_used: 500,
            error: Some("エラーが発生しました".to_string()),
        };

        assert!(!result.is_success());
    }

    #[test]
    fn test_workflow_result_is_not_success_when_partial() {
        let result = WorkflowResult {
            workflow_name: "test_workflow".to_string(),
            status: ExecutionStatus::PartialSuccess {
                completed: 2,
                total: 3,
            },
            steps: vec![],
            start_time: SystemTime::now(),
            end_time: SystemTime::now(),
            total_duration: Duration::from_secs(15),
            total_tokens_used: 1500,
            error: None,
        };

        assert!(!result.is_success());
    }

    #[test]
    fn test_completed_steps_count() {
        let result = WorkflowResult {
            workflow_name: "test_workflow".to_string(),
            status: ExecutionStatus::PartialSuccess {
                completed: 2,
                total: 4,
            },
            steps: vec![
                StepResult {
                    step_name: "step1".to_string(),
                    index: 0,
                    status: StepStatus::Success,
                    output: Some("output1".to_string()),
                    token_usage: TokenUsage {
                        input_tokens: 100,
                        output_tokens: 200,
                    },
                    duration: Duration::from_secs(5),
                    retry_count: 0,
                    error: None,
                },
                StepResult {
                    step_name: "step2".to_string(),
                    index: 1,
                    status: StepStatus::Retried { attempts: 2 },
                    output: Some("output2".to_string()),
                    token_usage: TokenUsage {
                        input_tokens: 150,
                        output_tokens: 250,
                    },
                    duration: Duration::from_secs(10),
                    retry_count: 2,
                    error: None,
                },
                StepResult {
                    step_name: "step3".to_string(),
                    index: 2,
                    status: StepStatus::Failed,
                    output: None,
                    token_usage: TokenUsage {
                        input_tokens: 100,
                        output_tokens: 0,
                    },
                    duration: Duration::from_secs(2),
                    retry_count: 3,
                    error: Some("実行エラー".to_string()),
                },
                StepResult {
                    step_name: "step4".to_string(),
                    index: 3,
                    status: StepStatus::Skipped,
                    output: None,
                    token_usage: TokenUsage {
                        input_tokens: 0,
                        output_tokens: 0,
                    },
                    duration: Duration::from_secs(0),
                    retry_count: 0,
                    error: None,
                },
            ],
            start_time: SystemTime::now(),
            end_time: SystemTime::now(),
            total_duration: Duration::from_secs(17),
            total_tokens_used: 800,
            error: None,
        };

        // Success + Retried のステップ数
        assert_eq!(result.completed_steps(), 2);
    }

    #[test]
    fn test_workflow_result_to_json() {
        let result = WorkflowResult {
            workflow_name: "test_workflow".to_string(),
            status: ExecutionStatus::Success,
            steps: vec![StepResult {
                step_name: "step1".to_string(),
                index: 0,
                status: StepStatus::Success,
                output: Some("output".to_string()),
                token_usage: TokenUsage {
                    input_tokens: 100,
                    output_tokens: 200,
                },
                duration: Duration::from_secs(5),
                retry_count: 0,
                error: None,
            }],
            start_time: SystemTime::now(),
            end_time: SystemTime::now(),
            total_duration: Duration::from_secs(5),
            total_tokens_used: 300,
            error: None,
        };

        let json = result.to_json().expect("JSON変換に失敗");
        assert!(json.contains("test_workflow"));
        assert!(json.contains("step1"));
        assert!(json.contains("output"));
    }

    #[test]
    fn test_execution_status_equality() {
        assert_eq!(ExecutionStatus::Success, ExecutionStatus::Success);
        assert_eq!(ExecutionStatus::Failed, ExecutionStatus::Failed);
        assert_eq!(
            ExecutionStatus::PartialSuccess {
                completed: 2,
                total: 3
            },
            ExecutionStatus::PartialSuccess {
                completed: 2,
                total: 3
            }
        );
        assert_ne!(ExecutionStatus::Success, ExecutionStatus::Failed);
    }

    #[test]
    fn test_step_status_equality() {
        assert_eq!(StepStatus::Success, StepStatus::Success);
        assert_eq!(StepStatus::Failed, StepStatus::Failed);
        assert_eq!(StepStatus::Skipped, StepStatus::Skipped);
        assert_eq!(
            StepStatus::Retried { attempts: 2 },
            StepStatus::Retried { attempts: 2 }
        );
        assert_ne!(
            StepStatus::Retried { attempts: 1 },
            StepStatus::Retried { attempts: 2 }
        );
        assert_ne!(StepStatus::Success, StepStatus::Failed);
    }

    #[test]
    fn test_execution_error_config_error() {
        let config_err = ConfigError::Validation("無効な設定".to_string());
        let exec_err = ExecutionError::from(config_err);

        assert!(matches!(exec_err, ExecutionError::ConfigError(_)));
        assert_eq!(
            exec_err.to_string(),
            "設定エラー: 設定のバリデーションに失敗しました: 無効な設定"
        );
    }

    #[test]
    fn test_execution_error_provider_error() {
        let provider_err = ProviderError::RateLimitExceeded;
        let exec_err = ExecutionError::from(provider_err);

        assert!(matches!(exec_err, ExecutionError::ProviderError(_)));
        assert_eq!(
            exec_err.to_string(),
            "プロバイダーエラー: レート制限を超えました"
        );
    }

    #[test]
    fn test_execution_error_timeout() {
        let exec_err = ExecutionError::TimeoutError {
            step_name: "long_running_step".to_string(),
            timeout_secs: 300,
        };

        assert_eq!(
            exec_err.to_string(),
            "タイムアウト: ステップ 'long_running_step' が 300秒以内に完了しませんでした"
        );
    }

    #[test]
    fn test_execution_error_validation() {
        let exec_err = ExecutionError::ValidationError("入力が空です".to_string());

        assert_eq!(
            exec_err.to_string(),
            "バリデーションエラー: 入力が空です"
        );
    }

    #[test]
    fn test_execution_error_context() {
        let exec_err = ExecutionError::ContextError("前のステップの出力が見つかりません".to_string());

        assert_eq!(
            exec_err.to_string(),
            "コンテキストエラー: 前のステップの出力が見つかりません"
        );
    }
}
