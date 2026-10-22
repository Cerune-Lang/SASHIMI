use sashimi_assembler::typed::{
    Instruction, Opcode, Operand, Register, Width, encode_instruction, render_instruction,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rax = Register::gpr(0, Width::Qword)?;
    let program = [
        Instruction::new(Opcode::MovAbs64, vec![Operand::Immediate(42), rax.into()]),
        Instruction::new(Opcode::Ret, vec![]),
    ];
    for instruction in &program {
        println!(
            "{} => {:02x?}",
            render_instruction(instruction)?,
            encode_instruction(instruction)?
        );
    }
    Ok(())
}
