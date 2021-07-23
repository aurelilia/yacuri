#![feature(box_patterns)]
#![no_std]

extern crate alloc;

use crate::{compiler::Compiler, error::Errors, parser::Parser, vm::JIT};

use crate::{compiler::module::ModuleCompiler, filesystem::Filesystem};
use alloc::{vec, vec::Vec};

pub use crate::vm::SymbolTable;
#[cfg(feature = "core")]
pub use cranelift_jit::{set_manager, MemoryManager};
pub use smol_str::SmolStr;

#[cfg(feature = "std")]
extern crate std;

mod compiler;
mod error;
pub mod filesystem;
mod lexer;
mod parser;
mod smol_str;
mod vm;

pub fn execute_module<T>(program: &str, symbols: SymbolTable) -> Result<T, Errors> {
    let parse = Parser::new(program).parse(vec![SmolStr::new_inline("script")])?;
    let ir = ModuleCompiler::new(parse).consume()?;
    let mut jit = JIT::new(symbols);
    jit.jit_module(&ir);
    Ok(jit.exec("main"))
}

#[cfg(feature = "std")]
pub fn execute_with_os_fs<T>(paths: &[&str], symbols: SymbolTable) -> Result<T, Vec<Errors>> {
    execute_path(filesystem::os_fs::OsFs, paths, symbols)
}

pub fn execute_path<FS: Filesystem, T>(
    fs: FS,
    paths: &[&str],
    symbols: SymbolTable,
) -> Result<T, Vec<Errors>> {
    let mut modules = Vec::with_capacity(20);
    let mut errors = Vec::new();

    for path in paths {
        fs.walk_directory(path, |file| {
            let parse = Parser::new(&file.contents).parse(file.path);
            match parse {
                Ok(module) => modules.push(module),
                Err(err) => errors.push(err),
            }
        })
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    let ir = Compiler::new(modules).consume()?;
    let mut jit = JIT::new(symbols);

    for module in &ir {
        jit.jit_module(module);
    }
    Ok(jit.exec("main"))
}

#[cfg(test)]
mod test {
    use crate::{execute_module, execute_with_os_fs};
    extern crate std;
    use crate::vm::SymbolTable;
    use core::fmt::Debug;
    use std::format;

    fn directory<T: Debug + PartialEq>(dir: &str, expect: T, symbols: SymbolTable) {
        let res = execute_with_os_fs::<T>(&[dir], symbols).unwrap();
        assert_eq!(res, expect)
    }

    fn file<T: Debug + PartialEq>(input: &str, expect: T) {
        let res = execute_module::<T>(input, &[]).unwrap();
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

    #[test]
    fn basic_modules() {
        directory(
            "tests/basic_modules",
            13,
            &[("hello", (|| 13) as fn() -> i64 as *const u8)],
        );
    }
}
