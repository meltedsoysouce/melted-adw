//! TOML デシリアライズ用の DTO (Data Transfer Object)
//!
//! # 責務
//!
//! このモジュールは、TOML ファイルからのデータ読み込み専用の構造体を提供します。
//! DTO はバリデーション前の「生データ」を表現し、ドメインモデルとは分離されています。
//!
//! ## 設計思想
//!
//! - **単一責務**: TOML のデシリアライズのみを担当
//! - **TOML 構造への密結合**: TOML の構造変更に柔軟に対応
//! - **バリデーション前の状態**: 不正なデータも一旦受け入れる
//! - **カプセル化**: config モジュール内部のみで使用（外部非公開）
//!
//! ## 変換フロー
//!
//! ```text
//! TOML ファイル
//!   ↓ (デシリアライズ)
//! WorkflowDto
//!   ↓ (TryFrom でバリデーション)
//! Workflow (ドメインモデル)
//! ```

use serde::{Deserialize, Serialize};

/// ワークフロー DTO
///
/// TOML の `[workflow]` セクションと `[[steps]]` 配列をデシリアライズ/シリアライズします。
///
/// **注**: この構造体は config モジュール内部の実装詳細です。
/// 外部からは [`Workflow`](super::workflow::Workflow) を使用してください。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct WorkflowDto {
    /// ワークフローのメタデータ
    pub(super) workflow: WorkflowMetadataDto,
    /// ステップの配列
    pub(super) steps: Vec<WorkflowStepDto>,
}

/// ワークフローメタデータ DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct WorkflowMetadataDto {
    /// ワークフロー名 (必須)
    pub(super) name: String,
    /// 説明 (オプション)
    #[serde(default)]
    pub(super) description: Option<String>,
    /// バージョン (オプション)
    #[serde(default)]
    pub(super) version: Option<String>,
}

/// ワークフローステップ DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct WorkflowStepDto {
    /// ステップ名 (必須)
    pub(super) name: String,
    /// システムプロンプト (必須)
    pub(super) system_prompt: String,
    /// プロバイダー (必須)
    pub(super) provider: String,
    /// モデルティア (必須)
    pub(super) model_tier: String,
    /// タイムアウト秒数 (オプション)
    #[serde(default)]
    pub(super) timeout: Option<u64>,
    /// リトライ回数 (オプション)
    #[serde(default)]
    pub(super) retry_count: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_workflow_dto() {
        let toml = r#"
[workflow]
name = "test"
description = "test workflow"

[[steps]]
name = "step1"
system_prompt = "test prompt"
provider = "anthropic"
model_tier = "heavy"
"#;

        let dto: WorkflowDto = toml::from_str(toml).expect("Failed to deserialize TOML");

        assert_eq!(dto.workflow.name, "test");
        assert_eq!(dto.workflow.description, Some("test workflow".to_string()));
        assert_eq!(dto.workflow.version, None);

        assert_eq!(dto.steps.len(), 1);
        assert_eq!(dto.steps[0].name, "step1");
        assert_eq!(dto.steps[0].system_prompt, "test prompt");
        assert_eq!(dto.steps[0].provider, "anthropic");
        assert_eq!(dto.steps[0].model_tier, "heavy");
        assert_eq!(dto.steps[0].timeout, None);
        assert_eq!(dto.steps[0].retry_count, None);
    }

    #[test]
    fn test_deserialize_workflow_dto_with_optional_fields() {
        let toml = r#"
[workflow]
name = "test"
description = "test workflow"
version = "1.0.0"

[[steps]]
name = "step1"
system_prompt = "test prompt"
provider = "anthropic"
model_tier = "heavy"
timeout = 60
retry_count = 3
"#;

        let dto: WorkflowDto = toml::from_str(toml).expect("Failed to deserialize TOML");

        assert_eq!(dto.workflow.name, "test");
        assert_eq!(dto.workflow.description, Some("test workflow".to_string()));
        assert_eq!(dto.workflow.version, Some("1.0.0".to_string()));

        assert_eq!(dto.steps.len(), 1);
        assert_eq!(dto.steps[0].name, "step1");
        assert_eq!(dto.steps[0].system_prompt, "test prompt");
        assert_eq!(dto.steps[0].provider, "anthropic");
        assert_eq!(dto.steps[0].model_tier, "heavy");
        assert_eq!(dto.steps[0].timeout, Some(60));
        assert_eq!(dto.steps[0].retry_count, Some(3));
    }

    #[test]
    fn test_deserialize_workflow_dto_minimal() {
        let toml = r#"
[workflow]
name = "minimal"

[[steps]]
name = "step1"
system_prompt = "prompt"
provider = "anthropic"
model_tier = "light"
"#;

        let dto: WorkflowDto = toml::from_str(toml).expect("Failed to deserialize TOML");

        assert_eq!(dto.workflow.name, "minimal");
        assert_eq!(dto.workflow.description, None);
        assert_eq!(dto.workflow.version, None);

        assert_eq!(dto.steps.len(), 1);
        assert_eq!(dto.steps[0].name, "step1");
        assert_eq!(dto.steps[0].timeout, None);
        assert_eq!(dto.steps[0].retry_count, None);
    }
}
