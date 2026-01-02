//! モデルティアマッピング
//!
//! # 責務
//!
//! - [`ModelTier`] と [`Provider`] の組み合わせから、実際のモデル名を解決
//! - プロバイダー別のモデル名定数を管理
//!
//! # マッピング表
//!
//! | Tier   | Anthropic       | OpenAI      |
//! |--------|----------------|-------------|
//! | Heavy  | claude-opus-4  | o1          |
//! | Medium | claude-sonnet-4-5 | gpt-4o   |
//! | Light  | claude-haiku   | gpt-4o-mini |
//!
//! # 注意
//!
//! モデル名はCLIツールが受け付ける形式に合わせています。
//! CLIツールのバージョンやAPI仕様の変更により、モデル名が変わる可能性があります。
//!
//! # 使用例
//!
//! ```rust
//! use melted_adw::provider::model_tier::resolve_model;
//! use melted_adw::config::step::{Provider, ModelTier};
//!
//! let model = resolve_model(&Provider::Anthropic, &ModelTier::Medium);
//! assert_eq!(model, "claude-sonnet-4-5");
//! ```

use crate::config::step::{Provider, ModelTier};

// Anthropic モデル名定数
const ANTHROPIC_HEAVY: &str = "claude-opus-4";
const ANTHROPIC_MEDIUM: &str = "claude-sonnet-4-5";
const ANTHROPIC_LIGHT: &str = "claude-haiku";

// OpenAI モデル名定数
const OPENAI_HEAVY: &str = "o1";
const OPENAI_MEDIUM: &str = "gpt-4o";
const OPENAI_LIGHT: &str = "gpt-4o-mini";

/// モデルティアとプロバイダーから実際のモデル名を解決する
///
/// # 引数
///
/// - `provider`: プロバイダーの種類（Anthropic or OpenAI）
/// - `tier`: モデルティア（Heavy/Medium/Light）
///
/// # 戻り値
///
/// モデル名の文字列スライス（'static ライフタイム）
///
/// # 例
///
/// ```rust
/// use melted_adw::provider::model_tier::resolve_model;
/// use melted_adw::config::step::{Provider, ModelTier};
///
/// // Anthropic Medium -> claude-sonnet-4-5
/// assert_eq!(
///     resolve_model(&Provider::Anthropic, &ModelTier::Medium),
///     "claude-sonnet-4-5"
/// );
///
/// // OpenAI Heavy -> o1
/// assert_eq!(
///     resolve_model(&Provider::OpenAI, &ModelTier::Heavy),
///     "o1"
/// );
/// ```
pub fn resolve_model(provider: &Provider, tier: &ModelTier) -> &'static str {
    match (provider, tier) {
        // Anthropic
        (Provider::Anthropic, ModelTier::Heavy) => ANTHROPIC_HEAVY,
        (Provider::Anthropic, ModelTier::Medium) => ANTHROPIC_MEDIUM,
        (Provider::Anthropic, ModelTier::Light) => ANTHROPIC_LIGHT,

        // OpenAI
        (Provider::OpenAI, ModelTier::Heavy) => OPENAI_HEAVY,
        (Provider::OpenAI, ModelTier::Medium) => OPENAI_MEDIUM,
        (Provider::OpenAI, ModelTier::Light) => OPENAI_LIGHT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anthropic_models() {
        assert_eq!(resolve_model(&Provider::Anthropic, &ModelTier::Heavy), "claude-opus-4");
        assert_eq!(resolve_model(&Provider::Anthropic, &ModelTier::Medium), "claude-sonnet-4-5");
        assert_eq!(resolve_model(&Provider::Anthropic, &ModelTier::Light), "claude-haiku");
    }

    #[test]
    fn test_openai_models() {
        assert_eq!(resolve_model(&Provider::OpenAI, &ModelTier::Heavy), "o1");
        assert_eq!(resolve_model(&Provider::OpenAI, &ModelTier::Medium), "gpt-4o");
        assert_eq!(resolve_model(&Provider::OpenAI, &ModelTier::Light), "gpt-4o-mini");
    }

    #[test]
    fn test_all_combinations() {
        // すべての組み合わせが正しくマッピングされることを確認
        let providers = [Provider::Anthropic, Provider::OpenAI];
        let tiers = [ModelTier::Heavy, ModelTier::Medium, ModelTier::Light];

        for provider in &providers {
            for tier in &tiers {
                let model = resolve_model(provider, tier);
                assert!(!model.is_empty(), "Model name should not be empty");
            }
        }
    }
}
