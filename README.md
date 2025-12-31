# melted-adw

Coding Agentを利用する、ワークフローを構築するCLI

## Words

- Workflow : 複数のCoding Agentで構築される、一連の開発の流れ
- Coding Agent : CLIで起動するLLMよるコーディングツール

## Features

### TOML define

TOMLファイルを使い、Workflowを定義できる。

### Teremetry

Coding Agentの作業内容やコストを計測する。
Teremetryデータを利用することで、Workflowの改善をする

## Architecture

以下のディレクトリで構築する

- specs : 仕様/機能のドキュメント
- src : プログラムのコード本体
  - cli : コマンドライン引数をパースし、プログラムに渡すモジュール
  - definitions : Workflowの定義体を提供するモジュール
  - provider : Coding Agentのプロバイダー情報を提供するモジュール
    - anthropic : Anthropic社のCoding AgentであるClaude Codeの操作を提供するモジュール
    - openai : OpenAI社のCoding AgentであるCodexの操作を提供するモジュール
  - agent : Coding Agentへの操作を抽象するモジュール。複数のCLIを、抽象して扱うので機能をまとめる
  - teremetry : ログ収集を責務とするモジュール。最終的にユーザーにCoding Agentの動きの内容を提供し、改善を促す
- as-is : 現状のプロジェクトのコード内容をドキュメントとして保持する。内容把握用
