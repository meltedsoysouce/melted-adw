//! Workflow 定義の読み込みと管理を行うモジュール
//!
//! # 責務
//!
//! このモジュールは、Agent 駆動開発のワークフローを TOML 形式で定義し、
//! それを Rust の型として扱うための機能を提供します。
//!
//! ## 主な機能
//!
//! - **TOML パース**: `workflows/` ディレクトリ内の TOML ファイルを読み込み、
//!   [`Workflow`] 構造体にデシリアライズ
//! - **ワークフロー定義**: 実装計画→実装→レビュー のような開発フローを
//!   ステップの連鎖として表現
//! - **メタデータ管理**: ワークフロー名、説明などの情報を保持
//! - **ステップ参照**: [`Step`](crate::config::step::Step) の配列を管理し、
//!   実行エンジンに渡す
//!
//! ## 設計思想
//!
//! - **宣言的定義**: 手続き的なコードではなく、TOML による宣言的な定義で
//!   ワークフローを記述可能にする
//! - **再利用性**: 一度定義したワークフローを繰り返し実行可能
//! - **可読性**: 非エンジニアでも理解しやすい TOML 形式を採用
//!
//! ## 使用例
//!
//! ```toml
//! [workflow]
//! name = "feature-implementation"
//! description = "新機能の実装ワークフロー"
//!
//! [[steps]]
//! name = "plan"
//! system_prompt = "実装計画を作成してください"
//! provider = "anthropic"
//! model_tier = "heavy"
//!
//! [[steps]]
//! name = "implement"
//! system_prompt = "計画に基づいて実装してください"
//! provider = "anthropic"
//! model_tier = "heavy"
//! ```
//!
//! ## 関連モジュール
//!
//! - [`crate::config::step`]: 各ステップの定義
//! - [`crate::engine::executor`]: ワークフローの実行エンジン
//! - [`crate::telemetry`]: ワークフロー実行時のメトリクス収集

use std::path::Path;

use crate::error::ConfigError;
use super::step::WorkflowStep;
use super::dto::WorkflowDto;

/// ワークフロー定義（ドメインモデル）
///
/// Agent 駆動開発のワークフロー全体を表す構造体です。
/// バリデーション済みの状態を保証し、ビジネスロジックを持ちます。
///
/// ## DTO との違い
///
/// - [`WorkflowDto`]: TOML デシリアライズ専用、バリデーション前の生データ
/// - [`Workflow`]: バリデーション済み、ドメインロジックを持つ
#[derive(Debug, Clone)]
pub struct Workflow {
    /// ワークフロー名
    name: String,
    /// 説明 (オプション)
    description: Option<String>,
    /// バージョン (オプション)
    version: Option<String>,
    /// ステップ配列
    steps: Vec<WorkflowStep>,
}

impl Workflow {
    /// ワークフロー名を取得
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 説明を取得
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// バージョンを取得
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    /// ステップ配列を取得
    pub fn steps(&self) -> &[WorkflowStep] {
        &self.steps
    }
}

impl Workflow {
    /// TOML ファイルからワークフローを読み込む
    ///
    /// # 処理フロー
    ///
    /// 1. ファイル読み込み
    /// 2. TOML デシリアライズ → [`WorkflowDto`]
    /// 3. バリデーション & 変換 → [`Workflow`]
    ///
    /// # 引数
    ///
    /// * `path` - TOML ファイルのパス
    ///
    /// # 戻り値
    ///
    /// * `Ok(Workflow)` - 読み込みに成功した場合
    /// * `Err(ConfigError)` - ファイルの読み込みまたはパースに失敗した場合
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        Self::from_str(&content)
    }

    /// TOML 文字列からワークフローを読み込む
    ///
    /// # 処理フロー
    ///
    /// 1. TOML デシリアライズ → [`WorkflowDto`]
    /// 2. バリデーション & 変換 → [`Workflow`]
    ///
    /// # 引数
    ///
    /// * `toml` - TOML 形式の文字列
    ///
    /// # 戻り値
    ///
    /// * `Ok(Workflow)` - パースに成功した場合
    /// * `Err(ConfigError)` - パースに失敗した場合
    pub fn from_str(toml: &str) -> Result<Self, ConfigError> {
        let dto: WorkflowDto = toml::from_str(toml)?;
        dto.try_into()
    }

    /// ワークフローを TOML 文字列に変換
    ///
    /// # 処理フロー
    ///
    /// 1. ドメインモデル → [`WorkflowDto`] 変換
    /// 2. TOML シリアライズ
    ///
    /// # 戻り値
    ///
    /// * `Ok(String)` - TOML 文字列
    /// * `Err(ConfigError)` - シリアライズに失敗した場合
    pub fn to_string(&self) -> Result<String, ConfigError> {
        let dto: WorkflowDto = self.clone().into();
        let toml_string = toml::to_string(&dto)?;
        Ok(toml_string)
    }

    /// ワークフローを TOML ファイルに保存
    ///
    /// # 処理フロー
    ///
    /// 1. ドメインモデル → TOML 文字列変換
    /// 2. ファイル書き込み
    ///
    /// # 引数
    ///
    /// * `path` - 保存先のファイルパス
    ///
    /// # 戻り値
    ///
    /// * `Ok(())` - 保存に成功した場合
    /// * `Err(ConfigError)` - ファイル書き込みに失敗した場合
    pub fn to_file(&self, path: impl AsRef<Path>) -> Result<(), ConfigError> {
        let toml_string = self.to_string()?;
        std::fs::write(path, toml_string)?;
        Ok(())
    }
}

/// DTO からドメインモデルへの変換（読み込み方向）
///
/// バリデーションを実施し、不正なデータの場合は [`ConfigError::Validation`] を返します。
///
/// # 処理フロー
///
/// 1. 各フィールドのバリデーション
/// 2. ステップの変換（`WorkflowStepDto` → `WorkflowStep`）
/// 3. `Workflow` の構築
impl TryFrom<WorkflowDto> for Workflow {
    type Error = ConfigError;

    fn try_from(dto: WorkflowDto) -> Result<Self, Self::Error> {
        // ワークフロー名のバリデーション
        if dto.workflow.name.trim().is_empty() {
            return Err(ConfigError::Validation(
                "ワークフロー名が空です".to_string()
            ));
        }

        // ステップリストの非空チェック
        if dto.steps.is_empty() {
            return Err(ConfigError::Validation(
                format!("ワークフロー '{}' にステップが定義されていません", dto.workflow.name)
            ));
        }

        // 各ステップを変換（バリデーションも同時に実行）
        let steps: Result<Vec<WorkflowStep>, ConfigError> = dto.steps
            .into_iter()
            .map(|step_dto| WorkflowStep::try_from(step_dto))
            .collect();
        let steps = steps?;

        // ステップ名の一意性確認
        let mut step_names = std::collections::HashSet::new();
        for step in &steps {
            if !step_names.insert(step.name()) {
                return Err(ConfigError::Validation(
                    format!(
                        "ワークフロー '{}' に重複するステップ名があります: '{}'",
                        dto.workflow.name, step.name()
                    )
                ));
            }
        }

        Ok(Workflow {
            name: dto.workflow.name,
            description: dto.workflow.description,
            version: dto.workflow.version,
            steps,
        })
    }
}

/// ドメインモデルから DTO への変換（書き込み方向）
///
/// バリデーション済みのドメインモデルから DTO を生成するため、
/// この変換は失敗しません（`From` トレイトを使用）。
///
/// # 使用例
///
/// ```ignore
/// let workflow: Workflow = /* ... */;
/// let dto: WorkflowDto = workflow.into();
/// let toml_string = toml::to_string(&dto)?;
/// ```
impl From<Workflow> for WorkflowDto {
    fn from(workflow: Workflow) -> Self {
        use super::dto::WorkflowMetadataDto;

        // 各ステップを DTO に変換
        let steps: Vec<super::dto::WorkflowStepDto> = workflow.steps
            .into_iter()
            .map(|step| step.into())
            .collect();

        WorkflowDto {
            workflow: WorkflowMetadataDto {
                name: workflow.name,
                description: workflow.description,
                version: workflow.version,
            },
            steps,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::dto::{WorkflowDto, WorkflowMetadataDto, WorkflowStepDto};

    fn create_valid_step_dto(name: &str) -> WorkflowStepDto {
        WorkflowStepDto {
            name: name.to_string(),
            system_prompt: "Test prompt".to_string(),
            provider: "anthropic".to_string(),
            model_tier: "heavy".to_string(),
            timeout: None,
            retry_count: None,
        }
    }

    fn create_valid_workflow_dto(name: &str, steps: Vec<WorkflowStepDto>) -> WorkflowDto {
        WorkflowDto {
            workflow: WorkflowMetadataDto {
                name: name.to_string(),
                description: Some("Test workflow".to_string()),
                version: Some("1.0.0".to_string()),
            },
            steps,
        }
    }

    #[test]
    fn test_validation_success() {
        // 正常系: 有効なDTOからドメインモデルへの変換
        let dto = create_valid_workflow_dto(
            "test_workflow",
            vec![
                create_valid_step_dto("step1"),
                create_valid_step_dto("step2"),
            ],
        );

        let result = Workflow::try_from(dto);
        assert!(result.is_ok());

        let workflow = result.unwrap();
        assert_eq!(workflow.name(), "test_workflow");
        assert_eq!(workflow.description(), Some("Test workflow"));
        assert_eq!(workflow.version(), Some("1.0.0"));
        assert_eq!(workflow.steps().len(), 2);
    }

    #[test]
    fn test_validation_minimal() {
        // 正常系: 最小限のフィールドのみ
        let dto = WorkflowDto {
            workflow: WorkflowMetadataDto {
                name: "minimal".to_string(),
                description: None,
                version: None,
            },
            steps: vec![create_valid_step_dto("step1")],
        };

        let result = Workflow::try_from(dto);
        assert!(result.is_ok());

        let workflow = result.unwrap();
        assert_eq!(workflow.name(), "minimal");
        assert_eq!(workflow.description(), None);
        assert_eq!(workflow.version(), None);
        assert_eq!(workflow.steps().len(), 1);
    }

    #[test]
    fn test_validation_empty_workflow_name() {
        // 異常系: 空のワークフロー名
        let dto = create_valid_workflow_dto("", vec![create_valid_step_dto("step1")]);

        let result = Workflow::try_from(dto);
        assert!(result.is_err());
        if let Err(ConfigError::Validation(msg)) = result {
            assert!(msg.contains("ワークフロー名が空です"));
        } else {
            panic!("Expected Validation error");
        }
    }

    #[test]
    fn test_validation_whitespace_workflow_name() {
        // 異常系: 空白のみのワークフロー名
        let dto = create_valid_workflow_dto("   ", vec![create_valid_step_dto("step1")]);

        let result = Workflow::try_from(dto);
        assert!(result.is_err());
        if let Err(ConfigError::Validation(msg)) = result {
            assert!(msg.contains("ワークフロー名が空です"));
        } else {
            panic!("Expected Validation error");
        }
    }

    #[test]
    fn test_validation_no_steps() {
        // 異常系: ステップが空
        let dto = create_valid_workflow_dto("workflow", vec![]);

        let result = Workflow::try_from(dto);
        assert!(result.is_err());
        if let Err(ConfigError::Validation(msg)) = result {
            assert!(msg.contains("ステップが定義されていません"));
        } else {
            panic!("Expected Validation error");
        }
    }

    #[test]
    fn test_validation_invalid_step() {
        // 異常系: 不正なステップを含む
        let mut dto = create_valid_workflow_dto(
            "workflow",
            vec![create_valid_step_dto("step1")],
        );

        // 不正なプロバイダーを持つステップを追加
        dto.steps.push(WorkflowStepDto {
            name: "step2".to_string(),
            system_prompt: "prompt".to_string(),
            provider: "invalid".to_string(),
            model_tier: "heavy".to_string(),
            timeout: None,
            retry_count: None,
        });

        let result = Workflow::try_from(dto);
        assert!(result.is_err());
        if let Err(ConfigError::Validation(msg)) = result {
            assert!(msg.contains("不正なプロバイダー"));
        } else {
            panic!("Expected Validation error");
        }
    }

    #[test]
    fn test_validation_duplicate_step_names() {
        // 異常系: 重複するステップ名
        let dto = create_valid_workflow_dto(
            "workflow",
            vec![
                create_valid_step_dto("duplicate"),
                create_valid_step_dto("duplicate"),
            ],
        );

        let result = Workflow::try_from(dto);
        assert!(result.is_err());
        if let Err(ConfigError::Validation(msg)) = result {
            assert!(msg.contains("重複するステップ名"));
            assert!(msg.contains("duplicate"));
        } else {
            panic!("Expected Validation error");
        }
    }

    #[test]
    fn test_validation_duplicate_among_many_steps() {
        // 異常系: 複数のステップの中に重複がある
        let dto = create_valid_workflow_dto(
            "workflow",
            vec![
                create_valid_step_dto("step1"),
                create_valid_step_dto("step2"),
                create_valid_step_dto("step3"),
                create_valid_step_dto("step2"), // 重複
                create_valid_step_dto("step4"),
            ],
        );

        let result = Workflow::try_from(dto);
        assert!(result.is_err());
        if let Err(ConfigError::Validation(msg)) = result {
            assert!(msg.contains("重複するステップ名"));
            assert!(msg.contains("step2"));
        } else {
            panic!("Expected Validation error");
        }
    }

    #[test]
    fn test_round_trip_conversion() {
        // 正常系: DTO → ドメインモデル → DTO の往復変換
        let original_dto = create_valid_workflow_dto(
            "test_workflow",
            vec![
                create_valid_step_dto("step1"),
                create_valid_step_dto("step2"),
            ],
        );

        // DTO → ドメインモデル
        let workflow = Workflow::try_from(original_dto.clone()).unwrap();

        // ドメインモデル → DTO
        let converted_dto: WorkflowDto = workflow.into();

        // 変換後のDTOが元と同じであることを確認
        assert_eq!(converted_dto.workflow.name, original_dto.workflow.name);
        assert_eq!(
            converted_dto.workflow.description,
            original_dto.workflow.description
        );
        assert_eq!(
            converted_dto.workflow.version,
            original_dto.workflow.version
        );
        assert_eq!(converted_dto.steps.len(), original_dto.steps.len());

        for (i, step) in converted_dto.steps.iter().enumerate() {
            assert_eq!(step.name, original_dto.steps[i].name);
            assert_eq!(step.system_prompt, original_dto.steps[i].system_prompt);
        }
    }

    #[test]
    fn test_workflow_accessors() {
        // ドメインモデルのアクセサーメソッドのテスト
        let dto = create_valid_workflow_dto(
            "workflow",
            vec![
                create_valid_step_dto("step1"),
                create_valid_step_dto("step2"),
            ],
        );

        let workflow = Workflow::try_from(dto).unwrap();

        assert_eq!(workflow.name(), "workflow");
        assert_eq!(workflow.description(), Some("Test workflow"));
        assert_eq!(workflow.version(), Some("1.0.0"));

        let steps = workflow.steps();
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].name(), "step1");
        assert_eq!(steps[1].name(), "step2");
    }

    #[test]
    fn test_complex_workflow() {
        // 正常系: 複雑なワークフローの変換
        let dto = WorkflowDto {
            workflow: WorkflowMetadataDto {
                name: "complex_workflow".to_string(),
                description: Some("A complex workflow".to_string()),
                version: Some("2.0.0".to_string()),
            },
            steps: vec![
                WorkflowStepDto {
                    name: "plan".to_string(),
                    system_prompt: "Create implementation plan".to_string(),
                    provider: "anthropic".to_string(),
                    model_tier: "heavy".to_string(),
                    timeout: Some(300),
                    retry_count: Some(3),
                },
                WorkflowStepDto {
                    name: "implement".to_string(),
                    system_prompt: "Implement the feature".to_string(),
                    provider: "openai".to_string(),
                    model_tier: "medium".to_string(),
                    timeout: Some(600),
                    retry_count: Some(5),
                },
                WorkflowStepDto {
                    name: "review".to_string(),
                    system_prompt: "Review the implementation".to_string(),
                    provider: "anthropic".to_string(),
                    model_tier: "light".to_string(),
                    timeout: None,
                    retry_count: None,
                },
            ],
        };

        let result = Workflow::try_from(dto);
        assert!(result.is_ok());

        let workflow = result.unwrap();
        assert_eq!(workflow.steps().len(), 3);
        assert_eq!(workflow.steps()[0].name(), "plan");
        assert_eq!(workflow.steps()[1].name(), "implement");
        assert_eq!(workflow.steps()[2].name(), "review");
    }

    #[test]
    fn test_from_str_valid_toml() {
        // 正常系: 有効なTOML文字列からの読み込み
        let toml = r#"
[workflow]
name = "test-workflow"
description = "A test workflow"
version = "1.0.0"

[[steps]]
name = "step1"
system_prompt = "First step"
provider = "anthropic"
model_tier = "heavy"

[[steps]]
name = "step2"
system_prompt = "Second step"
provider = "openai"
model_tier = "medium"
timeout = 120
retry_count = 3
"#;

        let result = Workflow::from_str(toml);
        assert!(result.is_ok());

        let workflow = result.unwrap();
        assert_eq!(workflow.name(), "test-workflow");
        assert_eq!(workflow.description(), Some("A test workflow"));
        assert_eq!(workflow.version(), Some("1.0.0"));
        assert_eq!(workflow.steps().len(), 2);
        assert_eq!(workflow.steps()[0].name(), "step1");
        assert_eq!(workflow.steps()[1].name(), "step2");
    }

    #[test]
    fn test_from_str_invalid_toml() {
        // 異常系: 不正なTOML形式のエラーハンドリング
        let invalid_toml = r#"
[workflow
name = "broken"
"#;

        let result = Workflow::from_str(invalid_toml);
        assert!(result.is_err());

        // ConfigError::TomlDeserialize エラーが返されることを確認
        match result {
            Err(ConfigError::TomlDeserialize(_)) => {
                // 期待通りのエラー
            }
            _ => panic!("Expected TomlDeserialize error"),
        }
    }

    #[test]
    fn test_from_str_validation_error() {
        // 異常系: TOML構文は正しいがバリデーションエラー
        let toml = r#"
[workflow]
name = ""

[[steps]]
name = "step1"
system_prompt = "prompt"
provider = "anthropic"
model_tier = "heavy"
"#;

        let result = Workflow::from_str(toml);
        assert!(result.is_err());

        // ConfigError::Validation エラーが返されることを確認
        match result {
            Err(ConfigError::Validation(_)) => {
                // 期待通りのエラー
            }
            _ => panic!("Expected Validation error"),
        }
    }

    #[test]
    fn test_to_string_and_back() {
        // 正常系: to_string → from_str のラウンドトリップ
        let dto = create_valid_workflow_dto(
            "roundtrip-test",
            vec![
                create_valid_step_dto("step1"),
                create_valid_step_dto("step2"),
            ],
        );

        // DTO → ドメインモデル
        let original_workflow = Workflow::try_from(dto).unwrap();

        // ドメインモデル → TOML文字列
        let toml_string = original_workflow.to_string().unwrap();

        // TOML文字列 → ドメインモデル
        let restored_workflow = Workflow::from_str(&toml_string).unwrap();

        // 元のデータと復元されたデータが一致することを確認
        assert_eq!(restored_workflow.name(), original_workflow.name());
        assert_eq!(
            restored_workflow.description(),
            original_workflow.description()
        );
        assert_eq!(restored_workflow.version(), original_workflow.version());
        assert_eq!(
            restored_workflow.steps().len(),
            original_workflow.steps().len()
        );

        for (i, step) in restored_workflow.steps().iter().enumerate() {
            assert_eq!(step.name(), original_workflow.steps()[i].name());
        }
    }

    #[test]
    fn test_roundtrip() {
        // 正常系: ファイル → ドメインモデル → ファイル のラウンドトリップテスト
        use std::io::Write;

        // 一時ファイルを作成
        let temp_dir = std::env::temp_dir();
        let input_path = temp_dir.join("test_workflow_input.toml");
        let output_path = temp_dir.join("test_workflow_output.toml");

        // テスト用のTOMLを書き込み
        let original_toml = r#"
[workflow]
name = "file-roundtrip"
description = "Testing file I/O"
version = "2.0.0"

[[steps]]
name = "read-step"
system_prompt = "Read from file"
provider = "anthropic"
model_tier = "heavy"
timeout = 100
retry_count = 2

[[steps]]
name = "write-step"
system_prompt = "Write to file"
provider = "openai"
model_tier = "light"
"#;

        let mut file = std::fs::File::create(&input_path).unwrap();
        file.write_all(original_toml.as_bytes()).unwrap();

        // ファイルから読み込み
        let workflow = Workflow::from_file(&input_path).unwrap();

        // ファイルに書き込み
        workflow.to_file(&output_path).unwrap();

        // 出力ファイルから再度読み込み
        let restored_workflow = Workflow::from_file(&output_path).unwrap();

        // データの一致を確認
        assert_eq!(restored_workflow.name(), "file-roundtrip");
        assert_eq!(restored_workflow.description(), Some("Testing file I/O"));
        assert_eq!(restored_workflow.version(), Some("2.0.0"));
        assert_eq!(restored_workflow.steps().len(), 2);
        assert_eq!(restored_workflow.steps()[0].name(), "read-step");
        assert_eq!(restored_workflow.steps()[1].name(), "write-step");

        // 一時ファイルをクリーンアップ
        let _ = std::fs::remove_file(input_path);
        let _ = std::fs::remove_file(output_path);
    }

    #[test]
    fn test_from_file_nonexistent() {
        // 異常系: 存在しないファイルの読み込み
        let result = Workflow::from_file("/nonexistent/path/to/workflow.toml");
        assert!(result.is_err());

        // ConfigError::FileRead エラーが返されることを確認
        match result {
            Err(ConfigError::FileRead(_)) => {
                // 期待通りのエラー
            }
            _ => panic!("Expected FileRead error"),
        }
    }

    #[test]
    fn test_to_file_invalid_path() {
        // 異常系: 書き込みできないパスへの保存
        let dto = create_valid_workflow_dto(
            "test",
            vec![create_valid_step_dto("step1")],
        );
        let workflow = Workflow::try_from(dto).unwrap();

        // 存在しないディレクトリに書き込もうとする
        let result = workflow.to_file("/nonexistent/directory/workflow.toml");
        assert!(result.is_err());

        // ConfigError::FileRead (std::io::Error から変換) が返されることを確認
        match result {
            Err(ConfigError::FileRead(_)) => {
                // 期待通りのエラー
            }
            _ => panic!("Expected FileRead error"),
        }
    }
}
