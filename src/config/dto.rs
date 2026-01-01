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
#[derive(Debug, Serialize, Deserialize)]
pub(super) struct WorkflowDto {
    /// ワークフローのメタデータ
    pub(super) workflow: WorkflowMetadataDto,
    /// ステップの配列
    pub(super) steps: Vec<WorkflowStepDto>,
}

/// ワークフローメタデータ DTO
#[derive(Debug, Serialize, Deserialize)]
pub(super) struct WorkflowMetadataDto {
    // TODO: フィールドは後から設計を詰める
}

/// ワークフローステップ DTO
#[derive(Debug, Serialize, Deserialize)]
pub(super) struct WorkflowStepDto {
    // TODO: フィールドは後から設計を詰める
}
