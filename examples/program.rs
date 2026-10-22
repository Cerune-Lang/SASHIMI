#[path = "support/program.rs"]
mod support;
use sashimi_assembler::{
    Target,
    program::{assemble_program, render_asm},
};
use std::{error::Error, path::PathBuf};
fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let target = args
        .next()
        .and_then(|s| Target::parse(&s))
        .ok_or("provide an explicit target triple and output directory")?;
    let dir = PathBuf::from(args.next().ok_or("provide an output directory")?);
    if args.next().is_some() {
        return Err("unexpected argument".into());
    }
    let program = support::example_program(target);
    let artifact = assemble_program(&program)?;
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("program.o"), &artifact.object)?;
    std::fs::write(dir.join("program.s"), render_asm(&program)?)?;
    std::fs::write(
        dir.join("origins.txt"),
        format!(
            "entries: {:#?}\nsymbols: {:#?}\nreferences: {:#?}\n",
            artifact.entries, artifact.symbols, artifact.references
        ),
    )?;
    println!(
        "wrote Program Object, ASM and section-relative origins to {}",
        dir.display()
    );
    Ok(())
}
