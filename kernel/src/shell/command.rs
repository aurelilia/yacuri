use alloc::{
    format,
    string::{String, ToString},
};
use logos::{Lexer, Logos};

#[derive(Debug)]
pub enum Command {
    Ls { directory: Option<String> },
    Cat { file: String },
    Cd { directory: String },
    Mkdir { directory: String },
    Put { file: String, text: String },
    Exec { file: String },
}

impl Command {
    pub fn from(input: &str) -> Result<Option<Command>, String> {
        let mut lexer = Lexer::<Token>::new(input);
        let cmd = lexer.next();
        match cmd {
            Some(Token::Ls) => Ok(Some(Command::Ls {
                directory: optional_path_arg(&mut lexer)?,
            })),

            Some(Token::Cat) => Ok(Some(Command::Cat {
                file: path_arg(&mut lexer)?,
            })),

            Some(Token::Cd) => Ok(Some(Command::Cd {
                directory: path_arg(&mut lexer)?,
            })),

            Some(Token::Mkdir) => Ok(Some(Command::Mkdir {
                directory: path_arg(&mut lexer)?,
            })),

            Some(Token::Put) => Ok(Some(Command::Put {
                file: path_arg(&mut lexer)?,
                text: path_arg(&mut lexer)?, // todo technically not a path, eh whatever
            })),

            Some(Token::Exec) => Ok(Some(Command::Exec {
                file: path_arg(&mut lexer)?,
            })),

            None => Ok(None),
            _ => Err(format!(
                "Expected a command, found '{}' ({:?}).",
                lexer.slice(),
                cmd
            )),
        }
    }
}

fn path_arg(lexer: &mut Lexer<Token>) -> Result<String, String> {
    match lexer.next() {
        Some(Token::Word | Token::Path | Token::Int) => Ok(lexer.slice().to_string()),
        Some(Token::Quote) => Ok(lexer.slice()[1..lexer.slice().len() - 1].to_string()),
        _ => Err(format!("Expected path, found '{}'", lexer.slice())),
    }
}

fn optional_path_arg(lexer: &mut Lexer<Token>) -> Result<Option<String>, String> {
    match lexer.next() {
        Some(Token::Word | Token::Path | Token::Int) => Ok(Some(lexer.slice().to_string())),
        Some(Token::Quote) => Ok(Some(lexer.slice()[1..lexer.slice().len() - 1].to_string())),
        None => Ok(None),
        _ => Err(format!("Expected path, found '{}'", lexer.slice())),
    }
}

fn expect(expected: Token, was: Token) -> Result<(), String> {
    if was == expected {
        Ok(())
    } else {
        Err(format!("Expected '{:?}', found '{:?}'.", expected, was))
    }
}

/// A direct token that implements Logos. Most are keywords or special chars.
/// The `Error` token is a special token signifying a syntax error.
#[derive(Logos, PartialEq, Eq, Debug, Clone, Copy, Hash)]
enum Token {
    #[token("ls")]
    Ls,
    #[token("cat")]
    Cat,
    #[token("cd")]
    Cd,
    #[token("mkdir")]
    Mkdir,
    #[token("put")]
    Put,
    #[token("exec")]
    Exec,

    #[regex("[a-zA-Z_][a-zA-Z0-9_]*", priority = 2)]
    Word,
    #[regex("[a-zA-Z0-9_/.]*")]
    Path,
    #[regex("\"[^\"]*\"")]
    Quote,
    #[regex(r"[0-9]+(?:(i|u)(size|8|16|32|64))?")]
    Int,
    #[regex(r"[0-9]+\.[0-9]+(?:(f)(32|64))?")]
    Float,

    #[regex(r"/\*([^*]|\*+[^*/])*\*?")] // https://github.com/maciejhirsz/logos/issues/180
    #[regex(r"[ \t\n\f]+", logos::skip)] // Whitespace
    #[regex(r"#[^\n]*", logos::skip)] // Comment
    #[error]
    Error,
}
