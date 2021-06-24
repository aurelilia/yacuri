#![no_std]

extern crate alloc;

use crate::{compiler::Compiler, error::Errors, parser::Parser, vm::JIT};

#[cfg(feature = "core")]
pub use cranelift_jit::{set_manager, MemoryManager};

#[cfg(feature = "std")]
extern crate std;

mod compiler;
mod error;
mod lexer;
mod parser;
mod smol_str;
mod vm;

pub fn execute_program<T>(program: &str) -> Result<T, Errors> {
    let parse = Parser::new(program).parse()?;
    let ir = Compiler::new(parse).consume()?;
    let mut jit = JIT::default();
    let main = jit.compile_mod(&ir);
    Ok(jit.exec(main))
}

#[cfg(test)]
mod test {
    use crate::execute_program;

    extern crate std;

    #[test]
    fn basic() {
        let code = "fun main() -> i64 { 5 + 37 }";
        assert_eq!(execute_program::<usize>(code).unwrap(), 42);
    }
}
