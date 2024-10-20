use std::{
    io::stderr,
    path::{Path, PathBuf},
};

use async_recursion::async_recursion;
use crossterm::{
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor, Stylize},
};
use glob::glob;
use hashbrown::HashSet;
use lexer::{Token, Tokenizer};
use tokio::{
    fs::{read_dir, read_to_string, write},
    task::{yield_now, JoinSet},
};

use crate::{
    config::ProjectConfig,
    file::{get_includes, get_language, Language},
};

use super::get_project_path;

mod lexer;

#[derive(Debug, Clone)]
pub struct FormatOptions {
    pub tab: String,
}

pub async fn format(
    files: Vec<String>,
    config_file: String,
    format_options: &FormatOptions,
) -> anyhow::Result<()> {
    let mut join_set = JoinSet::new();

    if files.is_empty() {
        let (project_path, project_config_path) = &get_project_path(&config_file);

        match ProjectConfig::load(project_config_path) {
            Ok(project_config) => {
                if let Some(package) = project_config.package {
                    let mut explored_path = HashSet::new();

                    for path in package.sources.iter() {
                        format_dir(
                            &mut join_set,
                            &mut explored_path,
                            &project_path.join(path),
                            &project_path,
                            &package.includes,
                            format_options,
                        )
                        .await;
                    }
                }
            }
            Err(error) => ProjectConfig::handle_error(error, project_config_path)?,
        }
    } else {
        for file in files {
            for path in glob(&file)? {
                join_set.spawn({
                    let format_options = format_options.clone();

                    async move { apply_format(&path?, &format_options).await }
                });
            }
        }
    }

    while let Some(handle) = join_set.join_next().await {
        handle??;
    }

    Ok(())
}

#[async_recursion]
async fn format_dir(
    join_set: &mut JoinSet<anyhow::Result<()>>,
    explored_path: &mut HashSet<PathBuf>,
    dir_path: &Path,
    project_path: &Path,
    include_path_vec: &Vec<PathBuf>,
    format_options: &FormatOptions,
) {
    if let Ok(mut read_dir) = read_dir(dir_path).await {
        while let Ok(Some(entry)) = read_dir.next_entry().await {
            let path = entry.path();

            if !explored_path.contains(&path) {
                explored_path.insert(path.clone());

                if path.is_file() {
                    format_file(
                        join_set,
                        explored_path,
                        &path,
                        project_path,
                        include_path_vec,
                        format_options,
                    )
                    .await;
                } else if path.is_dir() {
                    format_dir(
                        join_set,
                        explored_path,
                        &path,
                        project_path,
                        include_path_vec,
                        format_options,
                    )
                    .await;
                }
            }
        }
    }
}

#[async_recursion]
async fn format_file(
    join_set: &mut JoinSet<anyhow::Result<()>>,
    explored_path: &mut HashSet<PathBuf>,
    file_path: &Path,
    project_path: &Path,
    include_path_vec: &Vec<PathBuf>,
    format_options: &FormatOptions,
) {
    if let Some(extension) = file_path.extension() {
        if let Language::C | Language::Cpp = get_language(extension) {
            join_set.spawn({
                let file_path = file_path.to_path_buf();
                let format_options = format_options.clone();

                async move { apply_format(&file_path, &format_options).await }
            });

            if let Ok(code) = read_to_string(&file_path).await {
                for include_path in get_includes(&file_path, project_path, include_path_vec, &code)
                {
                    if !explored_path.contains(&include_path) {
                        explored_path.insert(include_path.clone());
                        format_file(
                            join_set,
                            explored_path,
                            &include_path,
                            project_path,
                            include_path_vec,
                            format_options,
                        )
                        .await;
                    }
                }
            }
        }
    }
}

async fn apply_format(path: &Path, format_options: &FormatOptions) -> anyhow::Result<()> {
    let code = read_to_string(path).await?;
    let tokenizer = Tokenizer::new(&code);
    let formatter = Formatter::new(tokenizer.lex().await, format_options.clone());

    let Ok(code) = formatter.format().await else {
        execute!(
            stderr(),
            SetForegroundColor(Color::Red),
            Print("Failed to format : ".bold()),
            ResetColor,
            Print(path.to_string_lossy()),
            Print("\n"),
        )?;

        return Ok(());
    };

    write(path, code).await?;

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scope {
    Block,
    Parenthesis,
}

#[derive(Debug)]
struct Formatter {
    options: FormatOptions,
    tokens: Vec<Token>,
    token_index: usize,
    scope: Vec<Scope>,
    formatted_code: String,
}

impl Formatter {
    pub fn new(tokens: Vec<Token>, options: FormatOptions) -> Self {
        Self {
            options,
            tokens,
            token_index: 0,
            scope: Vec::new(),
            formatted_code: String::new(),
        }
    }

    fn tab(&self) -> String {
        self.options.tab.repeat(self.scope.len())
    }

    fn add_code<T: AsRef<str>>(&mut self, code: T) {
        self.formatted_code += code.as_ref();
    }

    fn newline(&mut self) {
        self.formatted_code =
            self.formatted_code.trim_end_matches(' ').to_string() + "\n" + &self.tab()
    }

    fn trim_end(&mut self) {
        self.formatted_code = self.formatted_code.trim_end().to_string()
    }

    fn trim_end_whitespace(&mut self) {
        self.formatted_code = self.formatted_code.trim_end_matches(' ').to_string()
    }

    fn get(&self) -> Result<Token, ()> {
        self.tokens.get(self.token_index).cloned().ok_or(())
    }

    fn scope(&self) -> Option<Scope> {
        self.scope.last().copied()
    }

    pub async fn format(mut self) -> Result<String, ()> {
        let mut newline_count = 0u8;

        while self.token_index < self.tokens.len() {
            yield_now().await;

            let token = self.get()?;
            let token_string = token.to_string();

            if token != Token::Newline {
                newline_count = 0;
            }

            match token.clone() {
                Token::Newline => {
                    if newline_count < 2 {
                        if newline_count == 1 {
                            self.newline();
                        }

                        newline_count += 1;
                    }

                    self.token_index += 1;
                    continue;
                }
                Token::Space | Token::Tab => {
                    self.token_index += 1;
                    continue;
                }
                Token::Public
                | Token::Protected
                | Token::Private
                | Token::Case
                | Token::Default => {
                    self.trim_end_whitespace();

                    let scope = self.scope.pop();

                    self.add_code(self.tab());
                    self.add_code(token_string);
                    self.add_code(" ");

                    if let Some(scope) = scope {
                        self.scope.push(scope);
                    }

                    self.token_index += 1;
                }
                Token::Arrow | Token::Period | Token::ScopeResolution => {
                    self.trim_end();
                    self.add_code(token_string);

                    self.token_index += 1;
                }
                Token::Comma => {
                    self.trim_end();
                    self.add_code(token_string);
                    self.add_code(" ");

                    self.token_index += 1;
                }
                Token::Colon => {
                    self.trim_end();
                    self.add_code(token_string);
                    self.newline();

                    self.token_index += 1;
                }
                Token::SemiColon => {
                    self.trim_end();
                    self.add_code(token_string);

                    if self.scope() != Some(Scope::Parenthesis) {
                        self.newline();
                    } else {
                        self.add_code(" ");
                    }

                    self.token_index += 1;
                }
                Token::LeftParenthesis => {
                    for token in self.tokens[..self.token_index].iter().rev() {
                        match token {
                            Token::Identifier(_) | Token::Sizeof => {
                                self.trim_end();
                                break;
                            }
                            Token::Space | Token::Tab | Token::Newline => {}
                            _ => break,
                        }
                    }

                    self.add_code(token_string);
                    self.scope.push(Scope::Parenthesis);

                    self.token_index += 1;
                }
                Token::RightParenthesis => {
                    match self.scope.pop() {
                        Some(Scope::Parenthesis) => {}
                        _ => return Err(()),
                    }

                    self.trim_end();
                    self.add_code(token_string);
                    self.add_code(" ");

                    self.token_index += 1;
                }
                Token::LeftBrace => {
                    self.trim_end();
                    self.newline();
                    self.add_code(token_string);
                    self.scope.push(Scope::Block);
                    self.newline();

                    self.token_index += 1;
                }
                Token::RightBrace => {
                    match self.scope.pop() {
                        Some(Scope::Block) => {}
                        _ => return Err(()),
                    }

                    self.trim_end();
                    self.newline();
                    self.add_code(token_string);
                    self.newline();

                    self.token_index += 1;
                }
                Token::LeftBracket => {
                    self.trim_end();
                    self.add_code(token_string);

                    self.token_index += 1;
                }
                Token::RightBracket => {
                    self.trim_end();
                    self.add_code(token_string);
                    self.add_code(" ");

                    self.token_index += 1;
                }
                Token::Tilde | Token::Not => {
                    self.trim_end();
                    self.add_code(token_string);

                    self.token_index += 1;
                }
                Token::Ampersand | Token::Times | Token::Plus | Token::Minus => {
                    let token_index = self.token_index;

                    self.add_code(token_string);

                    loop {
                        self.token_index -= 1;

                        let token = self.get()?;

                        match token {
                            Token::Space
                            | Token::Tab
                            | Token::Newline
                            | Token::SimpleComment(_)
                            | Token::MultilineComment(_) => {}
                            Token::Identifier(_)
                            | Token::BoolValue(_)
                            | Token::CharValue(_)
                            | Token::StringValue(_)
                            | Token::FloatValue(_)
                            | Token::DigitValue(_)
                            | Token::RightBrace
                            | Token::RightBracket
                            | Token::RightParenthesis => {
                                self.add_code(" ");
                                break;
                            }
                            _ => break,
                        }
                    }

                    self.token_index = token_index + 1;
                }
                Token::Decrement | Token::Increment => {
                    self.token_index -= 1;

                    let token = self.get()?;

                    if token != Token::Space {
                        self.trim_end();
                    }

                    self.add_code(token_string);

                    if token != Token::Space {
                        self.add_code(" ");
                    }

                    self.token_index += 2;
                }
                Token::For | Token::While | Token::If => {
                    self.add_code(token_string);
                    self.add_code(" ");

                    self.token_index += 1;

                    while self.token_index < self.tokens.len() {
                        let token = self.get()?;

                        if token == Token::LeftParenthesis {
                            break;
                        }

                        self.token_index += 1;
                    }
                }
                Token::Macro(_) => {
                    self.trim_end_whitespace();
                    self.add_code(token_string);

                    self.token_index += 1;

                    while self.token_index < self.tokens.len() {
                        let token = self.get()?;

                        match token {
                            Token::SimpleComment(_) | Token::MultilineComment(_) => break,
                            Token::Newline => {
                                self.newline();
                                break;
                            }
                            _ => self.add_code(token.to_string()),
                        }

                        self.token_index += 1;
                    }
                }
                Token::SimpleComment(_) => {
                    self.add_code(token_string);
                    self.newline();

                    self.token_index += 1;
                }
                Token::MultilineComment(_) => {
                    let lines: Vec<&str> = token_string.lines().collect();

                    if lines[1..]
                        .iter()
                        .all(|line| line.trim_start().starts_with("*"))
                    {
                        self.add_code(
                            lines
                                .into_iter()
                                .map(|line| format!("{} {}", self.tab(), line.trim_start()))
                                .collect::<Vec<String>>()
                                .join("\n")
                                .trim_start(),
                        );
                    } else {
                        self.add_code(token_string);
                    };

                    self.token_index += 1;

                    let token = self.get()?;

                    if token == Token::Newline {
                        self.newline();
                    }
                }
                _ => {
                    self.add_code(token_string);
                    self.add_code(" ");

                    self.token_index += 1;
                }
            }
        }

        Ok(self.formatted_code.trim().to_string())
    }
}
