use std::ops::Range;

use tokio::task::yield_now;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Identifier(String),
    Macro(String),
    SimpleComment(String),
    MultilineComment(String),

    // literals
    DigitValue(String),
    FloatValue(String),
    StringValue(String),
    BoolValue(String),
    CharValue(String),

    // operators
    Plus,
    Minus,
    Times,
    Slash,
    Modulo,
    Period,
    QuestionMark,
    Ampersand,
    Caret,
    Pipe,
    Tilde,
    LeftShift,
    RightShift,
    Increment,
    Decrement,
    Arrow,
    ScopeResolution,

    // boolean operators
    EqualEqual,
    Not,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    Or,
    And,

    // assignment operators
    Equal,
    PlusEqual,
    MinusEqual,
    TimesEqual,
    SlashEqual,
    ModuloEqual,
    AmpersandEqual,
    CaretEqual,
    PipeEqual,
    LeftShiftEqual,
    RightShiftEqual,

    // seperators
    LeftParenthesis,
    RightParenthesis,
    LeftBracket,
    RightBracket,
    LeftBrace,
    RightBrace,
    Colon,
    SemiColon,
    Comma,

    // keywords
    Asm,
    Double,
    New,
    Switch,
    Auto,
    Else,
    Operator,
    Template,
    Break,
    Enum,
    Private,
    This,
    Case,
    Extern,
    Protected,
    Throw,
    Catch,
    Float,
    Public,
    Try,
    Char,
    For,
    Register,
    Typedef,
    Class,
    Friend,
    Return,
    Union,
    Const,
    Goto,
    Short,
    Unsigned,
    Continue,
    If,
    Signed,
    Virtual,
    Default,
    Inline,
    Sizeof,
    Void,
    Delete,
    Int,
    Static,
    Volatile,
    Do,
    Long,
    Struct,
    While,
    Null,
    Namespace,

    // unique
    Unknown(char),
    Newline,
    Space,
    Tab,
    EndOfFile,
}

impl ToString for Token {
    fn to_string(&self) -> String {
        match self {
            Token::Identifier(identifier) => identifier.to_string(),
            Token::Macro(r#macro) => "#".to_string() + r#macro,
            Token::SimpleComment(comment) => "//".to_string() + comment,
            Token::MultilineComment(comment) => "/*".to_string() + comment + "*/",
            Token::DigitValue(digit) => digit.to_string(),
            Token::FloatValue(float) => float.to_string(),
            Token::StringValue(string) => "\"".to_string() + string + "\"",
            Token::BoolValue(r#bool) => r#bool.to_string(),
            Token::CharValue(char) => "'".to_string() + char + "'",
            Token::Plus => "+".to_string(),
            Token::Minus => "-".to_string(),
            Token::Times => "*".to_string(),
            Token::Slash => "/".to_string(),
            Token::Modulo => "%".to_string(),
            Token::Period => ".".to_string(),
            Token::QuestionMark => "?".to_string(),
            Token::Ampersand => "&".to_string(),
            Token::Caret => "^".to_string(),
            Token::Pipe => "|".to_string(),
            Token::Tilde => "~".to_string(),
            Token::LeftShift => "<<".to_string(),
            Token::RightShift => ">>".to_string(),
            Token::Increment => "++".to_string(),
            Token::Decrement => "--".to_string(),
            Token::Arrow => "->".to_string(),
            Token::ScopeResolution => "::".to_string(),
            Token::EqualEqual => "==".to_string(),
            Token::Not => "!".to_string(),
            Token::NotEqual => "!=".to_string(),
            Token::Less => "<".to_string(),
            Token::Greater => ">".to_string(),
            Token::LessEqual => "<=".to_string(),
            Token::GreaterEqual => ">=".to_string(),
            Token::Or => "||".to_string(),
            Token::And => "&&".to_string(),
            Token::Equal => "=".to_string(),
            Token::PlusEqual => "+=".to_string(),
            Token::MinusEqual => "-=".to_string(),
            Token::TimesEqual => "*=".to_string(),
            Token::SlashEqual => "/=".to_string(),
            Token::ModuloEqual => "%=".to_string(),
            Token::AmpersandEqual => "&=".to_string(),
            Token::CaretEqual => "^=".to_string(),
            Token::PipeEqual => "|=".to_string(),
            Token::LeftShiftEqual => "<<=".to_string(),
            Token::RightShiftEqual => ">>=".to_string(),
            Token::LeftParenthesis => "(".to_string(),
            Token::RightParenthesis => ")".to_string(),
            Token::LeftBracket => "[".to_string(),
            Token::RightBracket => "]".to_string(),
            Token::LeftBrace => "{".to_string(),
            Token::RightBrace => "}".to_string(),
            Token::Colon => ":".to_string(),
            Token::SemiColon => ";".to_string(),
            Token::Comma => ",".to_string(),
            Token::If => "if".to_string(),
            Token::Else => "else".to_string(),
            Token::While => "while".to_string(),
            Token::For => "for".to_string(),
            Token::Return => "return".to_string(),
            Token::Do => "do".to_string(),
            Token::New => "new".to_string(),
            Token::Delete => "delete".to_string(),
            Token::Null => "null".to_string(),
            Token::Unknown(unknown) => unknown.to_string(),
            Token::Newline => "\n".to_string(),
            Token::Space => " ".to_string(),
            Token::EndOfFile => "\0".to_string(),
            Token::Tab => "\t".to_string(),
            Token::Asm => "asm".to_string(),
            Token::Double => "double".to_string(),
            Token::Switch => "switch".to_string(),
            Token::Auto => "auto".to_string(),
            Token::Operator => "operator".to_string(),
            Token::Template => "template".to_string(),
            Token::Break => "break".to_string(),
            Token::Enum => "enum".to_string(),
            Token::Private => "private".to_string(),
            Token::This => "this".to_string(),
            Token::Case => "case".to_string(),
            Token::Extern => "extern".to_string(),
            Token::Protected => "protected".to_string(),
            Token::Throw => "thrown".to_string(),
            Token::Catch => "catch".to_string(),
            Token::Float => "float".to_string(),
            Token::Public => "public".to_string(),
            Token::Try => "try".to_string(),
            Token::Char => "char".to_string(),
            Token::Register => "register".to_string(),
            Token::Typedef => "typedef".to_string(),
            Token::Class => "class".to_string(),
            Token::Friend => "friend".to_string(),
            Token::Union => "union".to_string(),
            Token::Const => "const".to_string(),
            Token::Goto => "goto".to_string(),
            Token::Short => "short".to_string(),
            Token::Unsigned => "unsigned".to_string(),
            Token::Continue => "continue".to_string(),
            Token::Signed => "signed".to_string(),
            Token::Virtual => "virtual".to_string(),
            Token::Default => "default".to_string(),
            Token::Inline => "inline".to_string(),
            Token::Sizeof => "sizeof".to_string(),
            Token::Void => "void".to_string(),
            Token::Int => "int".to_string(),
            Token::Static => "static".to_string(),
            Token::Volatile => "volatile".to_string(),
            Token::Long => "long".to_string(),
            Token::Struct => "struct".to_string(),
            Token::Namespace => "namespace".to_string(),
        }
    }
}

pub type TokenArray = Vec<Token>;

pub struct Tokenizer {
    code: Vec<(usize, char)>,
    index: usize,
}

impl Tokenizer {
    pub fn new(code: &str) -> Self {
        Self {
            code: (code.to_string() + " ")
                .replace("\r\n", "\n")
                .replace("\r", "\n")
                .chars()
                .enumerate()
                .collect(),
            index: 0,
        }
    }

    fn next(&mut self) -> Option<(usize, char)> {
        let char = self.code.get(self.index).copied();

        self.index += 1;
        char
    }

    fn skip(&mut self) {
        self.index += 1;
    }

    fn back(&mut self) {
        self.index -= 1;
    }

    fn get(&self, range: Range<usize>) -> String {
        self.code[range]
            .iter()
            .map(|(_, char)| char.to_string())
            .collect::<Vec<String>>()
            .join("")
    }

    pub async fn lex(mut self) -> TokenArray {
        let mut tokens = TokenArray::with_capacity(1000);

        loop {
            yield_now().await;

            if let Some((index, char)) = self.next() {
                let mut token = Token::Unknown(char);

                match char {
                    '\0' => token = Token::EndOfFile,

                    '(' => token = Token::LeftParenthesis,
                    ')' => token = Token::RightParenthesis,
                    '{' => token = Token::LeftBrace,
                    '}' => token = Token::RightBrace,
                    '[' => token = Token::LeftBracket,
                    ']' => token = Token::RightBracket,

                    '+' => {
                        token = match self.next() {
                            Some((_, '=')) => Token::PlusEqual,
                            Some((_, '+')) => Token::Increment,
                            Some(_) => {
                                self.back();
                                Token::Plus
                            }
                            None => return tokens,
                        }
                    }
                    '-' => {
                        token = match self.next() {
                            Some((_, '=')) => Token::MinusEqual,
                            Some((_, '-')) => Token::Decrement,
                            Some((_, '>')) => Token::Arrow,
                            Some(_) => {
                                self.back();
                                Token::Minus
                            }
                            None => return tokens,
                        }
                    }
                    '*' => {
                        token = match self.next() {
                            Some((_, '=')) => Token::TimesEqual,
                            Some(_) => {
                                self.back();
                                Token::Times
                            }
                            None => return tokens,
                        }
                    }
                    '%' => {
                        token = match self.next() {
                            Some((_, '=')) => Token::ModuloEqual,
                            Some(_) => {
                                self.back();
                                Token::Modulo
                            }
                            None => return tokens,
                        }
                    }
                    '/' => {
                        token = match self.next() {
                            Some((_, '=')) => Token::SlashEqual,
                            Some((_, '/')) => {
                                let start_index = index;

                                loop {
                                    if let Some((index, char)) = self.next() {
                                        if let '\n' = char {
                                            self.back();
                                            break Token::SimpleComment(
                                                self.get(start_index + 2..index),
                                            );
                                        }
                                    } else {
                                        return tokens;
                                    }
                                }
                            }
                            Some((_, '*')) => {
                                let start_index = index;

                                loop {
                                    if let Some((_, char)) = self.next() {
                                        if let '*' = char {
                                            if let Some((index, '/')) = self.next() {
                                                break Token::MultilineComment(
                                                    self.get(start_index + 2..index - 1),
                                                );
                                            } else {
                                                self.back();
                                            }
                                        }
                                    } else {
                                        return tokens;
                                    }
                                }
                            }
                            Some(_) => {
                                self.back();
                                Token::Slash
                            }
                            None => return tokens,
                        }
                    }
                    '?' => token = Token::QuestionMark,
                    '&' => {
                        token = match self.next() {
                            Some((_, '=')) => Token::AmpersandEqual,
                            Some((_, '&')) => Token::And,
                            Some(_) => {
                                self.back();
                                Token::Ampersand
                            }
                            None => return tokens,
                        }
                    }
                    '^' => {
                        token = match self.next() {
                            Some((_, '=')) => Token::CaretEqual,
                            Some(_) => {
                                self.back();
                                Token::Caret
                            }
                            None => return tokens,
                        }
                    }
                    '|' => {
                        token = match self.next() {
                            Some((_, '=')) => Token::PipeEqual,
                            Some((_, '|')) => Token::Or,
                            Some(_) => {
                                self.back();
                                Token::Pipe
                            }
                            None => return tokens,
                        }
                    }
                    '~' => token = Token::Tilde,

                    ':' => {
                        token = match self.next() {
                            Some((_, ':')) => Token::ScopeResolution,
                            Some(_) => {
                                self.back();
                                Token::Colon
                            }
                            None => return tokens,
                        }
                    }
                    ';' => token = Token::SemiColon,
                    ',' => token = Token::Comma,
                    '.' => token = Token::Period,

                    '=' => {
                        token = match self.next() {
                            Some((_, '=')) => Token::EqualEqual,
                            Some(_) => {
                                self.back();
                                Token::Equal
                            }
                            None => return tokens,
                        }
                    }
                    '>' => {
                        token = match self.next() {
                            Some((_, '=')) => Token::GreaterEqual,
                            Some((_, '>')) => match self.next() {
                                Some((_, '=')) => Token::RightShiftEqual,
                                Some(_) => {
                                    self.back();
                                    Token::RightShift
                                }
                                None => return tokens,
                            },
                            Some(_) => {
                                self.back();
                                Token::Greater
                            }
                            None => return tokens,
                        }
                    }
                    '<' => {
                        token = match self.next() {
                            Some((_, '=')) => Token::LessEqual,
                            Some((_, '<')) => match self.next() {
                                Some((_, '=')) => Token::LeftShiftEqual,
                                Some(_) => {
                                    self.back();
                                    Token::LeftShift
                                }
                                None => return tokens,
                            },
                            Some(_) => {
                                self.back();
                                Token::Less
                            }
                            None => return tokens,
                        }
                    }
                    '!' => {
                        token = match self.next() {
                            Some((_, '=')) => Token::NotEqual,
                            Some(_) => {
                                self.back();
                                Token::Not
                            }
                            None => return tokens,
                        }
                    }

                    '\n' => token = Token::Newline,
                    ' ' => token = Token::Space,
                    '\t' => token = Token::Tab,

                    '"' => {
                        let start_index = index;

                        loop {
                            if let Some((index, char)) = self.next() {
                                match char {
                                    '"' => {
                                        token =
                                            Token::StringValue(self.get(start_index + 1..index));
                                        break;
                                    }
                                    '\\' => {
                                        self.skip();
                                    }
                                    _ => {}
                                }
                            } else {
                                return tokens;
                            }
                        }
                    }
                    '\'' => {
                        let start_index = index;

                        loop {
                            if let Some((index, char)) = self.next() {
                                match char {
                                    '\'' => {
                                        token = Token::CharValue(self.get(start_index + 1..index));
                                        break;
                                    }
                                    '\\' => {
                                        self.skip();
                                    }
                                    _ => {}
                                }
                            } else {
                                return tokens;
                            }
                        }
                    }

                    '#' => {
                        let start_index = index;
                        let mut end_index;

                        loop {
                            if let Some((index, char)) = self.next() {
                                end_index = index;

                                if !is_letter(char) {
                                    self.back();
                                    break;
                                }
                            } else {
                                return tokens;
                            }
                        }

                        token = Token::Macro(self.get(start_index + 1..end_index));
                    }

                    _ => {
                        if is_letter(char) || char == '_' {
                            let start_index = index;
                            let mut end_index;

                            loop {
                                if let Some((index, char)) = self.next() {
                                    end_index = index;

                                    if !is_letter(char) && !is_numeric(char) && char != '_' {
                                        self.back();
                                        break;
                                    }
                                } else {
                                    return tokens;
                                }
                            }

                            let content = self.get(start_index..end_index);

                            match content.as_str() {
                                "if" => token = Token::If,
                                "else" => token = Token::Else,
                                "for" => token = Token::For,
                                "while" => token = Token::While,
                                "return" => token = Token::Return,
                                "do" => token = Token::Do,
                                "new" => token = Token::New,
                                "delete" => token = Token::Delete,
                                "null" => token = Token::Null,
                                "asm" => token = Token::Asm,
                                "double" => token = Token::Double,
                                "switch" => token = Token::Switch,
                                "auto" => token = Token::Auto,
                                "operator" => token = Token::Operator,
                                "template" => token = Token::Template,
                                "break" => token = Token::Break,
                                "enum" => token = Token::Enum,
                                "private" => token = Token::Private,
                                "this" => token = Token::This,
                                "case" => token = Token::Case,
                                "extern" => token = Token::Extern,
                                "protected" => token = Token::Protected,
                                "throw" => token = Token::Throw,
                                "catch" => token = Token::Catch,
                                "float" => token = Token::Float,
                                "public" => token = Token::Public,
                                "try" => token = Token::Try,
                                "char" => token = Token::Char,
                                "register" => token = Token::Register,
                                "typedef" => token = Token::Typedef,
                                "class" => token = Token::Class,
                                "friend" => token = Token::Friend,
                                "union" => token = Token::Union,
                                "const" => token = Token::Const,
                                "goto" => token = Token::Goto,
                                "short" => token = Token::Short,
                                "unsigned" => token = Token::Unsigned,
                                "continue" => token = Token::Continue,
                                "signed" => token = Token::Signed,
                                "virtual" => token = Token::Virtual,
                                "default" => token = Token::Default,
                                "inline" => token = Token::Inline,
                                "sizeof" => token = Token::Sizeof,
                                "void" => token = Token::Void,
                                "int" => token = Token::Int,
                                "static" => token = Token::Static,
                                "volatile" => token = Token::Volatile,
                                "long" => token = Token::Long,
                                "struct" => token = Token::Struct,
                                "namespace" => token = Token::Namespace,
                                "true" | "false" => token = Token::BoolValue(content),
                                _ => token = Token::Identifier(content),
                            }
                        } else if is_numeric(char) {
                            let start_index = index;
                            let mut end_index;
                            let mut is_float = false;

                            loop {
                                if let Some((index, char)) = self.next() {
                                    end_index = index;

                                    if char == '.' {
                                        is_float = true;
                                    }

                                    if !is_numeric(char) && (!is_float || char != '.') {
                                        self.back();
                                        break;
                                    }
                                } else {
                                    return tokens;
                                }
                            }

                            let content = self.get(start_index..end_index);

                            if is_float {
                                token = Token::FloatValue(content);
                            } else {
                                token = Token::DigitValue(content);
                            }
                        }
                    }
                }

                tokens.push(token);
            } else {
                break;
            }

            if let Token::EndOfFile = tokens.last().unwrap() {
                break;
            }
        }

        tokens
    }
}

fn is_letter(char: char) -> bool {
    match char {
        'a'..='z' | 'A'..='Z' => true,
        _ => false,
    }
}

fn is_numeric(char: char) -> bool {
    match char {
        '0'..='9' => true,
        _ => false,
    }
}
