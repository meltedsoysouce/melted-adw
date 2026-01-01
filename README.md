# Melted ADW - Agent Development Workflow Builder

Claude Code / Codex を統一的に扱える Workflow Builder

## 概要

Melted ADW は、Coding Agent（Claude Code、Codex など）を活用した開発ワークフローを定義・実行・分析するためのツールです。

「実装計画 → 実装 → レビュー」のような開発の流れを Workflow として定義し、効率的な Agent 駆動開発を実現します。

## 主な機能

- **Workflow 定義**: TOML 形式でワークフローを定義
- **マルチプロバイダー対応**: Anthropic / OpenAI を統一的に扱える抽象化レイヤー
- **ステップ連鎖**: 各ステップの出力を次のステップへ自動的に引き継ぎ
- **テレメトリー収集**: 実行速度・修正回数・コストを計測し、ワークフロー改善に活用

## アーキテクチャ

```
┌─────────────────────────────────────────────────────────────────┐
│                         CLI / API                                │
├─────────────────────────────────────────────────────────────────┤
│                    Workflow Engine                               │
│  ┌───────────┐  ┌───────────┐  ┌───────────┐  ┌───────────┐    │
│  │  Step 1   │→ │  Step 2   │→ │  Step 3   │→ │  Step N   │    │
│  │ (計画)    │  │ (実装)    │  │ (レビュー) │  │   ...     │    │
│  └───────────┘  └───────────┘  └───────────┘  └───────────┘    │
├─────────────────────────────────────────────────────────────────┤
│                   Provider Abstraction Layer                     │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │   Anthropic     │  │     OpenAI      │  │    Future...    │  │
│  │  (Claude Code)  │  │    (Codex)      │  │                 │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘  │
├─────────────────────────────────────────────────────────────────┤
│                     Telemetry Collector                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │
│  │ 実行速度    │  │ 修正回数    │  │ トークン数  │              │
│  └─────────────┘  └─────────────┘  └─────────────┘              │
│                           ↓                                      │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │  JSON Export  →  DuckDB (将来)                              ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

## ディレクトリ構造

```
melted-adw/
├── README.md
├── Cargo.toml
├── src/
│   ├── main.rs                 # エントリーポイント
│   ├── lib.rs                  # ライブラリルート
│   ├── error.rs                # エラー型定義
│   │
│   ├── cli.rs                  # CLI モジュール定義
│   ├── cli/
│   │   ├── commands.rs         # サブコマンド定義
│   │   └── args.rs             # 引数パーサー
│   │
│   ├── config.rs               # 設定モジュール定義
│   ├── config/
│   │   ├── workflow.rs         # Workflow TOML パーサー
│   │   └── step.rs             # Step 定義
│   │
│   ├── engine.rs               # エンジンモジュール定義
│   ├── engine/
│   │   ├── executor.rs         # ステップ実行ロジック
│   │   ├── context.rs          # 実行コンテキスト（ステップ間データ受け渡し）
│   │   └── result.rs           # 実行結果
│   │
│   ├── provider.rs             # プロバイダーモジュール定義
│   ├── provider/
│   │   ├── traits.rs           # Provider トレイト定義
│   │   ├── anthropic.rs        # Anthropic (Claude Code) 実装
│   │   ├── openai.rs           # OpenAI (Codex) 実装
│   │   └── model_tier.rs       # Heavy/Medium/Light モデル抽象化
│   │
│   ├── telemetry.rs            # テレメトリーモジュール定義
│   └── telemetry/
│       ├── collector.rs        # メトリクス収集
│       ├── metrics.rs          # KPI 定義（速度・修正数・コスト）
│       ├── exporter.rs         # エクスポーターモジュール定義
│       └── exporter/
│           ├── json.rs         # JSON エクスポート
│           └── duckdb.rs       # DuckDB エクスポート（将来）
│
├── workflows/                  # サンプルワークフロー
│   ├── default.toml            # デフォルトワークフロー
│   └── examples/
│       ├── implement.toml      # 実装ワークフロー例
│       └── review.toml         # レビューワークフロー例
│
├── telemetry/                  # テレメトリー出力先
│   └── .gitkeep
│
└── tests/                      # テスト
    ├── integration/
    └── fixtures/
```

## Workflow 定義例 (TOML)

```toml
[workflow]
name = "feature-implementation"
description = "新機能の実装ワークフロー"

[[steps]]
name = "plan"
system_prompt = """
あなたは優秀なソフトウェアアーキテクトです。
与えられた要件に対して、実装計画を作成してください。
"""
provider = "anthropic"
model_tier = "heavy"  # Heavy | Medium | Light

[[steps]]
name = "implement"
system_prompt = """
あなたは優秀なソフトウェアエンジニアです。
与えられた計画に基づいて、コードを実装してください。
"""
provider = "anthropic"
model_tier = "heavy"

[[steps]]
name = "review"
system_prompt = """
あなたはコードレビュアーです。
実装されたコードをレビューし、問題点があれば指摘してください。
"""
provider = "openai"
model_tier = "medium"
```

## モデルティア

各プロバイダーのモデルを抽象化し、用途に応じて選択可能にします。

| Tier   | 用途           | Anthropic        | OpenAI         |
|--------|---------------|------------------|----------------|
| Heavy  | 複雑な推論     | claude-opus-4    | o1 / o3        |
| Medium | 一般的なタスク | claude-sonnet-4  | gpt-4o         |
| Light  | 簡単なタスク   | claude-haiku     | gpt-4o-mini    |

## テレメトリー KPI

ワークフローの改善のため、以下の指標を収集・分析します。

1. **実行速度**: 指示から完成までの時間（短いほど良い）
2. **人の手による修正**: 成果物に対する手動修正回数（0 が理想）
3. **実行コスト**: 消費トークン数（少ないほど良い）

## 技術スタック

- **言語**: Rust
- **CLI**: clap
- **設定**: toml
- **非同期**: tokio
- **HTTP**: reqwest
- **テレメトリー**: serde_json → DuckDB（将来）
