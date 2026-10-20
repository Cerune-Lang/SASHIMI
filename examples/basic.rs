use sashimi_assembler::{Target, assemble};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let asm = ".text\n.globl main\nmain:\nretq\n";
    let obj = assemble(asm, Target::X86_64UnknownLinuxGnu)?;
    std::fs::write("example.o", obj)?;
    println!("Created example.o");
    Ok(())
}
