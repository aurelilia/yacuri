pub use logos::{Logos, Span};
pub use token::Token;

pub struct Lexer<'l> {
    logos: logos::Lexer<'l, Token>,
}

impl<'l> Lexer<'l> {
    pub fn span(&self) -> Span {
        self.logos.span()
    }

    pub fn new(input: &'l str) -> Self {
        Self {
            logos: Token::lexer(input),
        }
    }
}

impl<'l> Iterator for Lexer<'l> {
    type Item = (Token, &'l str);

    fn next(&mut self) -> Option<Self::Item> {
        let kind = self.logos.next()?;
        let text = self.logos.slice();
        Some((kind, text))
    }
}

/// A direct token that implements Logos. Most are keywords or special chars.
/// The `Error` token is a special token signifying a syntax error.
#[derive(Logos, PartialEq, Eq, Debug, Clone, Copy, Hash)]
pub enum Token {
    #[token("(")]
    LeftParen,
    #[token(")")]
    RightParen,
    #[token("[")]
    LeftBracket,
    #[token("]")]
    RightBracket,
    #[token("{")]
    LeftBrace,
    #[token("}")]
    RightBrace,
    #[token("~")]
    Tilde,
    #[token(",")]
    Comma,
    #[token(".")]
    Dot,
    #[token("-")]
    Minus,
    #[token("+")]
    Plus,
    #[token(";")]
    Semicolon,
    #[token(":")]
    Colon,
    #[token("::")]
    ColonColon,
    #[token("/")]
    Slash,
    #[token("*")]
    Star,
    #[token("->")]
    Arrow,
    #[token("?")]
    QuestionMark,

    #[token("!")]
    Bang,
    #[token("!=")]
    BangEqual,
    #[token("=")]
    Equal,
    #[token("==")]
    EqualEqual,
    #[token(">")]
    Greater,
    #[token(">=")]
    GreaterEqual,
    #[token("<")]
    Less,
    #[token("<=")]
    LessEqual,

    #[regex("[a-zA-Z_][a-zA-Z0-9_]*")]
    Identifier,
    #[regex("\"[^\"]*\"")]
    String,
    #[regex(r"[0-9]+(?:(i|u)(size|8|16|32|64))?")]
    Int,
    #[regex(r"[0-9]+\.[0-9]+(?:(f)(32|64))?")]
    Float,

    #[token("and")]
    And,
    #[token("break")]
    Break,
    #[token("class")]
    Class,
    #[token("else")]
    Else,
    #[token("enum")]
    Enum,
    #[token("false")]
    False,
    #[token("for")]
    For,
    #[token("func")]
    Func,
    #[token("if")]
    If,
    #[token("import")]
    Import,
    #[token("in")]
    In,
    #[token("interface")]
    Interface,
    #[token("is")]
    Is,
    #[token("null")]
    Null,
    #[token("or")]
    Or,
    #[token("return")]
    Return,
    #[token("true")]
    True,
    #[token("var")]
    Var,
    #[token("val")]
    Val,
    #[token("when")]
    When,

    #[regex(r"/\*([^*]|\*+[^*/])*\*?")] // https://github.com/maciejhirsz/logos/issues/180
    #[error]
    Error,

    #[regex(r"//[^\n]*")]
    #[regex(r"/\*([^*]|\**[^*/])*\*+/")]
    Comment,

    #[regex(r"[ \t\n\f]+")]
    Whitespace,
}
