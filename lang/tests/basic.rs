use yacari::asm::make_asm;

#[test]
fn basic() {
    let source = "fun main() { 5 + 5 }";
    yacari::execute_program(source).unwrap();

    panic!("{}", make_asm())
}
