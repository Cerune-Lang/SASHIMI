# Typed Program API

[日本語](typed-program.ja.md)

Stage 2 of the [overall design](typed-assembly.en.md). One `Program` produces assembly, ELF64/COFF objects and provenance mappings. Object generation does not pass through assembly text or a parser. The caller chooses instructions, ABI and runtime behavior.

## Inputs and results

| API/value | Contract |
| --- | --- |
| `Program::new(Target)` | Explicit x86-64 Linux GNU or Windows MSVC target; never inferred from the running OS |
| `declare(name, Linkage)` | Returns a Program-local SymbolId. Local/Global require exactly one definition; External forbids definitions |
| `push(EntryKind, Provenance)` | Returns an input-order EntryId: Section, Label, Instruction, Data, Data32, Data64, Align or Marker |
| `Operand::Symbol` / `RipRelative` | Explicit branch / RIP-relative symbol references; no name-based meaning |
| `assemble_program(&Program)` | Returns object bytes and entry, symbol and reference locations |
| `render_asm(&Program)` | Validates the same input and renders target-specific AT&T assembly |
| `ProgramDiagnostic` | Validate/Encode/Resolve/Object phase, reason, related EntryId, OriginId and SymbolId |

IDs are indices into each input array. They do not establish identity across Programs or array edits. Symbol declarations belong to the symbol table and have no EntryId. External declarations have no position; definitions have a section and offset.

Symbol names follow `[A-Za-z_.][A-Za-z0-9_.]*`, excluding a lone `.`. Duplicate names, invalid IDs, undefined internal symbols, external definitions and branches into data are rejected. Prefixes such as `cerune_` and `.L` have no special meaning.

## Layout and relocations

| Item | Implementation |
| --- | --- |
| Sections | Text, ReadOnlyData, Data; explicitly select one before labels, instructions, data, alignment or markers |
| Data | Preserve raw bytes; no implicit NUL, newline conversion or Unicode normalization. Empty Data sections are retained |
| Data32 / Data64 | Store u32/u64 bit patterns as 4/8 little-endian bytes and render .long/.quad. No value conversion or implicit alignment; floating-point meaning belongs to the caller |
| Align | Power-of-two exponent: 0–16 for ELF, 0–13 for COFF. Fill Text with 0x90 and other sections with zero; also set object section alignment. Minimum object alignment is 16 bytes |
| Branches | call, jmp and 16 jcc conditions, using fixed 32-bit displacements. Resolve Text references directly; no short-branch optimization |
| ELF | PLT32 for external branches; PC32 for RIP references to other sections or external data. Assembly also explicitly uses `@PLT` for external branches |
| COFF | REL32 for unresolved references, including addends for trailing instruction immediates |
| int | Only `int $0x29`, matching the existing encoder |

References to Text definitions within the Program resolve locally. Global means export visibility, not link-time interposition by another definition. Whether external-data RIP references work with PIE also depends on linker/layout; GOT/TLS are unsupported.

The legacy `assemble(&str, Target)` retains diagnostics and output. Its external jmp still uses PC32. PLT32 and PIE execution coverage apply to the new Program route.

## Reading provenance

`Artifact.entries` returns input-order EntryId, Provenance and half-open ranges `[start, end)`. Locations are **section-relative offsets in the generated Object**, not file offsets or linked addresses.

- One OriginId can cover several disjoint ranges; one instruction can carry several OriginIds.
- Synthetic is explicit and may coexist with OriginIds. Empty origin sets require Synthetic; duplicate IDs are rejected.
- Labels, markers, empty data and zero padding retain empty ranges. Section selections have no range.
- `references` records source EntryId, SymbolId, four-byte displacement field, addend and resolved/relocation kind. Addends are measured from the field start and include trailing immediates.
- Assembly comments include EntryId, OriginId and Synthetic. External assemblers may change branch lengths or padding; do not apply our Object mappings to their outputs.

Changing only valid provenance leaves the entire Object identical. Rendering/encoding does not mutate input, and mappings expose no execution-control API. Mapping back to Cerune NodeId, SourceId and Span is the caller's responsibility.

## Execution example and validation

`examples/program.rs` constructs a Program printing the same line three times through a normal call, tail jump and conditional external branch. The example explicitly chooses ABI argument registers and stack allocation.

```sh
cargo run --example program -- x86_64-unknown-linux-gnu target/program-linux
clang -fPIE -pie target/program-linux/program.o -o target/program-linux/from-object
clang -fPIE -pie target/program-linux/program.s -o target/program-linux/from-asm
./target/program-linux/from-object
./target/program-linux/from-asm
```

On Windows, select `x86_64-pc-windows-msvc` and link `program.o` and `program.s` separately with a matching clang/linker. The example also writes `origins.txt`, a debug display rather than a stable interchange format.

`SASHIMI_TEST_CLANG=clang cargo test --all-targets` enables external link/run comparisons. In PowerShell first set `$env:SASHIMI_TEST_CLANG = 'clang'`. Without the variable, the test reports the comparison was not run. CI sets it on both Windows and Linux; tool failures are not skipped.

Unit tests check object equality with the frozen extraction on both targets, all branches, relocation records, trailing immediates, provenance invariance, empty ranges, alignment headers and invalid input. Fixture hashes remain unchanged.

## Subsequent work

Cerune instruction selection and runtime helpers still use the existing assembly-text input. Next migrate them to typed Program construction and compare the existing 135 examples, MIR/SSA, origin annotations, failure diagnostics and I/O ordering.

Arbitrary sections, pointer relocations in data, indirect calls/jumps, GOT/TLS, dynamic linking, linked-address mappings, stable APIs and a CLI remain unsupported. The library never invokes an external linker or executor implicitly.

`CString(String)` appends one NUL to the UTF-8 bytes, preserving embedded NULs without normalization or implicit alignment. ASCII format strings render readably as escaped `.asciz`; embedded NULs, non-ASCII bytes and control characters other than LF use exact `.byte` sequences.
