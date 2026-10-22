//! Experimental extracted x86-64 assembler kernel, not general GNU-as compatibility.
use std::fmt;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Target {
    X86_64UnknownLinuxGnu,
    X86_64PcWindowsMsvc,
}
impl Target {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "x86_64-unknown-linux-gnu" => Some(Self::X86_64UnknownLinuxGnu),
            "x86_64-pc-windows-msvc" => Some(Self::X86_64PcWindowsMsvc),
            _ => None,
        }
    }
}
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Diagnostic {
    pub message: String,
}
impl Diagnostic {
    pub fn without_span(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}
impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for Diagnostic {}
mod diagnostic {
    pub use crate::Diagnostic;
}
mod object;
pub fn assemble(source: &str, target: Target) -> Result<Vec<u8>, Diagnostic> {
    object::assemble(source, target)
}

pub mod typed;

#[cfg(test)]
#[rustfmt::skip]
#[path = "../tests/fixtures/extraction/mod.rs"]
mod extracted;
