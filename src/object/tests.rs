use super::{Target, encode, parse};
#[test]
fn fixed_instruction_encoding() {
    assert_eq!(encode::instruction("retq").unwrap().0, [0xc3]);
    assert_eq!(encode::instruction("ud2").unwrap().0, [0x0f, 0x0b]);
    assert_eq!(
        encode::instruction("callq printf").unwrap().0,
        [0xe8, 0, 0, 0, 0]
    );
}
#[test]
fn external_call_relocation_is_retained() {
    let object = parse::assemble(".text\ncallq printf\nretq\n").unwrap();
    assert_eq!(object.relocations.len(), 1);
    assert_eq!(object.relocations[0].symbol, "printf");
    assert_eq!(object.relocations[0].addend, -4);
    assert!(object.relocations[0].call);
}
#[test]
fn object_file_magic_and_invalid_input() {
    let source = ".text\n.globl main\nmain:\nretq\n";
    let elf = super::assemble(source, Target::X86_64UnknownLinuxGnu).unwrap();
    assert_eq!(&elf[0..4], b"\x7fELF");
    let coff = super::assemble(source, Target::X86_64PcWindowsMsvc).unwrap();
    assert_eq!(&coff[0..2], &[0x64, 0x86]);
    assert!(
        super::assemble(
            ".text\nthis_is_not_an_instruction\n",
            Target::X86_64UnknownLinuxGnu
        )
        .is_err()
    );
}
