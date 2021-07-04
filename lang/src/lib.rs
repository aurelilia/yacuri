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

    fn file<T: Debug + PartialEq>(input: &str, expect: T) {
        let res = execute_program::<T>(input).unwrap();
        assert_eq!(res, expect)
    }

    fn expr<T: Debug + PartialEq>(input: &str, ret_type: &str, expect: T) {
        file::<T>(
            &format!("fun main() {} {{ {} \n }}", ret_type, input),
            expect,
        );
    }

    fn expr_none(input: &str) {
        expr(input, "", ())
    }
    fn expr_bool(input: &str, expect: bool) {
        expr(input, "-> bool", expect)
    }
    fn expr_i64(input: &str, expect: i64) {
        expr(input, "-> i64", expect)
    }

    #[test]
    fn block() {
        expr_i64("5 + 5 \n  2 - 2 \n 1", 1);
    }

    #[test]
    fn binary() {
        expr_i64("5 + 37", 42);
        expr_i64("3 - 2", 1);
        expr_i64("5 * 2", 10);
        expr_i64("64 / 8", 8);
    }

    #[test]
    fn logic() {
        expr_bool("5 == 5", true);
        expr_bool("5 != 5", false);
        expr_bool("5 == 7", false);
        expr_bool("5 != 7", true);

        expr_bool("5 <= 5", true);
        expr_bool("5 < 5", false);
        expr_bool("5 >= 5", true);
        expr_bool("5 > 5", false);

        expr_bool("5 <= 7", true);
        expr_bool("5 < 7", true);
        expr_bool("5 >= 7", false);
        expr_bool("5 > 7", false);
    }

    #[test]
    fn if_stmt() {
        expr_none("if (true) 35");
        expr_none("if (false) 35.42 else 0");
    }

    #[test]
    fn if_expr() {
        expr_i64("if (true) 35 else 0", 35);
        expr_i64("if (false) 35 else 0", 0);
    }

    #[test]
    fn while_() {
        expr_i64("var a = 3 \n while (a < 10) { a = a + 1 } \n a", 10);
        expr_i64("var a = 3 \n while (a > 10) { a = a + 1 } \n a", 3);
    }

    #[test]
    fn var_decl() {
        expr_i64("val a = 44 \n a", 44);
        expr_i64("var c = 24 + 1 \n c", 25);
    }

    #[test]
    fn assignment() {
        expr_i64("var a = 44 \n a = 4 \n a", 4);
        expr_i64("var c = 24 + 1 \n c = c + 2 \n c", 27);
    }

    #[test]
    fn basic_funcs() {
        file(include_str!("../tests/basic_funcs.yacari"), 422);
    }
}
