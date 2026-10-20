//! Ceruneが生成する限定されたASMを命令バイトへ符号化します。
//! ソースの意味解析は繰り返さず、外部アセンブラへフォールバックしません。
mod encode;
mod format;
mod parse;
#[cfg(test)]
mod tests;

use super::Target;
use crate::diagnostic::Diagnostic;

#[derive(Debug)]
struct Symbol {
    name: String,
    section: Option<usize>,
    offset: usize,
    global: bool,
}

#[derive(Debug, PartialEq, Eq)]
struct Fixup {
    offset: usize,
    symbol: String,
    addend: i64,
    call: bool,
}

struct Object {
    sections: [Vec<u8>; 3],
    symbols: Vec<Symbol>,
    relocations: Vec<Fixup>,
}

pub(super) fn assemble(assembly: &str, target: Target) -> Result<Vec<u8>, Diagnostic> {
    let object = parse::assemble(assembly).map_err(|message| {
        Diagnostic::without_span(format!("native object encoding failed: {message}"))
    })?;
    format::write(&object, target).map_err(Diagnostic::without_span)
}
