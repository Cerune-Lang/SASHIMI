use super::{Object, Target};

fn put(bytes: &mut [u8], offset: usize, value: u64, width: usize) {
    bytes[offset..offset + width].copy_from_slice(&value.to_le_bytes()[..width]);
}
fn append(bytes: &mut Vec<u8>, value: u64, width: usize) {
    bytes.extend_from_slice(&value.to_le_bytes()[..width]);
}
fn string(table: &mut Vec<u8>, name: &str) -> u64 {
    let offset = table.len() as u64;
    table.extend(name.bytes());
    table.push(0);
    offset
}

pub(super) fn write(object: &Object, target: Target) -> Result<Vec<u8>, String> {
    // COFFの32ビット位置と再配置数を暗黙に切り捨てません。
    if object.sections.iter().any(|s| s.len() > i32::MAX as usize)
        || object.symbols.len() > 1_000_000
    {
        return Err("native object exceeds encoder size limit".into());
    }
    match target {
        Target::X86_64UnknownLinuxGnu => Ok(elf(object)),
        Target::X86_64PcWindowsMsvc => coff(object),
    }
}

fn elf(object: &Object) -> Vec<u8> {
    let mut names = vec![0];
    let mut symbols = vec![0; 24];
    for symbol in &object.symbols {
        append(&mut symbols, string(&mut names, &symbol.name), 4);
        symbols.push(if symbol.global { 0x10 } else { 0 });
        symbols.push(0);
        append(
            &mut symbols,
            symbol.section.map_or(0, |s| if s == 2 { 8 } else { s + 1 }) as u64,
            2,
        );
        append(&mut symbols, symbol.offset as u64, 8);
        append(&mut symbols, 0, 8);
    }
    let mut relocations = Vec::new();
    for relocation in &object.relocations {
        let index = object
            .symbols
            .iter()
            .position(|s| s.name == relocation.symbol)
            .unwrap()
            + 1;
        append(&mut relocations, relocation.offset as u64, 8);
        // 外部呼び出しはPLT32、データへのRIP相対参照はPC32です。
        append(
            &mut relocations,
            ((index as u64) << 32) | if relocation.call { 4 } else { 2 },
            8,
        );
        append(&mut relocations, relocation.addend as u64, 8);
    }
    let mut section_names = vec![0];
    let mut section_offsets: Vec<_> = [
        ".text",
        ".rodata",
        ".rela.text",
        ".symtab",
        ".strtab",
        ".shstrtab",
        ".note.GNU-stack",
    ]
    .iter()
    .map(|s| string(&mut section_names, s))
    .collect();
    let has_data = !object.sections[2].is_empty();
    if has_data {
        section_offsets.push(string(&mut section_names, ".data"));
    }
    let mut data = vec![
        object.sections[0].as_slice(),
        object.sections[1].as_slice(),
        relocations.as_slice(),
        symbols.as_slice(),
        names.as_slice(),
        section_names.as_slice(),
        &[],
    ];
    let mut alignments = vec![16, 16, 8, 8, 1, 1, 1];
    if has_data {
        data.push(&object.sections[2]);
        alignments.push(16);
    }
    let section_count = data.len() + 1;
    let mut bytes = vec![0; 64];
    let mut offsets = Vec::new();
    for (section, alignment) in data.iter().zip(alignments.iter().copied()) {
        bytes.resize(bytes.len().next_multiple_of(alignment), 0);
        offsets.push(bytes.len());
        bytes.extend_from_slice(section);
    }
    bytes.resize(bytes.len().next_multiple_of(8), 0);
    let shoff = bytes.len();
    bytes.resize(shoff + section_count * 64, 0);
    bytes[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
    put(&mut bytes, 16, 1, 2);
    put(&mut bytes, 18, 62, 2);
    put(&mut bytes, 20, 1, 4);
    put(&mut bytes, 40, shoff as u64, 8);
    put(&mut bytes, 52, 64, 2);
    put(&mut bytes, 58, 64, 2);
    put(&mut bytes, 60, section_count as u64, 2);
    put(&mut bytes, 62, 6, 2);
    let first_global = object
        .symbols
        .iter()
        .position(|s| s.global)
        .unwrap_or(object.symbols.len())
        + 1;
    for index in 0..data.len() {
        let header = shoff + (index + 1) * 64;
        put(&mut bytes, header, section_offsets[index], 4);
        put(
            &mut bytes,
            header + 4,
            match index {
                2 => 4,
                3 => 2,
                4 | 5 => 3,
                _ => 1,
            },
            4,
        );
        put(
            &mut bytes,
            header + 8,
            match index {
                0 => 6,
                1 => 2,
                7 => 3,
                _ => 0,
            },
            8,
        );
        put(&mut bytes, header + 24, offsets[index] as u64, 8);
        put(&mut bytes, header + 32, data[index].len() as u64, 8);
        put(
            &mut bytes,
            header + 40,
            match index {
                2 => 4,
                3 => 5,
                _ => 0,
            },
            4,
        );
        put(
            &mut bytes,
            header + 44,
            match index {
                2 => 1,
                3 => first_global as u64,
                _ => 0,
            },
            4,
        );
        put(&mut bytes, header + 48, alignments[index] as u64, 8);
        put(
            &mut bytes,
            header + 56,
            if index == 2 || index == 3 { 24 } else { 0 },
            8,
        );
    }
    bytes
}

fn coff(object: &Object) -> Result<Vec<u8>, String> {
    if object.relocations.len() > u16::MAX as usize {
        return Err("COFF relocation count exceeds 65535".into());
    }
    let section_count = if object.sections[2].is_empty() { 2 } else { 3 };
    let length = 24u64
        + section_count as u64 * 40
        + object
            .sections
            .iter()
            .map(|section| section.len() as u64)
            .sum::<u64>()
        + object.relocations.len() as u64 * 10
        + object.symbols.len() as u64 * 18
        + object
            .symbols
            .iter()
            .filter(|symbol| symbol.name.len() > 8)
            .map(|symbol| symbol.name.len() as u64 + 1)
            .sum::<u64>();
    if length > u32::MAX as u64 {
        return Err("COFF object positions exceed 32 bits".into());
    }
    let mut bytes = vec![0; 20 + section_count * 40];
    let mut offsets = Vec::new();
    for section in &object.sections[..section_count] {
        offsets.push(bytes.len());
        bytes.extend_from_slice(section);
    }
    let relocations_offset = bytes.len();
    for relocation in &object.relocations {
        let index = object
            .symbols
            .iter()
            .position(|s| s.name == relocation.symbol)
            .unwrap();
        // COFF REL32の暗黙addendは変位フィールド末尾から数えます。
        let addend =
            i32::try_from(relocation.addend + 4).map_err(|_| "COFF addend exceeds 32 bits")?;
        put(
            &mut bytes,
            offsets[0] + relocation.offset,
            addend as u32 as u64,
            4,
        );
        append(&mut bytes, relocation.offset as u64, 4);
        append(&mut bytes, index as u64, 4);
        append(&mut bytes, 4, 2);
    }
    let symbol_offset = bytes.len();
    let mut names = vec![0; 4];
    for symbol in &object.symbols {
        if symbol.name.len() <= 8 {
            let start = bytes.len();
            bytes.extend(symbol.name.bytes());
            bytes.resize(start + 8, 0);
        } else {
            append(&mut bytes, 0, 4);
            append(&mut bytes, string(&mut names, &symbol.name), 4);
        }
        append(&mut bytes, symbol.offset as u64, 4);
        append(&mut bytes, symbol.section.map_or(0, |s| s + 1) as u64, 2);
        append(&mut bytes, 0, 2);
        bytes.push(if symbol.global { 2 } else { 3 });
        bytes.push(0);
    }
    let length = names.len() as u64;
    put(&mut names, 0, length, 4);
    bytes.extend(names);
    put(&mut bytes, 0, 0x8664, 2);
    put(&mut bytes, 2, section_count as u64, 2);
    // TimeDateStampは0固定。入力とターゲットだけで成果物を再現できます。
    put(&mut bytes, 8, symbol_offset as u64, 4);
    put(&mut bytes, 12, object.symbols.len() as u64, 4);
    for (index, offset) in offsets.iter().enumerate() {
        let start = 20 + index * 40;
        let name = match index {
            0 => b".text".as_slice(),
            1 => b".rdata".as_slice(),
            _ => b".data".as_slice(),
        };
        bytes[start..start + name.len()].copy_from_slice(name);
        put(
            &mut bytes,
            start + 16,
            object.sections[index].len() as u64,
            4,
        );
        put(&mut bytes, start + 20, *offset as u64, 4);
        if index == 0 && !object.relocations.is_empty() {
            put(&mut bytes, start + 24, relocations_offset as u64, 4);
            put(&mut bytes, start + 32, object.relocations.len() as u64, 2);
        }
        put(
            &mut bytes,
            start + 36,
            match index {
                0 => 0x60500020,
                1 => 0x40500040,
                _ => 0xc0500040,
            },
            4,
        );
    }
    Ok(bytes)
}
