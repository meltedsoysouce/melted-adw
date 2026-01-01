# Config Module Implementation Plan

## Overview

### As-Is (現状)
- エラー型定義: 完全実装済み
- モジュール構造: 完全設計済み
- 列挙型 (Provider, ModelTier): 完全実装済み
- DTO/ドメインモデル: 骨組みのみ (フィールド未定義)
- バリデーション・I/O: 未実装

### To-Be (終了後の状態)
TOML形式で定義されたワークフローを読み込み、バリデーション済みのドメインモデルとして提供できる状態。

具体的には:
1. TOMLファイルの読み込み・書き込みが可能
2. TOML ↔ Rust型の双方向変換が実装されている
3. バリデーションロジックが実装され、不正なデータを検出できる
4. ドメインモデルがビジネスロジックを保持し、エンジンから利用可能

### Gap Analysis (差分)

| コンポーネント | 現状 | 目標 | 必要な作業 |
|-------------|------|------|-----------|
| WorkflowMetadataDto | 骨組みのみ | フィールド定義済み | フィールド設計・serde属性追加 |
| WorkflowStepDto | 骨組みのみ | フィールド定義済み | フィールド設計・serde属性追加 |
| WorkflowDto | 構造体定義済み | 完全実装 | 特になし |
| WorkflowStep | 骨組みのみ | フィールド定義・変換実装 | フィールド設計・TryFrom/From実装 |
| Workflow | 骨組みのみ | フィールド定義・I/O・変換実装 | フィールド設計・全メソッド実装 |

## Implementation Phases

実装を4つのフェーズに分割します。各フェーズは独立したエージェントによって実装可能です。

### Phase 1: DTO Field Definition
**目的**: TOMLデシリアライズ用のDTOフィールドを定義

**作業内容**:
1. `WorkflowMetadataDto` のフィールド設計・実装
   - ファイル: `src/config/dto.rs:42-45`
   - 必要なフィールド:
     - `name: String` - ワークフロー名 (必須)
     - `description: Option<String>` - 説明 (オプション)
     - `version: Option<String>` - バージョン (オプション)
   - serde属性の追加 (必要に応じて)

2. `WorkflowStepDto` のフィールド設計・実装
   - ファイル: `src/config/dto.rs:47-51`
   - 必要なフィールド:
     - `name: String` - ステップ名 (必須)
     - `system_prompt: String` - システムプロンプト (必須)
     - `provider: String` - プロバイダー (必須、文字列として受け取り)
     - `model_tier: String` - モデルティア (必須、文字列として受け取り)
     - `timeout: Option<u64>` - タイムアウト秒数 (オプション)
     - `retry_count: Option<u32>` - リトライ回数 (オプション)
   - serde属性の追加 (必要に応じて)

**成果物**:
- `src/config/dto.rs` の完全実装

**依存関係**: なし

**Context Windowへの影響**: 小 (1ファイルのみの変更)

---

### Phase 2: Domain Model Field Definition
**目的**: ドメインモデルのフィールドを定義

**作業内容**:
1. `WorkflowStep` のフィールド設計・実装
   - ファイル: `src/config/step.rs:23-25`
   - 必要なフィールド:
     - `name: String` - ステップ名
     - `system_prompt: String` - システムプロンプト
     - `provider: Provider` - プロバイダー (enum)
     - `model_tier: ModelTier` - モデルティア (enum)
     - `timeout: Option<u64>` - タイムアウト秒数
     - `retry_count: Option<u32>` - リトライ回数
   - 必要なメソッドの追加 (getter等)

2. `Workflow` のフィールド設計・実装
   - ファイル: `src/config/workflow.rs:66-69`
   - 必要なフィールド:
     - `name: String` - ワークフロー名
     - `description: Option<String>` - 説明
     - `version: Option<String>` - バージョン
     - `steps: Vec<WorkflowStep>` - ステップ配列
   - 必要なメソッドの追加 (getter等)

**成果物**:
- `src/config/step.rs` のドメインモデル定義完了
- `src/config/workflow.rs` のドメインモデル定義完了

**依存関係**: Phase 1完了後

**Context Windowへの影響**: 小 (2ファイルの変更)

---

### Phase 3: Validation and Conversion Logic
**目的**: DTO ↔ ドメインモデル間の変換とバリデーションを実装

**作業内容**:
1. `TryFrom<WorkflowStepDto> for WorkflowStep` の実装
   - ファイル: `src/config/step.rs:52-59`
   - バリデーション内容:
     - ステップ名の存在確認・形式チェック
     - システムプロンプトの存在確認・長さチェック
     - provider文字列 → Provider enum 変換
     - model_tier文字列 → ModelTier enum 変換
     - 無効な値の検出とエラー返却

2. `From<WorkflowStep> for WorkflowStepDto` の実装
   - ファイル: `src/config/step.rs:65-70`
   - ドメインモデル → DTO 変換 (常に成功)

3. `TryFrom<WorkflowDto> for Workflow` の実装
   - ファイル: `src/config/workflow.rs:159-166`
   - バリデーション内容:
     - ワークフロー名の存在確認・形式チェック
     - ステップリストの非空チェック
     - ステップ名の一意性確認
     - 各ステップの変換 (WorkflowStepDto → WorkflowStep)
     - 無効な値の検出とエラー返却

4. `From<Workflow> for WorkflowDto` の実装
   - ファイル: `src/config/workflow.rs:180-185`
   - ドメインモデル → DTO 変換 (常に成功)

**成果物**:
- バリデーションロジック実装完了
- 双方向変換実装完了

**依存関係**: Phase 1, Phase 2完了後

**Context Windowへの影響**: 中 (2ファイル、複雑なロジック)

**注意事項**:
- バリデーションエラーは `ConfigError::Validation` を使用
- エラーメッセージは具体的かつ明確に

---

### Phase 4: File I/O and Serialization
**目的**: ファイル読み込み・書き込み機能の実装

**作業内容**:
1. `Workflow::from_file()` の実装
   - ファイル: `src/config/workflow.rs:88-91`
   - 処理フロー:
     1. ファイル読み込み (std::fs::read_to_string)
     2. `from_str()` 呼び出し
   - エラーハンドリング: `ConfigError::FileRead`

2. `Workflow::from_str()` の実装
   - ファイル: `src/config/workflow.rs:108-111`
   - 処理フロー:
     1. TOML デシリアライズ (toml::from_str → WorkflowDto)
     2. バリデーション (WorkflowDto.try_into() → Workflow)
   - エラーハンドリング:
     - `ConfigError::TomlDeserialize` (デシリアライズ失敗)
     - `ConfigError::Validation` (バリデーション失敗)

3. `Workflow::to_string()` の実装
   - ファイル: `src/config/workflow.rs:124-127`
   - 処理フロー:
     1. ドメインモデル → DTO 変換 (Workflow.into() → WorkflowDto)
     2. TOML シリアライズ (toml::to_string)
   - エラーハンドリング: `ConfigError::TomlSerialize`

4. `Workflow::to_file()` の実装
   - ファイル: `src/config/workflow.rs:144-147`
   - 処理フロー:
     1. `to_string()` 呼び出し
     2. ファイル書き込み (std::fs::write)
   - エラーハンドリング: `ConfigError::FileRead`

**成果物**:
- ファイルI/O機能の完全実装
- TOML ↔ ドメインモデル変換の完全実装

**依存関係**: Phase 3完了後

**Context Windowへの影響**: 小 (1ファイル、シンプルなI/O処理)

---

## Testing Strategy

各フェーズ完了後に、以下のテストを実施することを推奨:

### Phase 1後のテスト
```rust
#[test]
fn test_dto_deserialize() {
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

    let dto: WorkflowDto = toml::from_str(toml).unwrap();
    assert_eq!(dto.workflow.name, "test");
    assert_eq!(dto.steps.len(), 1);
}
```

### Phase 2後のテスト
- ドメインモデルのフィールドアクセステスト
- getter/setterの動作確認

### Phase 3後のテスト
```rust
#[test]
fn test_validation_success() {
    // 正常系: 有効なDTOからドメインモデルへの変換
}

#[test]
fn test_validation_empty_name() {
    // 異常系: 空のワークフロー名
}

#[test]
fn test_validation_invalid_provider() {
    // 異常系: 不正なプロバイダー名
}

#[test]
fn test_validation_duplicate_step_names() {
    // 異常系: 重複するステップ名
}
```

### Phase 4後のテスト
```rust
#[test]
fn test_roundtrip() {
    // ファイル → ドメインモデル → ファイル のラウンドトリップテスト
    let workflow = Workflow::from_file("workflows/default.toml").unwrap();
    workflow.to_file("/tmp/test.toml").unwrap();
    let workflow2 = Workflow::from_file("/tmp/test.toml").unwrap();
    // workflow と workflow2 の内容が一致することを確認
}
```

---

## Implementation Order

```
Phase 1: DTO Field Definition
   ↓ (DTOが完成)
Phase 2: Domain Model Field Definition
   ↓ (ドメインモデルの構造が確定)
Phase 3: Validation and Conversion Logic
   ↓ (変換ロジックが完成)
Phase 4: File I/O and Serialization
   ↓ (全機能完成)
Testing & Integration
```

---

## Success Criteria

実装完了の判断基準:

1. **Phase 1**: DTOが完全に定義され、TOMLデシリアライズが成功する
2. **Phase 2**: ドメインモデルのフィールドが定義され、アクセス可能
3. **Phase 3**: バリデーションが動作し、不正なデータを検出できる
4. **Phase 4**: ファイルからの読み込み・書き込みが動作する
5. **Integration**: `workflows/default.toml` を読み込み、エンジンで使用できる

---

## Risk Management

### リスク1: TOML構造の変更
- **対策**: DTOパターンにより、TOML変更の影響をDTOレイヤーに閉じ込める

### リスク2: バリデーションロジックの複雑化
- **対策**: Phase 3を独立したフェーズとし、十分な時間を確保

### リスク3: Context Windowの超過
- **対策**: 各フェーズを小規模に保ち、別エージェントで実行可能にする

---

## Notes

- **原則crateの追加は禁止**: 現在の依存関係 (toml, serde, thiserror) で実装可能
- **エラーハンドリング**: 既存の `ConfigError` を活用
- **コードスタイル**: 既存コードのドキュメントスタイルに準拠

---

## Appendix: File Structure

実装対象ファイル一覧:

```
src/
├── config.rs               (変更なし)
├── config/
│   ├── dto.rs             [Phase 1] フィールド定義
│   ├── step.rs            [Phase 2, 3] フィールド定義・変換実装
│   └── workflow.rs        [Phase 2, 3, 4] フィールド定義・変換・I/O実装
└── error.rs               (変更なし)
```

各フェーズで変更するファイル数:
- Phase 1: 1ファイル (dto.rs)
- Phase 2: 2ファイル (step.rs, workflow.rs)
- Phase 3: 2ファイル (step.rs, workflow.rs)
- Phase 4: 1ファイル (workflow.rs)
