#![feature(box_syntax)]
#![no_std]

extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod compiler;
mod error;
mod lexer;
mod parser;
mod vm;
