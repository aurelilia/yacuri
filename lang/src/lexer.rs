use crate::smol_str::SmolStr;
pub use logos::{Logos, Span};

pub struct Lexer<'l> {
    logos: logos::Lexer<'l, TKind>,
}

impl<'l> Lexer<'l> {
    pub fn new(input: &'l str) -> Self {
        Self {
            logos: TKind::lexer(input),
        }
    }
}

impl<'l> Iterator for Lexer<'l> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        let kind = self.logos.next()?;
        let lexeme = self.logos.slice();
        let span = self.logos.span();
        Some(Token {
            kind,
            lex: SmolStr::new(lexeme),
            start: span.start,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TKind,
    pub lex: SmolStr,
    pub start: usize,
}

/// A direct token that implements Logos. Most are keywords or special chars.
/// The `Error` token is a special token signifying a syntax error.
#[derive(Logos, PartialEq, Eq, Debug, Clone, Copy, Hash)]
pub enum TKind {
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
    #[token("fun")]
    Fun,
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

    #[regex(r"//[^\n]*", logos::skip)]
    #[regex(r"/\*([^*]|\**[^*/])*\*+/", logos::skip)]
    Comment,

    #[regex(r"[ \t\f]+", logos::skip)]
    Whitespace,
    #[token(r"[\n]+", logos::skip)]
    Newline,
}

impl TKind {
    pub fn infix_binding_power(&self) -> Option<(u8, u8)> {
        Some(match self {
            Self::Equal => (6, 5),
            Self::Or => (10, 9),
            Self::And => (12, 11),
            Self::BangEqual | Self::EqualEqual => (14, 13),
            Self::Less | Self::LessEqual | Self::Greater | Self::GreaterEqual => (16, 15),
            Self::Plus | Self::Minus => (16, 15),
            Self::Star | Self::Slash => (18, 17),
            Self::Is => (20, 19),
            _ => return None,
        })
    }

    pub fn prefix_binding_power(&self) -> Option<u8> {
        Some(match self {
            Self::Minus | Self::Bang => 30,
            _ => return None,
        })
    }

    pub fn is_binary_logic(&self) -> bool {
        match self {
            TKind::EqualEqual
            | TKind::BangEqual
            | TKind::Less
            | TKind::LessEqual
            | TKind::Greater
            | TKind::GreaterEqual
            | TKind::And
            | TKind::Or => true,
            _ => false,
        }
    }
}
