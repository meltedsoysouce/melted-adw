//! ワークフローステップの定義
//!
//! # 責務
//!
//! Workflowを構成するStepの定義体を提供するモジュール
//! アプリケーションに対して、[WorkflowStep] を提供する。

use serde::{Deserialize, Serialize};

use crate::error::ConfigError;
use super::dto::WorkflowStepDto;

/// ワークフローステップ（ドメインモデル）
///
/// ワークフロー内の1つの処理単位を表します。
/// 各ステップは、特定のプロバイダーとモデルを使用してタスクを実行します。
///
/// ## DTO との違い
///
/// - [`WorkflowStepDto`](super::dto::WorkflowStepDto): TOML デシリアライズ専用
/// - [`WorkflowStep`]: バリデーション済み、ドメインロジックを持つ
#[derive(Debug, Clone)]
pub struct WorkflowStep {
    /// ステップ名
    name: String,
    /// システムプロンプト
    system_prompt: String,
    /// プロバイダー
    provider: Provider,
    /// モデルティア
    model_tier: ModelTier,
    /// タイムアウト秒数 (オプション)
    timeout: Option<u64>,
    /// リトライ回数 (オプション)
    retry_count: Option<u32>,
}

impl WorkflowStep {
    /// ステップ名を取得
    pub fn name(&self) -> &str {
        &self.name
    }

    /// システムプロンプトを取得
    pub fn system_prompt(&self) -> &str {
        &self.system_prompt
    }

    /// プロバイダーを取得
    pub fn provider(&self) -> &Provider {
        &self.provider
    }

    /// モデルティアを取得
    pub fn model_tier(&self) -> &ModelTier {
        &self.model_tier
    }

    /// タイムアウト秒数を取得
    pub fn timeout(&self) -> Option<u64> {
        self.timeout
    }

    /// リトライ回数を取得
    pub fn retry_count(&self) -> Option<u32> {
        self.retry_count
    }
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

/// AI プロバイダー
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    /// Anthropic (Claude Code)
    Anthropic,
    /// OpenAI (Codex)
    OpenAI,
}

/// DTO からドメインモデルへの変換（読み込み方向）
///
/// バリデーションを実施し、不正なデータの場合は [`ConfigError::Validation`] を返します。
impl TryFrom<WorkflowStepDto> for WorkflowStep {
    type Error = ConfigError;

    fn try_from(dto: WorkflowStepDto) -> Result<Self, Self::Error> {
        // ステップ名のバリデーション
        if dto.name.trim().is_empty() {
            return Err(ConfigError::Validation(
                "ステップ名が空です".to_string()
            ));
        }

        // システムプロンプトのバリデーション
        if dto.system_prompt.trim().is_empty() {
            return Err(ConfigError::Validation(
                format!("ステップ '{}' のシステムプロンプトが空です", dto.name)
            ));
        }

        // システムプロンプトの長さチェック（10000文字を上限とする）
        if dto.system_prompt.len() > 10000 {
            return Err(ConfigError::Validation(
                format!("ステップ '{}' のシステムプロンプトが長すぎます（最大10000文字）", dto.name)
            ));
        }

        // プロバイダーの変換
        let provider = match dto.provider.to_lowercase().as_str() {
            "anthropic" => Provider::Anthropic,
            "openai" => Provider::OpenAI,
            _ => {
                return Err(ConfigError::Validation(
                    format!(
                        "ステップ '{}' の不正なプロバイダー: '{}' (有効な値: anthropic, openai)",
                        dto.name, dto.provider
                    )
                ));
            }
        };

        // モデルティアの変換
        let model_tier = match dto.model_tier.to_lowercase().as_str() {
            "heavy" => ModelTier::Heavy,
            "medium" => ModelTier::Medium,
            "light" => ModelTier::Light,
            _ => {
                return Err(ConfigError::Validation(
                    format!(
                        "ステップ '{}' の不正なモデルティア: '{}' (有効な値: heavy, medium, light)",
                        dto.name, dto.model_tier
                    )
                ));
            }
        };

        Ok(WorkflowStep {
            name: dto.name,
            system_prompt: dto.system_prompt,
            provider,
            model_tier,
            timeout: dto.timeout,
            retry_count: dto.retry_count,
        })
    }
}

/// ドメインモデルから DTO への変換（書き込み方向）
///
/// バリデーション済みのドメインモデルから DTO を生成するため、
/// この変換は失敗しません（`From` トレイトを使用）。
impl From<WorkflowStep> for WorkflowStepDto {
    fn from(step: WorkflowStep) -> Self {
        // Enum を文字列に変換（serde の lowercase と同じ形式）
        let provider = match step.provider {
            Provider::Anthropic => "anthropic".to_string(),
            Provider::OpenAI => "openai".to_string(),
        };

        let model_tier = match step.model_tier {
            ModelTier::Heavy => "heavy".to_string(),
            ModelTier::Medium => "medium".to_string(),
            ModelTier::Light => "light".to_string(),
        };

        WorkflowStepDto {
            name: step.name,
            system_prompt: step.system_prompt,
            provider,
            model_tier,
            timeout: step.timeout,
            retry_count: step.retry_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_success() {
        // 正常系: 有効なDTOからドメインモデルへの変換
        let dto = WorkflowStepDto {
            name: "test_step".to_string(),
            system_prompt: "This is a test prompt".to_string(),
            provider: "anthropic".to_string(),
            model_tier: "heavy".to_string(),
            timeout: Some(60),
            retry_count: Some(3),
        };

        let result = WorkflowStep::try_from(dto);
        assert!(result.is_ok());

        let step = result.unwrap();
        assert_eq!(step.name(), "test_step");
        assert_eq!(step.system_prompt(), "This is a test prompt");
        assert_eq!(step.timeout(), Some(60));
        assert_eq!(step.retry_count(), Some(3));
    }

    #[test]
    fn test_validation_success_all_providers_and_tiers() {
        // すべてのプロバイダーとティアの組み合わせをテスト
        let providers = vec!["anthropic", "openai", "Anthropic", "OpenAI", "ANTHROPIC"];
        let tiers = vec!["heavy", "medium", "light", "Heavy", "LIGHT"];

        for provider in providers {
            for tier in &tiers {
                let dto = WorkflowStepDto {
                    name: "step".to_string(),
                    system_prompt: "prompt".to_string(),
                    provider: provider.to_string(),
                    model_tier: tier.to_string(),
                    timeout: None,
                    retry_count: None,
                };

                let result = WorkflowStep::try_from(dto);
                assert!(
                    result.is_ok(),
                    "Failed for provider: {}, tier: {}",
                    provider,
                    tier
                );
            }
        }
    }

    #[test]
    fn test_validation_empty_step_name() {
        // 異常系: 空のステップ名
        let dto = WorkflowStepDto {
            name: "".to_string(),
            system_prompt: "prompt".to_string(),
            provider: "anthropic".to_string(),
            model_tier: "heavy".to_string(),
            timeout: None,
            retry_count: None,
        };

        let result = WorkflowStep::try_from(dto);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConfigError::Validation(_)));
    }

    #[test]
    fn test_validation_whitespace_step_name() {
        // 異常系: 空白のみのステップ名
        let dto = WorkflowStepDto {
            name: "   ".to_string(),
            system_prompt: "prompt".to_string(),
            provider: "anthropic".to_string(),
            model_tier: "heavy".to_string(),
            timeout: None,
            retry_count: None,
        };

        let result = WorkflowStep::try_from(dto);
        assert!(result.is_err());
        if let Err(ConfigError::Validation(msg)) = result {
            assert!(msg.contains("ステップ名が空です"));
        } else {
            panic!("Expected Validation error");
        }
    }

    #[test]
    fn test_validation_empty_system_prompt() {
        // 異常系: 空のシステムプロンプト
        let dto = WorkflowStepDto {
            name: "step".to_string(),
            system_prompt: "".to_string(),
            provider: "anthropic".to_string(),
            model_tier: "heavy".to_string(),
            timeout: None,
            retry_count: None,
        };

        let result = WorkflowStep::try_from(dto);
        assert!(result.is_err());
        if let Err(ConfigError::Validation(msg)) = result {
            assert!(msg.contains("システムプロンプトが空です"));
        } else {
            panic!("Expected Validation error");
        }
    }

    #[test]
    fn test_validation_system_prompt_too_long() {
        // 異常系: システムプロンプトが長すぎる
        let long_prompt = "a".repeat(10001);
        let dto = WorkflowStepDto {
            name: "step".to_string(),
            system_prompt: long_prompt,
            provider: "anthropic".to_string(),
            model_tier: "heavy".to_string(),
            timeout: None,
            retry_count: None,
        };

        let result = WorkflowStep::try_from(dto);
        assert!(result.is_err());
        if let Err(ConfigError::Validation(msg)) = result {
            assert!(msg.contains("長すぎます"));
        } else {
            panic!("Expected Validation error");
        }
    }

    #[test]
    fn test_validation_invalid_provider() {
        // 異常系: 不正なプロバイダー名
        let dto = WorkflowStepDto {
            name: "step".to_string(),
            system_prompt: "prompt".to_string(),
            provider: "invalid_provider".to_string(),
            model_tier: "heavy".to_string(),
            timeout: None,
            retry_count: None,
        };

        let result = WorkflowStep::try_from(dto);
        assert!(result.is_err());
        if let Err(ConfigError::Validation(msg)) = result {
            assert!(msg.contains("不正なプロバイダー"));
            assert!(msg.contains("invalid_provider"));
        } else {
            panic!("Expected Validation error");
        }
    }

    #[test]
    fn test_validation_invalid_model_tier() {
        // 異常系: 不正なモデルティア
        let dto = WorkflowStepDto {
            name: "step".to_string(),
            system_prompt: "prompt".to_string(),
            provider: "anthropic".to_string(),
            model_tier: "invalid_tier".to_string(),
            timeout: None,
            retry_count: None,
        };

        let result = WorkflowStep::try_from(dto);
        assert!(result.is_err());
        if let Err(ConfigError::Validation(msg)) = result {
            assert!(msg.contains("不正なモデルティア"));
            assert!(msg.contains("invalid_tier"));
        } else {
            panic!("Expected Validation error");
        }
    }

    #[test]
    fn test_round_trip_conversion() {
        // 正常系: DTO → ドメインモデル → DTO の往復変換
        let original_dto = WorkflowStepDto {
            name: "test_step".to_string(),
            system_prompt: "Test prompt".to_string(),
            provider: "anthropic".to_string(),
            model_tier: "medium".to_string(),
            timeout: Some(120),
            retry_count: Some(5),
        };

        // DTO → ドメインモデル
        let step = WorkflowStep::try_from(original_dto.clone()).unwrap();

        // ドメインモデル → DTO
        let converted_dto: WorkflowStepDto = step.into();

        // 変換後のDTOが元と同じであることを確認
        assert_eq!(converted_dto.name, original_dto.name);
        assert_eq!(converted_dto.system_prompt, original_dto.system_prompt);
        assert_eq!(converted_dto.provider, original_dto.provider);
        assert_eq!(converted_dto.model_tier, original_dto.model_tier);
        assert_eq!(converted_dto.timeout, original_dto.timeout);
        assert_eq!(converted_dto.retry_count, original_dto.retry_count);
    }

    #[test]
    fn test_case_insensitive_provider_conversion() {
        // 大文字小文字を区別しないプロバイダー変換のテスト
        let dto = WorkflowStepDto {
            name: "step".to_string(),
            system_prompt: "prompt".to_string(),
            provider: "ANTHROPIC".to_string(),
            model_tier: "heavy".to_string(),
            timeout: None,
            retry_count: None,
        };

        let step = WorkflowStep::try_from(dto).unwrap();
        let converted: WorkflowStepDto = step.into();

        // 小文字に正規化されることを確認
        assert_eq!(converted.provider, "anthropic");
    }
}
