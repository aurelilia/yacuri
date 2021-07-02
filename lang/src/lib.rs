#![feature(box_patterns)]
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
    use core::fmt::Debug;
    use std::format;

    fn expr<T: Debug + PartialEq>(input: &str, ret_type: &str, expect: T) {
        let res =
            execute_program::<T>(&format!("fun main() {} {{ {} \n }}", ret_type, input)).unwrap();
        assert_eq!(res, expect)
    }

    fn nothing(input: &str) {
        expr(input, "", ())
    }
    fn bool_(input: &str, expect: bool) {
        expr(input, "-> bool", expect)
    }
    fn i64_(input: &str, expect: i64) {
        expr(input, "-> i64", expect)
    }

    #[test]
    fn block() {
        i64_("5 + 5 \n  2 - 2 \n 1", 1);
    }

    #[test]
    fn binary() {
        i64_("5 + 37", 42);
        i64_("3 - 2", 1);
        i64_("5 * 2", 10);
        i64_("64 / 8", 8);
    }

    #[test]
    fn logic() {
        bool_("5 == 5", true);
        bool_("5 != 5", false);
        bool_("5 == 7", false);
        bool_("5 != 7", true);

        bool_("5 <= 5", true);
        bool_("5 < 5", false);
        bool_("5 >= 5", true);
        bool_("5 > 5", false);

        bool_("5 <= 7", true);
        bool_("5 < 7", true);
        bool_("5 >= 7", false);
        bool_("5 > 7", false);
    }

    #[test]
    fn if_stmt() {
        nothing("if (true) 35");
        nothing("if (false) 35.42 else 0");
    }

    #[test]
    fn if_expr() {
        i64_("if (true) 35 else 0", 35);
        i64_("if (false) 35 else 0", 0);
    }

    #[test]
    fn while_() {
        i64_("var a = 3 \n while (a < 10) { a = a + 1 } \n a", 10);
        i64_("var a = 3 \n while (a > 10) { a = a + 1 } \n a", 3);
    }

    #[test]
    fn var_decl() {
        i64_("val a = 44 \n a", 44);
        i64_("var c = 24 + 1 \n c", 25);
    }

    #[test]
    fn assignment() {
        i64_("var a = 44 \n a = 4 \n a", 4);
        i64_("var c = 24 + 1 \n c = c + 2 \n c", 27);
    }
}
