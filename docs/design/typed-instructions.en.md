# Typed single-instruction API

[日本語](typed-instructions.ja.md)

Stage 1 of the [overall design](typed-assembly.en.md). This directly encodes one instruction; it does not produce objects from a typed Program.

## Example

Run `cargo run --example typed` to display assembly and machine bytes constructed from Rust values.

```rust
use sashimi_assembler::typed::{
    Instruction, Opcode, Operand, Register, Width,
    encode_instruction, render_instruction,
};
let rax = Register::gpr(0, Width::Qword)?;
let instruction = Instruction::new(
    Opcode::MovAbs64, vec![Operand::Immediate(42), rax.into()],
);
assert_eq!(render_instruction(&instruction)?, "movabsq $42, %rax");
assert_eq!(encode_instruction(&instruction)?,
    vec![0x48, 0xb8, 42, 0, 0, 0, 0, 0, 0, 0]);
# Ok::<(), sashimi_assembler::Diagnostic>(())
```

Operands follow AT&T source-before-destination order. GPR numbers 0–7 represent RAX/RCX/RDX/RBX/RSP/RBP/RSI/RDI; 8–15 represent R8–R15. Width selects 8/32/64 bits. Use `Register::xmm` for vector registers. High-byte registers such as AH are unsupported.

`Memory::new(base, index, displacement)` accepts a 64-bit GPR base, optional (index, Scale), and i32 displacement. Scale is 1/2/4/8. RSP as index and 32-bit/XMM addressing registers are rejected. R12 is a valid index.

## Coverage

| Implemented | Details |
| --- | --- |
| Types | Opcode, Instruction, Operand, Register, Width, Memory, Scale |
| 81 Opcodes | Moves, integer arithmetic, comparisons, shifts, setcc, SSE, conversions, push/pop, ret/cqto/ud2. [Exact enumeration](../../src/typed.rs) |
| Direct encoding | Registers, immediates, base/index memory; no assembly text or parser |
| Assembly rendering | Validate before rendering, without mutating input |
| Diagnostics | Reject invalid register numbers, widths, forms and immediates using the existing Diagnostic |

Typed calls/jumps/conditional branches, RIP-relative symbols, int, Program, sections, labels, data, objects and OriginId are not implemented. Existing `assemble(&str, Target)` retains its coverage. Cerune production is not switched to this partial API.

An isolated x86-64 instruction has no OS ABI or object format, so this API has no Target argument. Future Program/object operations will require an explicit Target. There is no API to execute the returned bytes.

## Shared implementation and validation

Pass the fixed mnemonic selected by Opcode and typed operands directly to existing validation/encoding functions. There is no arbitrary mnemonic-string input. The legacy text entry point parses first, then uses the same functions. Rendering validates as well, so malformed instructions are not successfully displayed.

- Every Opcode has a valid case compared with the extraction in both object formats.
- 7,452 instruction/operand combinations compare acceptance, rejection and encoding.
- Twelve independent byte expectations, REX/SIB and immediate/displacement boundaries, invalid widths and every GPR number are tested.
- Repeated rendering/encoding preserves input and output; existing labels, relocations and diagnostics remain unchanged.

Agreement with the extraction cannot rule out shared bugs. The twelve byte expectations were also checked with LLVM's assembler. The external-jump PIE limitation remains as documented in the [overall design](typed-assembly.en.md).

Original sources live in [fixed fixtures](../../tests/fixtures/extraction/README.en.md). SOURCE_SNAPSHOT.json hashes are unchanged; verification now targets the fixtures.
