use super::*;
use crate::typed::{Opcode, Register, Width};
fn reg(n: u8) -> Operand {
    Register::gpr(n, Width::Qword).unwrap().into()
}
fn push(p: &mut Program, kind: EntryKind) -> EntryId {
    p.push(kind, Provenance::source(OriginId(7)))
}
fn ins(p: &mut Program, op: Opcode, args: Vec<Operand>) -> EntryId {
    push(p, EntryKind::Instruction(Instruction::new(op, args)))
}
const TARGETS: [Target; 2] = [Target::X86_64UnknownLinuxGnu, Target::X86_64PcWindowsMsvc];
fn number(bytes: &[u8], at: usize, width: usize) -> u64 {
    let mut n = [0u8; 8];
    n[..width].copy_from_slice(&bytes[at..at + width]);
    u64::from_le_bytes(n)
}
fn text_bytes(artifact: &Artifact, target: Target) -> &[u8] {
    let bytes = &artifact.object;
    let (offset, size) = match target {
        Target::X86_64UnknownLinuxGnu => {
            let h = number(bytes, 40, 8) as usize + 64;
            (number(bytes, h + 24, 8), number(bytes, h + 32, 8))
        }
        Target::X86_64PcWindowsMsvc => (number(bytes, 40, 4), number(bytes, 36, 4)),
    };
    &bytes[offset as usize..(offset + size) as usize]
}
#[test]
fn objects_match_frozen_encoder_for_sections_symbols_branches_and_data() {
    for target in TARGETS {
        let mut p = Program::new(target);
        let state = p.declare("state", Linkage::Local);
        let msg = p.declare("msg", Linkage::Local);
        let main = p.declare("main", Linkage::Global);
        let done = p.declare(".Ldone", Linkage::Local);
        let puts = p.declare("puts", Linkage::External);
        push(&mut p, EntryKind::Section(Section::Data));
        push(&mut p, EntryKind::Label(state));
        push(&mut p, EntryKind::Data(vec![1]));
        push(&mut p, EntryKind::Align(3));
        push(&mut p, EntryKind::Section(Section::ReadOnlyData));
        push(&mut p, EntryKind::Label(msg));
        push(&mut p, EntryKind::Data("日本語\0\r\n".as_bytes().to_vec()));
        push(&mut p, EntryKind::Section(Section::Text));
        push(&mut p, EntryKind::Label(main));
        ins(
            &mut p,
            Opcode::Test8,
            vec![Operand::Immediate(0), Operand::RipRelative(state)],
        );
        ins(
            &mut p,
            Opcode::Lea64,
            vec![Operand::RipRelative(msg), reg(1)],
        );
        ins(&mut p, Opcode::JumpE, vec![Operand::Symbol(done)]);
        ins(&mut p, Opcode::Call, vec![Operand::Symbol(puts)]);
        push(&mut p, EntryKind::Label(done));
        ins(&mut p, Opcode::Ret, vec![]);
        let asm = render_asm(&p)
            .unwrap()
            .lines()
            .filter(|line| !line.starts_with(".extern "))
            .collect::<Vec<_>>()
            .join("\n")
            .replace("@PLT", "");
        assert_eq!(
            assemble_program(&p).unwrap().object,
            crate::extracted::assemble(&asm, target).unwrap()
        );
    }
}
#[test]
fn origins_markers_padding_and_observation_do_not_change_objects() {
    for target in TARGETS {
        let mut p = Program::new(target);
        let ext = p.declare("external_unused", Linkage::External);
        push(&mut p, EntryKind::Section(Section::Text));
        ins(&mut p, Opcode::Mov64, vec![reg(0), reg(1)]);
        p.push(EntryKind::Align(3), Provenance::synthetic());
        p.push(
            EntryKind::Instruction(Instruction::new(Opcode::Undefined, vec![])),
            Provenance {
                origins: vec![OriginId(7), OriginId(8)],
                synthetic: false,
            },
        );
        push(&mut p, EntryKind::Marker);
        push(&mut p, EntryKind::Data(vec![]));
        push(&mut p, EntryKind::Align(0));
        let saved = p.clone();
        let a = assemble_program(&p).unwrap();
        assert_eq!(a.symbols[ext.0].position, None);
        assert_eq!(a.entries[0].range, None);
        let ranges = a
            .entries
            .iter()
            .skip(1)
            .map(|e| {
                let r = e.range.unwrap();
                (r.start, r.end)
            })
            .collect::<Vec<_>>();
        assert_eq!(
            ranges,
            vec![(0, 3), (3, 8), (8, 10), (10, 10), (10, 10), (10, 10)]
        );
        assert_eq!(&text_bytes(&a, target)[3..8], &[0x90; 5]);
        assert_eq!(
            a.entries[3].provenance.origins,
            vec![OriginId(7), OriginId(8)]
        );
        for _ in 0..3 {
            render_asm(&p).unwrap();
            assert_eq!(assemble_program(&p).unwrap(), a);
            assert_eq!(p, saved);
        }
        for entry in &mut p.entries {
            entry.provenance = Provenance::synthetic();
        }
        assert_eq!(assemble_program(&p).unwrap().object, a.object);
    }
}
#[test]
fn every_branch_resolves_in_both_directions_and_external_branches_relocate() {
    for target in TARGETS {
        for &opcode in Opcode::ALL
            .iter()
            .filter(|op| op.mnemonic().starts_with('j') || **op == Opcode::Call)
        {
            for external in [false, true] {
                let mut p = Program::new(target);
                let start = p.declare("start", Linkage::Local);
                let end = p.declare(
                    "cerune_external_or_internal",
                    if external {
                        Linkage::External
                    } else {
                        Linkage::Local
                    },
                );
                push(&mut p, EntryKind::Section(Section::Text));
                push(&mut p, EntryKind::Label(start));
                ins(&mut p, opcode, vec![Operand::Symbol(end)]);
                ins(&mut p, opcode, vec![Operand::Symbol(start)]);
                if !external {
                    push(&mut p, EntryKind::Label(end));
                }
                let a = assemble_program(&p).unwrap();
                let r = &a.references[0];
                assert_eq!(
                    r.kind,
                    if !external {
                        ReferenceKind::ResolvedRelative
                    } else if target == TARGETS[0] {
                        ReferenceKind::ElfPlt32
                    } else {
                        ReferenceKind::CoffRel32
                    }
                );
                let r = &a.references[1];
                assert_eq!(r.kind, ReferenceKind::ResolvedRelative);
                let bytes = text_bytes(&a, target);
                let signed = number(bytes, r.field.start as usize, 4) as u32 as i32;
                assert_eq!(signed as i64, r.addend - r.field.start as i64);
                if external {
                    assert!(
                        crate::typed::encode_instruction(&Instruction::new(
                            opcode,
                            vec![Operand::Symbol(end)]
                        ))
                        .is_err()
                    );
                }
            }
        }
    }
}
#[test]
fn relocation_fields_and_trailing_immediate_addends_match_file_records() {
    for target in TARGETS {
        let mut p = Program::new(target);
        let data = p.declare("state", Linkage::Global);
        let function = p.declare("puts", Linkage::External);
        push(&mut p, EntryKind::Section(Section::Text));
        ins(
            &mut p,
            Opcode::Test8,
            vec![Operand::Immediate(7), Operand::RipRelative(data)],
        );
        ins(&mut p, Opcode::Jump, vec![Operand::Symbol(function)]);
        push(&mut p, EntryKind::Section(Section::Data));
        push(&mut p, EntryKind::Label(data));
        push(&mut p, EntryKind::Data(vec![1]));
        let a = assemble_program(&p).unwrap();
        assert_eq!(a.references[0].addend, -5);
        assert_eq!(a.references[1].addend, -4);
        let b = &a.object;
        if target == TARGETS[0] {
            let shoff = number(b, 40, 8) as usize;
            let reloc = number(b, shoff + 3 * 64 + 24, 8) as usize;
            for (i, r) in a.references.iter().enumerate() {
                assert_eq!(number(b, reloc + i * 24, 8), r.field.start);
                assert_eq!(number(b, reloc + i * 24 + 8, 4), if i == 0 { 2 } else { 4 });
                assert_eq!(number(b, reloc + i * 24 + 16, 8) as i64, r.addend);
            }
        } else {
            let reloc = number(b, 44, 4) as usize;
            let text = text_bytes(&a, target);
            for (i, r) in a.references.iter().enumerate() {
                assert_eq!(number(b, reloc + i * 10, 4), r.field.start);
                assert_eq!(number(b, reloc + i * 10 + 8, 2), 4);
                assert_eq!(
                    number(text, r.field.start as usize, 4) as u32 as i32 as i64,
                    r.addend + 4
                );
            }
        }
    }
}
#[test]
fn alignments_and_empty_data_definitions_have_valid_section_headers() {
    for target in TARGETS {
        let mut p = Program::new(target);
        let empty = p.declare("empty", Linkage::Global);
        push(&mut p, EntryKind::Section(Section::Data));
        push(&mut p, EntryKind::Align(8));
        push(&mut p, EntryKind::Label(empty));
        let a = assemble_program(&p).unwrap();
        assert_eq!(
            a.symbols[0].position,
            Some(Position {
                section: Section::Data,
                offset: 0
            })
        );
        let b = &a.object;
        if target == TARGETS[0] {
            assert_eq!(number(b, 60, 2), 9);
            let h = number(b, 40, 8) as usize + 8 * 64;
            assert_eq!(number(b, h + 48, 8), 256);
            assert_eq!(number(b, h + 24, 8) % 256, 0);
            assert_eq!(number(b, h + 32, 8), 0);
        } else {
            assert_eq!(number(b, 2, 2), 3);
            assert_eq!((number(b, 20 + 2 * 40 + 36, 4) >> 20) & 15, 9);
        }
        push(
            &mut p,
            EntryKind::Align(if target == TARGETS[0] { 17 } else { 14 }),
        );
        assert_eq!(assemble_program(&p).unwrap_err().phase, Phase::Validate);
    }
}
#[test]
fn invalid_inputs_report_entry_origin_symbol_and_phase_without_parsing_text() {
    let mut p = Program::new(TARGETS[0]);
    let missing = p.declare("missing", Linkage::Local);
    push(&mut p, EntryKind::Section(Section::Text));
    let call = ins(&mut p, Opcode::Call, vec![Operand::Symbol(missing)]);
    let e = assemble_program(&p).unwrap_err();
    assert_eq!(
        (e.phase, e.entry, e.symbol),
        (Phase::Resolve, Some(call), Some(missing))
    );
    assert_eq!(e.origins, vec![OriginId(7)]);
    p.symbols[0].linkage = Linkage::External;
    assemble_program(&p).unwrap();
    push(&mut p, EntryKind::Label(missing));
    assert_eq!(assemble_program(&p).unwrap_err().phase, Phase::Validate);
    p.entries.pop();
    p.entries[call.0].kind = EntryKind::Instruction(Instruction::new(
        Opcode::Call,
        vec![Operand::Symbol(SymbolId(99))],
    ));
    assert_eq!(assemble_program(&p).unwrap_err().symbol, Some(SymbolId(99)));
    p.entries[call.0].kind = EntryKind::Instruction(Instruction::new(Opcode::Mov64, vec![reg(0)]));
    assert_eq!(assemble_program(&p).unwrap_err().phase, Phase::Encode);
    p.entries[call.0].kind = EntryKind::Marker;
    p.entries[call.0].provenance.origins.clear();
    assert_eq!(assemble_program(&p).unwrap_err().entry, Some(call));
    p.entries[call.0].provenance.origins = vec![OriginId(1), OriginId(1)];
    assert!(assemble_program(&p).is_err());
    for name in ["", ".", "9abc", "x\n.text", "a\0b", "a; ret", "$bad"] {
        let mut p = Program::new(TARGETS[0]);
        p.declare(name, Linkage::External);
        assert!(assemble_program(&p).is_err(), "{name:?}");
        assert!(render_asm(&p).is_err());
    }
    let mut p = Program::new(TARGETS[0]);
    p.declare("same", Linkage::External);
    p.declare("same", Linkage::Local);
    assert!(assemble_program(&p).is_err());
    let mut p = Program::new(TARGETS[0]);
    push(&mut p, EntryKind::Marker);
    assert!(assemble_program(&p).is_err());
    let mut p = Program::new(TARGETS[0]);
    let data = p.declare("data", Linkage::Local);
    push(&mut p, EntryKind::Section(Section::Data));
    push(&mut p, EntryKind::Label(data));
    push(&mut p, EntryKind::Label(data));
    assert!(assemble_program(&p).is_err());
    p.entries.pop();
    ins(&mut p, Opcode::Ret, vec![]);
    assert!(assemble_program(&p).is_err());
    p.entries.pop();
    push(&mut p, EntryKind::Section(Section::Text));
    ins(&mut p, Opcode::Jump, vec![Operand::Symbol(data)]);
    assert_eq!(assemble_program(&p).unwrap_err().phase, Phase::Resolve);
}

#[test]
fn symbol_declaration_order_does_not_change_object_order() {
    for target in TARGETS {
        let mut p = Program::new(target);
        let function = p.declare("entry", Linkage::Global);
        let data = p.declare("data", Linkage::Local);
        let external = p.declare("external", Linkage::External);
        push(&mut p, EntryKind::Section(Section::Text));
        push(&mut p, EntryKind::Label(function));
        ins(
            &mut p,
            Opcode::Lea64,
            vec![Operand::RipRelative(data), reg(0)],
        );
        ins(&mut p, Opcode::Call, vec![Operand::Symbol(external)]);
        push(&mut p, EntryKind::Section(Section::ReadOnlyData));
        push(&mut p, EntryKind::Label(data));
        push(&mut p, EntryKind::Data(vec![0]));
        let expected = assemble_program(&p).unwrap().object;
        p.symbols.reverse();
        let remap = |id: &mut SymbolId| id.0 = 2 - id.0;
        for entry in &mut p.entries {
            match &mut entry.kind {
                EntryKind::Label(id) => remap(id),
                EntryKind::Instruction(i) => {
                    for op in &mut i.operands {
                        if let Operand::Symbol(id) | Operand::RipRelative(id) = op {
                            remap(id);
                        }
                    }
                }
                _ => {}
            }
        }
        assert_eq!(assemble_program(&p).unwrap().object, expected);
    }
}
