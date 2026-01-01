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
    // TODO: フィールドは後から設計を詰める
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
        // TODO: バリデーションと変換ロジックは後から設計を詰める
        todo!()
    }
}

/// ドメインモデルから DTO への変換（書き込み方向）
///
/// バリデーション済みのドメインモデルから DTO を生成するため、
/// この変換は失敗しません（`From` トレイトを使用）。
impl From<WorkflowStep> for WorkflowStepDto {
    fn from(step: WorkflowStep) -> Self {
        // TODO: 変換ロジックは後から設計を詰める
        todo!()
    }
}
