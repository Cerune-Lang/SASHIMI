# SASHIMI 🍣 — extracted assembler kernel (experimental)

A minimal, dependency-free extraction of Cerune's x86-64 assembler and ELF64/COFF object writer.

**Status:** Experimental, unreleased, and untested by a local Rust toolchain. This is the subset of GNU-style assembly currently emitted by Cerune, **not** a general-purpose assembler. CLI, provenance manifests, source-to-byte tracing, dynamic linking and stable APIs are not implemented.

```rust
use sashimi_assembler::{assemble, Target};
let obj = assemble(".text\n.globl main\nmain:\nretq\n", Target::X86_64UnknownLinuxGnu)?;
assert_eq!(&obj[0..4], b"\x7fELF");
# Ok::<(), sashimi_assembler::Diagnostic>(())
```

Run `cargo test --all-targets` and `python3 scripts/verify_snapshot.py`. GitHub Actions runs these automatically on pull requests.

## Extraction and fidelity

`src/object.rs`, `src/object/parse.rs`, `src/object/encode.rs`, and `src/object/format.rs` are byte-for-byte copies of [Cerune's x86-64 object encoder](https://github.com/Cerune-Lang/Cerune/tree/master/src/codegen/x86_64). `SOURCE_SNAPSHOT.json` pins each source file's Git blob SHA-1. `src/lib.rs` supplies standalone target and diagnostic types. Tests are a small standalone subset; Cerune-dependent integration tests have not been ported.

Original code: Copyright (c) 2026 Hokutaka; MIT OR Apache-2.0. See the included license files.
