---
name: selfhost-stage1
depth: Minimal
testStrategy: Standard
keywords: []
description: 既存コードの契約差分を実装・検証し、stage-1 のセルフホスト運用へ切り替える
---

# selfhost-stage1 スコープ

チーム規則の再確認、要求差分、作業分割、契約差分、直列実行計画、TDD による実装、検証、実地切替を行う。
既存コードの再設計・再文書化は行わず、必要な経路だけを現行 main から実測する。
ゴールデンの基準版と実行に必要な動詞・受領記録の範囲は、人間の裁定を受けて確定する。

工程は承認済みの `scope-grid.json` エントリを正とする。初期化を含めて 11 工程を実施し、22 工程を省略する。
テスト範囲は Standard、実装順序は TDD とし、既存のカバレッジ床 90%・Quint/ITF・ゴールデン・CI を維持する。
キーワード推論には参加せず、`--scope selfhost-stage1` で明示的に選択する。
