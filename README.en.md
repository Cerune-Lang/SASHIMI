# SASHIMI — experimental x86-64 assembler

[日本語](README.md)

A Rust assembler extracted from Cerune, with no external crate dependencies. It reads the limited GNU-style assembly emitted by Cerune and produces Linux ELF64 or Windows COFF objects.

| Available | Not implemented |
| --- | --- |
| `assemble(&str, Target)`, explicit targets, instruction encoding, labels, relocations and objects | Typed input, provenance mappings, CLI, general GNU assembler compatibility, dynamic linking and stable APIs |

Experimental and unreleased. Three standalone Rust tests passed locally on 2026-10-23. [Cerune PR #184](https://github.com/Cerune-Lang/Cerune/pull/184) adds the pinned connection and migration comparisons.

```rust
use sashimi_assembler::{assemble, Target};
let obj = assemble(".text\n.globl main\nmain:\nretq\n", Target::X86_64UnknownLinuxGnu)?;
assert_eq!(&obj[0..4], b"\x7fELF");
# Ok::<(), sashimi_assembler::Diagnostic>(())
```

Run `cargo test --all-targets` and `python3 scripts/verify_snapshot.py`. GitHub Actions runs these checks too. Snapshot verification compares exact Git source bytes; use a checkout without line-ending conversion.

## Design and extraction

The next step is the [typed instruction and generated-location contract](docs/design/typed-assembly.en.md). It is an unimplemented proposal, including API names; the current text API remains supported. Porting to the Cerune language is a separate stage in [Issue #2](https://github.com/Cerune-Lang/SASHIMI/issues/2).

`src/object.rs` and `src/object/{parse,encode,format}.rs` are byte-for-byte copies of the [original Cerune encoder](https://github.com/Cerune-Lang/Cerune/tree/f48a72c01f33ff32f483a8c125d0b69b4f7d2444/src/codegen/x86_64). `SOURCE_SNAPSHOT.json` pins their Git blob SHA-1 hashes; `src/lib.rs` supplies standalone target and diagnostic types. Standalone tests cover a small subset; Cerune-dependent integration tests have not all been ported.

Original code: Copyright (c) 2026 Hokutaka; MIT OR Apache-2.0. See the included license files.
