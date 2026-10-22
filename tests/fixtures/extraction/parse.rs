use super::{Fixup, Object, Symbol, encode};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn assemble(assembly: &str) -> Result<Object, String> {
    let mut sections = [Vec::new(), Vec::new(), Vec::new()];
    let mut current = 1;
    let mut symbols = BTreeMap::new();
    let mut globals = BTreeSet::new();
    let mut fixups = Vec::new();
    for (number, line) in assembly.lines().enumerate() {
        let line = line.split('#').next().unwrap().trim();
        if line.is_empty() {
            continue;
        }
        let result: Result<(), String> = (|| {
            if let Some(name) = line.strip_suffix(':') {
                if symbols
                    .insert(name.to_owned(), (current, sections[current].len()))
                    .is_some()
                {
                    return Err(format!("duplicate label {name}"));
                }
                return Ok(());
            }
            if line == ".data" {
                current = 2;
                return Ok(());
            }
            if line == ".text" {
                current = 0;
                return Ok(());
            }
            if line.starts_with(".section ") {
                match line.split_whitespace().nth(1).unwrap() {
                    ".rodata" | ".rdata,\"dr\"" => current = 1,
                    ".note.GNU-stack,\"\",@progbits" => {}
                    _ => return Err(format!("unsupported section {line}")),
                }
                return Ok(());
            }
            if let Some(name) = line.strip_prefix(".globl ") {
                globals.insert(name.to_owned());
                return Ok(());
            }
            if let Some(value) = line.strip_prefix(".p2align ") {
                let exponent = value.parse::<u32>().map_err(|_| "invalid alignment")?;
                if exponent > 16 {
                    return Err("alignment exceeds encoder limit".into());
                }
                let alignment = 1usize << exponent;
                let section = &mut sections[current];
                section.resize(
                    section.len().next_multiple_of(alignment),
                    if current == 0 { 0x90 } else { 0 },
                );
                return Ok(());
            }
            if let Some(value) = line.strip_prefix(".asciz ") {
                let value = value
                    .strip_prefix('"')
                    .and_then(|v| v.strip_suffix('"'))
                    .ok_or("invalid string data")?;
                let mut bytes = value.bytes();
                while let Some(byte) = bytes.next() {
                    sections[current].push(if byte == b'\\' {
                        match bytes.next() {
                            Some(b'n') => 10,
                            Some(b'0') => 0,
                            Some(b'\\') => b'\\',
                            Some(b'"') => b'"',
                            _ => return Err("unsupported data escape".into()),
                        }
                    } else {
                        byte
                    });
                }
                sections[current].push(0);
                return Ok(());
            }
            for (directive, width) in [(".byte ", 1), (".long ", 4), (".quad ", 8)] {
                if let Some(value) = line.strip_prefix(directive) {
                    let value = encode::number(value)?;
                    if value < 0 || value >= (1i128 << (width * 8)) {
                        return Err("data value out of range".into());
                    }
                    sections[current].extend_from_slice(&value.to_le_bytes()[..width]);
                    return Ok(());
                }
            }
            if current != 0 {
                return Err("instruction outside .text".into());
            }
            let (bytes, fixup) = encode::instruction(line)?;
            let start = sections[0].len();
            if let Some(mut fixup) = fixup {
                fixup.offset += start;
                fixups.push(fixup);
            }
            sections[0].extend(bytes);
            Ok(())
        })();
        result.map_err(|e| format!("assembly line {}: {e} ({line})", number + 1))?;
    }
    let mut relocations = Vec::<Fixup>::new();
    for fixup in fixups {
        if let Some(&(section, offset)) = symbols.get(&fixup.symbol) {
            if section == 0 {
                let displacement =
                    i32::try_from(offset as i64 + fixup.addend - fixup.offset as i64)
                        .map_err(|_| "relative displacement exceeds 32 bits")?;
                sections[0][fixup.offset..fixup.offset + 4]
                    .copy_from_slice(&displacement.to_le_bytes());
                continue;
            }
        } else if fixup.symbol.starts_with(".L") || fixup.symbol.starts_with("cerune_") {
            return Err(format!("unresolved internal label {}", fixup.symbol));
        }
        relocations.push(fixup);
    }
    let mut all: Vec<_> = symbols
        .into_iter()
        .map(|(name, (section, offset))| Symbol {
            global: globals.contains(&name),
            name,
            section: Some(section),
            offset,
        })
        .collect();
    for relocation in &relocations {
        if !all.iter().any(|s| s.name == relocation.symbol) {
            all.push(Symbol {
                name: relocation.symbol.clone(),
                section: None,
                offset: 0,
                global: true,
            });
        }
    }
    all.sort_by(|a, b| (a.global, &a.name).cmp(&(b.global, &b.name)));
    Ok(Object {
        sections,
        symbols: all,
        relocations,
    })
}
