//! ステップ実行コンテキストの管理
//!
//! # 責務
//!
//! - ステップ実行の進行状況を追跡
//! - 各ステップの入力と出力を保持
//! - テレメトリー情報（トークン使用量、実行時間、リトライ回数）の累積
//!
//! # 主要な型
//!
//! - [`ExecutionContext`][]: ワークフロー実行全体のコンテキスト
//! - [`StepOutput`][]: 個々のステップの実行結果
//!
//! # 使用例
//!
//! ```rust
//! use melted_adw::engine::context::{ExecutionContext, StepOutput};
//! use melted_adw::provider::TokenUsage;
//! use std::time::Duration;
//!
//! let mut ctx = ExecutionContext::new("example_workflow".to_string());
//!
//! // ステップ実行開始
//! ctx.start_step("step1");
//!
//! // ステップ結果を記録
//! let output = StepOutput::new(
//!     "step1".to_string(),
//!     "Result from step 1".to_string(),
//!     TokenUsage { input_tokens: 100, output_tokens: 50 },
//!     Duration::from_secs(2),
//! );
//! ctx.record_step_result(output);
//!
//! // 次のステップで前のステップの出力を参照
//! if let Some(last_output) = ctx.get_last_output() {
//!     println!("Previous step output: {}", last_output.content);
//! }
//!
//! // テレメトリー情報を取得
//! println!("Total tokens used: {}", ctx.total_tokens());
//! println!("Total duration: {:?}", ctx.total_duration());
//! ```

use crate::provider::TokenUsage;
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

/// ステップ実行コンテキスト
///
/// ワークフロー実行全体の状態を管理し、各ステップの実行履歴、
/// ステップ間のデータ受け渡し、テレメトリー情報を保持します。
///
/// # フィールド
///
/// - `workflow_name`: 実行中のワークフロー名
/// - `start_time`: ワークフロー開始時刻
/// - `steps_executed`: 完了したステップ名のリスト（実行順）
/// - `current_step`: 現在実行中のステップ名
/// - `step_outputs`: 各ステップの実行結果（ステップ間データ受け渡しに使用）
/// - `total_tokens_used`: ワークフロー全体で使用したトークン数の累積
/// - `execution_times`: 各ステップの実行時間のリスト
/// - `retry_counts`: ステップ名をキーとしたリトライ回数のマップ
#[derive(Debug)]
pub struct ExecutionContext {
    #[allow(dead_code)]
    workflow_name: String,
    #[allow(dead_code)]
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
    /// 新しい実行コンテキストを生成
    ///
    /// # 引数
    ///
    /// - `workflow_name`: 実行するワークフローの名前
    ///
    /// # 戻り値
    ///
    /// 初期化された ExecutionContext インスタンス
    ///
    /// # 例
    ///
    /// ```rust
    /// use melted_adw::engine::context::ExecutionContext;
    ///
    /// let ctx = ExecutionContext::new("my_workflow".to_string());
    /// ```
    pub fn new(workflow_name: String) -> Self {
        Self {
            workflow_name,
            start_time: SystemTime::now(),
            steps_executed: Vec::new(),
            current_step: None,
            step_outputs: Vec::new(),
            total_tokens_used: 0,
            execution_times: Vec::new(),
            retry_counts: HashMap::new(),
        }
    }

    /// ステップ実行を開始
    ///
    /// 現在実行中のステップとして記録します。
    ///
    /// # 引数
    ///
    /// - `step_name`: 開始するステップの名前
    ///
    /// # 例
    ///
    /// ```rust
    /// use melted_adw::engine::context::ExecutionContext;
    ///
    /// let mut ctx = ExecutionContext::new("workflow".to_string());
    /// ctx.start_step("validation");
    /// ```
    pub fn start_step(&mut self, step_name: &str) {
        self.current_step = Some(step_name.to_string());
    }

    /// ステップ完了と結果を記録
    ///
    /// ステップの実行結果を保存し、テレメトリー情報を更新します。
    /// このメソッドを呼ぶと、ステップは完了したものとして扱われます。
    ///
    /// # 引数
    ///
    /// - `output`: ステップの実行結果
    ///
    /// # 例
    ///
    /// ```rust
    /// use melted_adw::engine::context::{ExecutionContext, StepOutput};
    /// use melted_adw::provider::TokenUsage;
    /// use std::time::Duration;
    ///
    /// let mut ctx = ExecutionContext::new("workflow".to_string());
    /// ctx.start_step("step1");
    ///
    /// let output = StepOutput::new(
    ///     "step1".to_string(),
    ///     "Output content".to_string(),
    ///     TokenUsage { input_tokens: 10, output_tokens: 20 },
    ///     Duration::from_secs(1),
    /// );
    /// ctx.record_step_result(output);
    /// ```
    pub fn record_step_result(&mut self, output: StepOutput) {
        // テレメトリー情報を更新
        self.total_tokens_used += output.token_usage.total();
        self.execution_times.push(output.execution_time);

        // ステップ履歴を更新
        self.steps_executed.push(output.step_name.clone());

        // 出力を保存
        self.step_outputs.push(output);

        // 現在のステップをクリア
        self.current_step = None;
    }

    /// 最後のステップの出力を取得
    ///
    /// 次のステップの入力として前のステップの出力を参照する際に使用します。
    ///
    /// # 戻り値
    ///
    /// - `Some(&StepOutput)`: 最後のステップの出力（ステップが実行済みの場合）
    /// - `None`: まだステップが実行されていない場合
    ///
    /// # 例
    ///
    /// ```rust
    /// use melted_adw::engine::context::{ExecutionContext, StepOutput};
    /// use melted_adw::provider::TokenUsage;
    /// use std::time::Duration;
    ///
    /// let mut ctx = ExecutionContext::new("workflow".to_string());
    ///
    /// let output = StepOutput::new(
    ///     "step1".to_string(),
    ///     "Output from step 1".to_string(),
    ///     TokenUsage { input_tokens: 10, output_tokens: 20 },
    ///     Duration::from_secs(1),
    /// );
    /// ctx.record_step_result(output);
    ///
    /// if let Some(last_output) = ctx.get_last_output() {
    ///     assert_eq!(last_output.step_name, "step1");
    /// }
    /// ```
    pub fn get_last_output(&self) -> Option<&StepOutput> {
        self.step_outputs.last()
    }

    /// 特定のステップの出力を取得
    ///
    /// ステップ名を指定して、そのステップの実行結果を取得します。
    ///
    /// # 引数
    ///
    /// - `step_name`: 取得したいステップの名前
    ///
    /// # 戻り値
    ///
    /// - `Some(&StepOutput)`: 指定されたステップの出力（存在する場合）
    /// - `None`: 指定されたステップが存在しない、または実行されていない場合
    ///
    /// # 例
    ///
    /// ```rust
    /// use melted_adw::engine::context::{ExecutionContext, StepOutput};
    /// use melted_adw::provider::TokenUsage;
    /// use std::time::Duration;
    ///
    /// let mut ctx = ExecutionContext::new("workflow".to_string());
    ///
    /// let output = StepOutput::new(
    ///     "validation".to_string(),
    ///     "Valid".to_string(),
    ///     TokenUsage { input_tokens: 5, output_tokens: 10 },
    ///     Duration::from_millis(500),
    /// );
    /// ctx.record_step_result(output);
    ///
    /// if let Some(output) = ctx.get_step_output("validation") {
    ///     assert_eq!(output.content, "Valid");
    /// }
    /// ```
    pub fn get_step_output(&self, step_name: &str) -> Option<&StepOutput> {
        self.step_outputs
            .iter()
            .find(|output| output.step_name == step_name)
    }

    /// リトライカウントを増加
    ///
    /// 指定されたステップのリトライ回数を1増やします。
    /// ステップが初めてリトライされる場合、カウントは1になります。
    ///
    /// # 引数
    ///
    /// - `step_name`: リトライするステップの名前
    ///
    /// # 例
    ///
    /// ```rust
    /// use melted_adw::engine::context::ExecutionContext;
    ///
    /// let mut ctx = ExecutionContext::new("workflow".to_string());
    ///
    /// ctx.increment_retry("step1");
    /// assert_eq!(ctx.get_retry_count("step1"), 1);
    ///
    /// ctx.increment_retry("step1");
    /// assert_eq!(ctx.get_retry_count("step1"), 2);
    /// ```
    pub fn increment_retry(&mut self, step_name: &str) {
        *self.retry_counts.entry(step_name.to_string()).or_insert(0) += 1;
    }

    /// リトライカウントを取得
    ///
    /// 指定されたステップのリトライ回数を取得します。
    ///
    /// # 引数
    ///
    /// - `step_name`: 確認したいステップの名前
    ///
    /// # 戻り値
    ///
    /// リトライ回数（リトライされたことがない場合は0）
    ///
    /// # 例
    ///
    /// ```rust
    /// use melted_adw::engine::context::ExecutionContext;
    ///
    /// let mut ctx = ExecutionContext::new("workflow".to_string());
    ///
    /// assert_eq!(ctx.get_retry_count("step1"), 0);
    ///
    /// ctx.increment_retry("step1");
    /// assert_eq!(ctx.get_retry_count("step1"), 1);
    /// ```
    pub fn get_retry_count(&self, step_name: &str) -> u32 {
        *self.retry_counts.get(step_name).unwrap_or(&0)
    }

    /// 総トークン使用量を取得
    ///
    /// ワークフロー全体で使用された総トークン数を返します。
    ///
    /// # 戻り値
    ///
    /// 総トークン数（入力トークン + 出力トークン）
    ///
    /// # 例
    ///
    /// ```rust
    /// use melted_adw::engine::context::{ExecutionContext, StepOutput};
    /// use melted_adw::provider::TokenUsage;
    /// use std::time::Duration;
    ///
    /// let mut ctx = ExecutionContext::new("workflow".to_string());
    ///
    /// let output1 = StepOutput::new(
    ///     "step1".to_string(),
    ///     "Output 1".to_string(),
    ///     TokenUsage { input_tokens: 100, output_tokens: 50 },
    ///     Duration::from_secs(1),
    /// );
    /// ctx.record_step_result(output1);
    ///
    /// let output2 = StepOutput::new(
    ///     "step2".to_string(),
    ///     "Output 2".to_string(),
    ///     TokenUsage { input_tokens: 200, output_tokens: 100 },
    ///     Duration::from_secs(2),
    /// );
    /// ctx.record_step_result(output2);
    ///
    /// assert_eq!(ctx.total_tokens(), 450); // 100 + 50 + 200 + 100
    /// ```
    pub fn total_tokens(&self) -> u32 {
        self.total_tokens_used
    }

    /// 総実行時間を取得
    ///
    /// 全ステップの実行時間の合計を返します。
    ///
    /// # 戻り値
    ///
    /// 総実行時間
    ///
    /// # 例
    ///
    /// ```rust
    /// use melted_adw::engine::context::{ExecutionContext, StepOutput};
    /// use melted_adw::provider::TokenUsage;
    /// use std::time::Duration;
    ///
    /// let mut ctx = ExecutionContext::new("workflow".to_string());
    ///
    /// let output1 = StepOutput::new(
    ///     "step1".to_string(),
    ///     "Output 1".to_string(),
    ///     TokenUsage { input_tokens: 10, output_tokens: 10 },
    ///     Duration::from_secs(1),
    /// );
    /// ctx.record_step_result(output1);
    ///
    /// let output2 = StepOutput::new(
    ///     "step2".to_string(),
    ///     "Output 2".to_string(),
    ///     TokenUsage { input_tokens: 10, output_tokens: 10 },
    ///     Duration::from_secs(2),
    /// );
    /// ctx.record_step_result(output2);
    ///
    /// assert_eq!(ctx.total_duration(), Duration::from_secs(3));
    /// ```
    pub fn total_duration(&self) -> Duration {
        self.execution_times.iter().sum()
    }
}

/// ステップ出力
///
/// 個々のステップの実行結果を表します。
/// ステップ間のデータ受け渡しとテレメトリー情報収集に使用されます。
///
/// # フィールド
///
/// - `step_name`: ステップの名前
/// - `content`: ステップが生成した出力内容（LLMのレスポンスなど）
/// - `token_usage`: ステップで使用されたトークン数
/// - `execution_time`: ステップの実行にかかった時間
#[derive(Debug, Clone)]
pub struct StepOutput {
    pub step_name: String,
    pub content: String,
    pub token_usage: TokenUsage,
    pub execution_time: Duration,
}

impl StepOutput {
    /// 新しいステップ出力を生成
    ///
    /// # 引数
    ///
    /// - `step_name`: ステップの名前
    /// - `content`: ステップの出力内容
    /// - `token_usage`: トークン使用量
    /// - `execution_time`: 実行時間
    ///
    /// # 戻り値
    ///
    /// 初期化された StepOutput インスタンス
    ///
    /// # 例
    ///
    /// ```rust
    /// use melted_adw::engine::context::StepOutput;
    /// use melted_adw::provider::TokenUsage;
    /// use std::time::Duration;
    ///
    /// let output = StepOutput::new(
    ///     "validation".to_string(),
    ///     "Input is valid".to_string(),
    ///     TokenUsage {
    ///         input_tokens: 50,
    ///         output_tokens: 20,
    ///     },
    ///     Duration::from_millis(500),
    /// );
    ///
    /// assert_eq!(output.step_name, "validation");
    /// assert_eq!(output.content, "Input is valid");
    /// assert_eq!(output.token_usage.total(), 70);
    /// ```
    pub fn new(
        step_name: String,
        content: String,
        token_usage: TokenUsage,
        execution_time: Duration,
    ) -> Self {
        Self {
            step_name,
            content,
            token_usage,
            execution_time,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ExecutionContext::new() のテスト
    #[test]
    fn test_execution_context_new() {
        let ctx = ExecutionContext::new("test_workflow".to_string());
        assert_eq!(ctx.workflow_name, "test_workflow");
        assert!(ctx.current_step.is_none());
        assert_eq!(ctx.steps_executed.len(), 0);
        assert_eq!(ctx.step_outputs.len(), 0);
        assert_eq!(ctx.total_tokens_used, 0);
        assert_eq!(ctx.execution_times.len(), 0);
        assert_eq!(ctx.retry_counts.len(), 0);
    }

    /// start_step() と current_step の更新をテスト
    #[test]
    fn test_start_step() {
        let mut ctx = ExecutionContext::new("workflow".to_string());
        ctx.start_step("step1");
        assert_eq!(ctx.current_step, Some("step1".to_string()));

        ctx.start_step("step2");
        assert_eq!(ctx.current_step, Some("step2".to_string()));
    }

    /// record_step_result() によるステップ出力の記録をテスト
    #[test]
    fn test_record_step_result() {
        let mut ctx = ExecutionContext::new("workflow".to_string());
        ctx.start_step("step1");

        let output = StepOutput::new(
            "step1".to_string(),
            "Result 1".to_string(),
            TokenUsage {
                input_tokens: 100,
                output_tokens: 50,
            },
            Duration::from_secs(1),
        );

        ctx.record_step_result(output);

        assert_eq!(ctx.steps_executed.len(), 1);
        assert_eq!(ctx.steps_executed[0], "step1");
        assert_eq!(ctx.step_outputs.len(), 1);
        assert_eq!(ctx.step_outputs[0].step_name, "step1");
        assert_eq!(ctx.step_outputs[0].content, "Result 1");
        assert!(ctx.current_step.is_none());
    }

    /// get_last_output() のテスト
    #[test]
    fn test_get_last_output() {
        let mut ctx = ExecutionContext::new("workflow".to_string());

        // 出力がない場合
        assert!(ctx.get_last_output().is_none());

        // 1つ目の出力を記録
        let output1 = StepOutput::new(
            "step1".to_string(),
            "Result 1".to_string(),
            TokenUsage {
                input_tokens: 10,
                output_tokens: 5,
            },
            Duration::from_secs(1),
        );
        ctx.record_step_result(output1);

        let last = ctx.get_last_output().unwrap();
        assert_eq!(last.step_name, "step1");
        assert_eq!(last.content, "Result 1");

        // 2つ目の出力を記録
        let output2 = StepOutput::new(
            "step2".to_string(),
            "Result 2".to_string(),
            TokenUsage {
                input_tokens: 20,
                output_tokens: 10,
            },
            Duration::from_secs(2),
        );
        ctx.record_step_result(output2);

        let last = ctx.get_last_output().unwrap();
        assert_eq!(last.step_name, "step2");
        assert_eq!(last.content, "Result 2");
    }

    /// get_step_output() のテスト
    #[test]
    fn test_get_step_output() {
        let mut ctx = ExecutionContext::new("workflow".to_string());

        let output1 = StepOutput::new(
            "validation".to_string(),
            "Valid".to_string(),
            TokenUsage {
                input_tokens: 10,
                output_tokens: 5,
            },
            Duration::from_millis(500),
        );
        ctx.record_step_result(output1);

        let output2 = StepOutput::new(
            "processing".to_string(),
            "Processed".to_string(),
            TokenUsage {
                input_tokens: 20,
                output_tokens: 10,
            },
            Duration::from_secs(1),
        );
        ctx.record_step_result(output2);

        // 存在するステップの出力を取得
        let validation_output = ctx.get_step_output("validation").unwrap();
        assert_eq!(validation_output.step_name, "validation");
        assert_eq!(validation_output.content, "Valid");

        let processing_output = ctx.get_step_output("processing").unwrap();
        assert_eq!(processing_output.step_name, "processing");
        assert_eq!(processing_output.content, "Processed");

        // 存在しないステップの出力を取得
        assert!(ctx.get_step_output("nonexistent").is_none());
    }

    /// increment_retry() と get_retry_count() のテスト
    #[test]
    fn test_retry_counts() {
        let mut ctx = ExecutionContext::new("workflow".to_string());

        // 初期状態はリトライなし
        assert_eq!(ctx.get_retry_count("step1"), 0);

        // リトライを増やす
        ctx.increment_retry("step1");
        assert_eq!(ctx.get_retry_count("step1"), 1);

        ctx.increment_retry("step1");
        assert_eq!(ctx.get_retry_count("step1"), 2);

        // 別のステップのリトライ
        ctx.increment_retry("step2");
        assert_eq!(ctx.get_retry_count("step2"), 1);
        assert_eq!(ctx.get_retry_count("step1"), 2); // step1は変わらない
    }

    /// total_tokens() のテスト - トークン使用量の累積
    #[test]
    fn test_total_tokens() {
        let mut ctx = ExecutionContext::new("workflow".to_string());

        assert_eq!(ctx.total_tokens(), 0);

        let output1 = StepOutput::new(
            "step1".to_string(),
            "Result 1".to_string(),
            TokenUsage {
                input_tokens: 100,
                output_tokens: 50,
            },
            Duration::from_secs(1),
        );
        ctx.record_step_result(output1);
        assert_eq!(ctx.total_tokens(), 150); // 100 + 50

        let output2 = StepOutput::new(
            "step2".to_string(),
            "Result 2".to_string(),
            TokenUsage {
                input_tokens: 200,
                output_tokens: 100,
            },
            Duration::from_secs(2),
        );
        ctx.record_step_result(output2);
        assert_eq!(ctx.total_tokens(), 450); // 150 + 200 + 100
    }

    /// total_duration() のテスト - 実行時間の累積
    #[test]
    fn test_total_duration() {
        let mut ctx = ExecutionContext::new("workflow".to_string());

        assert_eq!(ctx.total_duration(), Duration::from_secs(0));

        let output1 = StepOutput::new(
            "step1".to_string(),
            "Result 1".to_string(),
            TokenUsage {
                input_tokens: 10,
                output_tokens: 10,
            },
            Duration::from_secs(1),
        );
        ctx.record_step_result(output1);
        assert_eq!(ctx.total_duration(), Duration::from_secs(1));

        let output2 = StepOutput::new(
            "step2".to_string(),
            "Result 2".to_string(),
            TokenUsage {
                input_tokens: 10,
                output_tokens: 10,
            },
            Duration::from_millis(1500),
        );
        ctx.record_step_result(output2);
        assert_eq!(ctx.total_duration(), Duration::from_millis(2500));
    }

    /// StepOutput::new() のテスト
    #[test]
    fn test_step_output_new() {
        let output = StepOutput::new(
            "test_step".to_string(),
            "Test content".to_string(),
            TokenUsage {
                input_tokens: 50,
                output_tokens: 25,
            },
            Duration::from_millis(750),
        );

        assert_eq!(output.step_name, "test_step");
        assert_eq!(output.content, "Test content");
        assert_eq!(output.token_usage.input_tokens, 50);
        assert_eq!(output.token_usage.output_tokens, 25);
        assert_eq!(output.token_usage.total(), 75);
        assert_eq!(output.execution_time, Duration::from_millis(750));
    }

    /// 複数ステップの履歴管理をテスト
    #[test]
    fn test_multiple_steps_history() {
        let mut ctx = ExecutionContext::new("complex_workflow".to_string());

        // 3つのステップを実行
        let steps = vec![
            ("step1", "Output 1", 100, 50, 1000),
            ("step2", "Output 2", 200, 100, 2000),
            ("step3", "Output 3", 150, 75, 1500),
        ];

        for (name, content, input_tokens, output_tokens, duration_ms) in steps {
            ctx.start_step(name);
            let output = StepOutput::new(
                name.to_string(),
                content.to_string(),
                TokenUsage {
                    input_tokens,
                    output_tokens,
                },
                Duration::from_millis(duration_ms),
            );
            ctx.record_step_result(output);
        }

        // 履歴の確認
        assert_eq!(ctx.steps_executed.len(), 3);
        assert_eq!(ctx.step_outputs.len(), 3);
        assert_eq!(ctx.steps_executed, vec!["step1", "step2", "step3"]);

        // 最後の出力
        let last = ctx.get_last_output().unwrap();
        assert_eq!(last.step_name, "step3");

        // 特定のステップの出力
        let step2_output = ctx.get_step_output("step2").unwrap();
        assert_eq!(step2_output.content, "Output 2");

        // テレメトリー
        assert_eq!(ctx.total_tokens(), 675); // (100+50) + (200+100) + (150+75)
        assert_eq!(ctx.total_duration(), Duration::from_millis(4500));
    }

    /// リトライと正常実行の組み合わせをテスト
    #[test]
    fn test_retry_with_execution() {
        let mut ctx = ExecutionContext::new("workflow_with_retries".to_string());

        // step1を2回リトライしてから成功
        ctx.increment_retry("step1");
        ctx.increment_retry("step1");
        ctx.start_step("step1");
        let output1 = StepOutput::new(
            "step1".to_string(),
            "Success after retries".to_string(),
            TokenUsage {
                input_tokens: 100,
                output_tokens: 50,
            },
            Duration::from_secs(1),
        );
        ctx.record_step_result(output1);

        // step2は1発で成功
        ctx.start_step("step2");
        let output2 = StepOutput::new(
            "step2".to_string(),
            "Success".to_string(),
            TokenUsage {
                input_tokens: 50,
                output_tokens: 25,
            },
            Duration::from_secs(1),
        );
        ctx.record_step_result(output2);

        assert_eq!(ctx.get_retry_count("step1"), 2);
        assert_eq!(ctx.get_retry_count("step2"), 0);
        assert_eq!(ctx.steps_executed.len(), 2);
    }
}
