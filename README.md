# SASHIMI — 実験的なx86-64アセンブラ

[English](README.en.md)

Ceruneから切り出した、外部crate依存のないRust製アセンブラです。型付きProgramまたはCeruneが生成する限定的なGNU形式ASMを受け取り、Linux向けELF64／Windows向けCOFFを生成します。

| 現在使える機能 | 未実装 |
| --- | --- |
| `assemble(&str, Target)`、明示target、命令符号化・ラベル・再配置・Object出力、[型付き命令](docs/design/typed-instructions.ja.md)・[Program/Object・出自対応表](docs/design/typed-program.ja.md) | CLI、汎用GNU assembler互換、動的リンク、安定API |

実験段階・未リリースです。Windows/Linuxで単体テスト21件、ASM/Objectの外部リンク・実行比較、API文書例を検証します。Ceruneは固定コミットへの接続と移行比較を導入済みです。

```rust
use sashimi_assembler::{assemble, Target};
let obj = assemble(".text\n.globl main\nmain:\nretq\n", Target::X86_64UnknownLinuxGnu)?;
assert_eq!(&obj[0..4], b"\x7fELF");
# Ok::<(), sashimi_assembler::Diagnostic>(())
```

検証は `SASHIMI_TEST_CLANG=clang cargo test --all-targets`（PowerShellでは環境変数を先に設定）と `python3 scripts/verify_snapshot.py` です。GitHub Actionsでも実行します。snapshot検査はGit原本のバイト列を比較するため、改行を変換しないcheckoutを使用してください。

## 設計と出典

[型付き機械命令と生成位置の契約](docs/design/typed-assembly.ja.md)に沿い、単一命令・Program・Object・出自対応を実装しました。次はCeruneの命令選択を型付き構築へ移します。現在のASM入力は維持します。Cerune言語への移植は[Issue #2](https://github.com/Cerune-Lang/SASHIMI/issues/2)の別段階です。

[固定fixture](tests/fixtures/extraction/README.md)の4ファイルは[Ceruneの切り出し元](https://github.com/Cerune-Lang/Cerune/tree/f48a72c01f33ff32f483a8c125d0b69b4f7d2444/src/codegen/x86_64)からのバイト単位のコピーです。`SOURCE_SNAPSHOT.json` が各Git blob SHA-1を固定し、`src/lib.rs` が独立したtarget・診断型を提供します。型付き命令・原本比較のテストを追加しました。Cerune依存の統合テストの移植は未完了です。

原著作権：Copyright (c) 2026 Hokutaka。MIT OR Apache-2.0。ライセンス全文は同梱ファイルを参照してください。
