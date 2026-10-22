# SASHIMI — 実験的なx86-64アセンブラ

[English](README.en.md)

Ceruneから切り出した、外部crate依存のないRust製アセンブラです。Ceruneが生成する限定的なGNU形式ASMを読み、Linux向けELF64／Windows向けCOFFを生成します。

| 現在使える機能 | 未実装 |
| --- | --- |
| `assemble(&str, Target)`、明示target、命令符号化・ラベル・再配置・Object出力 | 型付き入力、出自対応表、CLI、汎用GNU assembler互換、動的リンク、安定API |

実験段階・未リリースです。2026-10-23に独立Rustテスト3件のローカル成功を確認しました。[Cerune PR #184](https://github.com/Cerune-Lang/Cerune/pull/184)は固定版接続と移行比較を追加します。

```rust
use sashimi_assembler::{assemble, Target};
let obj = assemble(".text\n.globl main\nmain:\nretq\n", Target::X86_64UnknownLinuxGnu)?;
assert_eq!(&obj[0..4], b"\x7fELF");
# Ok::<(), sashimi_assembler::Diagnostic>(())
```

検証は `cargo test --all-targets` と `python3 scripts/verify_snapshot.py` です。GitHub Actionsでも実行します。snapshot検査はGit原本のバイト列を比較するため、改行を変換しないcheckoutを使用してください。

## 設計と出典

次の作業は[型付き機械命令と生成位置の契約](docs/design/typed-assembly.ja.md)です。API名を含め未実装の設計案であり、現在のASM入力は維持します。Cerune言語への移植は[Issue #2](https://github.com/Cerune-Lang/SASHIMI/issues/2)の別段階です。

`src/object.rs` と `src/object/{parse,encode,format}.rs` は[Ceruneの切り出し元](https://github.com/Cerune-Lang/Cerune/tree/f48a72c01f33ff32f483a8c125d0b69b4f7d2444/src/codegen/x86_64)からのバイト単位のコピーです。`SOURCE_SNAPSHOT.json` が各Git blob SHA-1を固定し、`src/lib.rs` が独立したtarget・診断型を提供します。独立テストは小さな一部で、Cerune依存の統合テストの移植は未完了です。

原著作権：Copyright (c) 2026 Hokutaka。MIT OR Apache-2.0。ライセンス全文は同梱ファイルを参照してください。
