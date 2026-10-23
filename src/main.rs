use font8x8::{UnicodeFonts, BASIC_FONTS};
use minifb::{Key, MouseButton, MouseMode, Window, WindowOptions};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::{Read, Write},
    path::Path,
};

const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;
const MAX_SOURCE_BYTES: u64 = 10 * 1024 * 1024;
const MAX_PACKAGE_BYTES: u64 = 20 * 1024 * 1024;
const MAX_PACKAGE_ENTRIES: usize = 2;

const MANIFEST_NAME: &str = "manifest.json";

#[derive(Debug, Serialize, Deserialize)]
struct Manifest {
    format: String,
    format_version: u32,
    id: String,
    name: String,
    version: String,
    entry: String,
    permissions: Vec<String>,
}

fn usage() {
    eprintln!(
        "UNX prototype 0.1\nUsage:\n  unx run <source.unx|package.unx>\n  unx check <source.unx>\n  unx pack <project-directory> <output.unx>\n  unx inspect <package.unx>\n\nPrototype statements: say \"Hello\", set name to \"UNX\", set total to 2 + 3 * 4, say total, if total > 5 / else / end blocks, if name equals \"UNX\" say \"matched\", open window \"Title\", set window width/height to \"800\", button \"Click me\" say \"Clicked!\" or say variableName, # comment"
    );
}

fn parse_quoted(input: &str) -> Option<String> {
    let s = input.trim();
    if s.len() < 2 || !s.starts_with('"') || !s.ends_with('"') {
        return None;
    }

    let inner = &s[1..s.len() - 1];
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return None,
            '\\' => match chars.next()? {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                _ => return None,
            },
            other => out.push(other),
        }
    }
    Some(out)
}

#[derive(Debug, PartialEq)]
enum Statement {
    SayString { value: String, line: usize, column: usize },
    SayVariable { name: String, line: usize, column: usize },
    SetString { name: String, value: String, line: usize, column: usize },
    SetVariable { name: String, source: String, line: usize, column: usize },
    SetNumber { name: String, expression: Expr, line: usize, column: usize },
    IfEqualsSay { variable: String, expected: String, message: ConditionalMessage, line: usize, column: usize },
    IfEqualsBegin { variable: String, expected: String, line: usize, column: usize },
    IfCompareSay { variable: String, operator: ComparisonOperator, expression: Expr, message: ConditionalMessage, line: usize, column: usize },
    IfCompareBegin { variable: String, operator: ComparisonOperator, expression: Expr, line: usize, column: usize },
    ElseBlock { line: usize, column: usize },
    EndBlock { line: usize, column: usize },
    OpenWindow { title: String, line: usize, column: usize },
    SetWindowWidth { value: String, line: usize, column: usize },
    SetWindowHeight { value: String, line: usize, column: usize },
    Button { label: String, message: String, line: usize, column: usize },
    ButtonVariable { label: String, variable: String, line: usize, column: usize },
}

#[derive(Debug, PartialEq)]
enum ConditionalMessage {
    String(String),
    Variable(String),
}

#[derive(Debug, PartialEq)]
enum TokenKind {
    Say,
    Set,
    If,
    Equals,
    Else,
    End,
    To,
    Open,
    Window,
    Width,
    Height,
    Button,
    Number(i64),
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    LeftParen,
    RightParen,
    Greater,
    Less,
    GreaterEqual,
    LessEqual,
    EqualEqual,
    NotEqual,
    Identifier(String),
    String(String),
}

#[derive(Debug, PartialEq)]
struct Token {
    kind: TokenKind,
    column: usize,
}

fn tokenize_line(line: &str, line_no: usize, base_column: usize) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut offset = 0;
    while offset < line.len() {
        let rest = &line[offset..];
        let first = rest.chars().next().expect("offset is within line");
        if first == '#' {
            break;
        }
        if first == ' ' || first == '\t' {
            offset += first.len_utf8();
            continue;
        }
        let column = base_column + line[..offset].chars().count();
        if first == '"' {
            let cursor = offset + 1;
            let mut escaped = false;
            let mut closing = None;
            for (relative, ch) in line[cursor..].char_indices() {
                let absolute = cursor + relative;
                if escaped {
                    escaped = false;
                    continue;
                }
                if ch == '\\' {
                    escaped = true;
                } else if ch == '"' {
                    closing = Some(absolute);
                    break;
                }
            }
            let close = closing.ok_or_else(|| format!("line {line_no}, column {column}: unterminated string literal"))?;
            let literal = &line[offset..=close];
            let value = parse_quoted(literal).ok_or_else(|| {
                let bad_column = column;
                format!("line {line_no}, column {bad_column}: invalid string escape or string literal; expected a valid quoted string")
            })?;
            tokens.push(Token { kind: TokenKind::String(value), column });
            offset = close + 1;
            if offset < line.len() {
                let next = line[offset..].chars().next().expect("offset is within line");
                if next != ' ' && next != '\t' {
                    return Err(format!("line {line_no}, column {}: expected whitespace after string literal", base_column + line[..offset].chars().count()));
                }
            }
            continue;
        }
        let operator = match first {
            '+' => Some((TokenKind::Plus, 1)),
            '-' => Some((TokenKind::Minus, 1)),
            '*' => Some((TokenKind::Star, 1)),
            '/' => Some((TokenKind::Slash, 1)),
            '%' => Some((TokenKind::Percent, 1)),
            '(' => Some((TokenKind::LeftParen, 1)),
            ')' => Some((TokenKind::RightParen, 1)),
            '>' if rest.starts_with(">=") => Some((TokenKind::GreaterEqual, 2)),
            '<' if rest.starts_with("<=") => Some((TokenKind::LessEqual, 2)),
            '=' if rest.starts_with("==") => Some((TokenKind::EqualEqual, 2)),
            '!' if rest.starts_with("!=") => Some((TokenKind::NotEqual, 2)),
            '>' => Some((TokenKind::Greater, 1)),
            '<' => Some((TokenKind::Less, 1)),
            _ => None,
        };
        if let Some((kind, width)) = operator {
            tokens.push(Token { kind, column });
            offset += width;
            continue;
        }
        if first == '=' || first == '!' {
            return Err(format!("line {line_no}, column {column}: unexpected token '{first}'"));
        }
        let token_start = offset;
        while offset < line.len() {
            let ch = line[offset..].chars().next().expect("offset is within line");
            if ch == ' ' || ch == '\t' || "+-*/%()><=!".contains(ch) { break; }
            offset += ch.len_utf8();
        }
        let word = &line[token_start..offset];
        let kind = match word {
            "say" => TokenKind::Say,
            "set" => TokenKind::Set,
            "if" => TokenKind::If,
            "equals" => TokenKind::Equals,
            "else" => TokenKind::Else,
            "end" => TokenKind::End,
            "to" => TokenKind::To,
            "open" => TokenKind::Open,
            "window" => TokenKind::Window,
            "width" => TokenKind::Width,
            "height" => TokenKind::Height,
            "button" => TokenKind::Button,
            _ if word.parse::<i64>().is_ok() => TokenKind::Number(word.parse::<i64>().expect("integer was parsed successfully")),
            _ if word.chars().all(|c| c == '_' || c.is_ascii_alphanumeric()) => TokenKind::Identifier(word.to_string()),
            _ => return Err(format!("line {line_no}, column {column}: unexpected token '{word}'")),
        };
        tokens.push(Token { kind, column });
    }
    Ok(tokens)
}

fn is_valid_identifier(name: &str) -> bool {
    !name.is_empty() && name.chars().enumerate().all(|(i, c)| {
        c == '_' || (c.is_ascii_alphanumeric() && (i > 0 || !c.is_ascii_digit()))
    })
}

#[derive(Debug, PartialEq, Clone)]
enum Expr {
    Number(i64),
    Variable(String),
    Negate(Box<Expr>),
    Binary(Box<Expr>, ArithmeticOperator, Box<Expr>),
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum ArithmeticOperator { Add, Subtract, Multiply, Divide, Remainder }

#[derive(Debug, PartialEq, Clone, Copy)]
enum ComparisonOperator { Greater, Less, GreaterEqual, LessEqual, Equal, NotEqual }

fn comparison_operator(kind: &TokenKind) -> Option<ComparisonOperator> {
    match kind {
        TokenKind::Greater => Some(ComparisonOperator::Greater),
        TokenKind::Less => Some(ComparisonOperator::Less),
        TokenKind::GreaterEqual => Some(ComparisonOperator::GreaterEqual),
        TokenKind::LessEqual => Some(ComparisonOperator::LessEqual),
        TokenKind::EqualEqual => Some(ComparisonOperator::Equal),
        TokenKind::NotEqual => Some(ComparisonOperator::NotEqual),
        _ => None,
    }
}

fn parse_expression(tokens: &[Token], index: &mut usize, line: usize) -> Result<Expr, String> {
    parse_additive(tokens, index, line)
}

fn parse_additive(tokens: &[Token], index: &mut usize, line: usize) -> Result<Expr, String> {
    let mut expression = parse_multiplicative(tokens, index, line)?;
    loop {
        let operator = match tokens.get(*index).map(|token| &token.kind) {
            Some(TokenKind::Plus) => ArithmeticOperator::Add,
            Some(TokenKind::Minus) => ArithmeticOperator::Subtract,
            _ => break,
        };
        *index += 1;
        let right = parse_multiplicative(tokens, index, line)?;
        expression = Expr::Binary(Box::new(expression), operator, Box::new(right));
    }
    Ok(expression)
}

fn parse_multiplicative(tokens: &[Token], index: &mut usize, line: usize) -> Result<Expr, String> {
    let mut expression = parse_unary(tokens, index, line)?;
    loop {
        let operator = match tokens.get(*index).map(|token| &token.kind) {
            Some(TokenKind::Star) => ArithmeticOperator::Multiply,
            Some(TokenKind::Slash) => ArithmeticOperator::Divide,
            Some(TokenKind::Percent) => ArithmeticOperator::Remainder,
            _ => break,
        };
        *index += 1;
        let right = parse_unary(tokens, index, line)?;
        expression = Expr::Binary(Box::new(expression), operator, Box::new(right));
    }
    Ok(expression)
}

fn parse_unary(tokens: &[Token], index: &mut usize, line: usize) -> Result<Expr, String> {
    if matches!(tokens.get(*index).map(|token| &token.kind), Some(TokenKind::Minus)) {
        *index += 1;
        return Ok(Expr::Negate(Box::new(parse_unary(tokens, index, line)?)));
    }
    parse_primary(tokens, index, line)
}

fn parse_primary(tokens: &[Token], index: &mut usize, line: usize) -> Result<Expr, String> {
    let token = tokens.get(*index).ok_or_else(|| format!("line {line}: expected a number, numeric variable, or parenthesized expression"))?;
    *index += 1;
    match &token.kind {
        TokenKind::Number(value) => Ok(Expr::Number(*value)),
        TokenKind::Identifier(name) if is_valid_identifier(name) => Ok(Expr::Variable(name.clone())),
        TokenKind::LeftParen => {
            let expression = parse_expression(tokens, index, line)?;
            if !matches!(tokens.get(*index).map(|token| &token.kind), Some(TokenKind::RightParen)) {
                return Err(format!("line {line}, column {}: expected ')' to close expression", token.column));
            }
            *index += 1;
            Ok(expression)
        }
        _ => Err(format!("line {line}, column {}: expected a number, numeric variable, or parenthesized expression", token.column)),
    }
}

fn evaluate_expression(expression: &Expr, vars: &HashMap<String, String>, line: usize, column: usize) -> Result<i64, String> {
    match expression {
        Expr::Number(value) => Ok(*value),
        Expr::Variable(name) => {
            let value = vars.get(name).ok_or_else(|| format!("line {line}, column {column}: undefined variable '{name}'"))?;
            value.parse::<i64>().map_err(|_| format!("line {line}, column {column}: variable '{name}' is not an integer"))
        }
        Expr::Negate(inner) => evaluate_expression(inner, vars, line, column)?
            .checked_neg().ok_or_else(|| format!("line {line}, column {column}: integer overflow in negation")),
        Expr::Binary(left, operator, right) => {
            let left = evaluate_expression(left, vars, line, column)?;
            let right = evaluate_expression(right, vars, line, column)?;
            let value = match operator {
                ArithmeticOperator::Add => left.checked_add(right),
                ArithmeticOperator::Subtract => left.checked_sub(right),
                ArithmeticOperator::Multiply => left.checked_mul(right),
                ArithmeticOperator::Divide if right == 0 => return Err(format!("line {line}, column {column}: division by zero")),
                ArithmeticOperator::Remainder if right == 0 => return Err(format!("line {line}, column {column}: remainder by zero")),
                ArithmeticOperator::Divide => left.checked_div(right),
                ArithmeticOperator::Remainder => left.checked_rem(right),
            };
            value.ok_or_else(|| format!("line {line}, column {column}: integer overflow in arithmetic expression"))
        }
    }
}

fn compare_numbers(left: i64, operator: ComparisonOperator, right: i64) -> bool {
    match operator {
        ComparisonOperator::Greater => left > right,
        ComparisonOperator::Less => left < right,
        ComparisonOperator::GreaterEqual => left >= right,
        ComparisonOperator::LessEqual => left <= right,
        ComparisonOperator::Equal => left == right,
        ComparisonOperator::NotEqual => left != right,
    }
}

fn parse_program(source: &str) -> Result<Vec<Statement>, String> {
    let mut statements = Vec::new();
    let mut blocks: Vec<(usize, bool)> = Vec::new();
    for (index, raw) in source.lines().enumerate() {
        let line_no = index + 1;
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let leading_bytes = raw.len() - raw.trim_start().len();
        let base_column = raw[..leading_bytes].chars().count() + 1;
        let tokens = tokenize_line(trimmed, line_no, base_column)?;
        let Some(first) = tokens.first() else { continue; };
        match &first.kind {
            TokenKind::If => {
                if tokens.len() == 4 && tokens[2].kind == TokenKind::Equals {
                    let variable = match &tokens[1].kind {
                        TokenKind::Identifier(name) if is_valid_identifier(name) => name.clone(),
                        _ => return Err(format!("line {line_no}, column {}: expected a variable name after 'if'", tokens[1].column)),
                    };
                    let expected = match &tokens[3].kind {
                        TokenKind::String(value) => value.clone(),
                        _ => return Err(format!("line {line_no}, column {}: expected a quoted comparison value after 'equals'", tokens[3].column)),
                    };
                    blocks.push((line_no, false));
                    statements.push(Statement::IfEqualsBegin { variable, expected, line: line_no, column: first.column });
                    continue;
                }
                if tokens.len() >= 4 {
                    if let Some(operator) = comparison_operator(&tokens[2].kind) {
                        let variable = match &tokens[1].kind {
                            TokenKind::Identifier(name) if is_valid_identifier(name) => name.clone(),
                            _ => return Err(format!("line {line_no}, column {}: expected a numeric variable after 'if'", tokens[1].column)),
                        };
                        let say_index = tokens.iter().position(|token| token.kind == TokenKind::Say);
                        let expression_end = say_index.unwrap_or(tokens.len());
                        let mut expression_index = 3;
                        let expression = parse_expression(&tokens[3..expression_end], &mut expression_index, line_no)?;
                        if expression_index != expression_end - 3 {
                            let token = &tokens[3 + expression_index];
                            return Err(format!("line {line_no}, column {}: unexpected token in comparison expression", token.column));
                        }
                        if let Some(say_index) = say_index {
                            if say_index + 2 != tokens.len() {
                                return Err(format!("line {line_no}, column {}: expected 'say' followed by a quoted message or variable", tokens[say_index].column));
                            }
                            let message = match &tokens[say_index + 1].kind {
                                TokenKind::String(value) => ConditionalMessage::String(value.clone()),
                                TokenKind::Identifier(name) if is_valid_identifier(name) => ConditionalMessage::Variable(name.clone()),
                                _ => return Err(format!("line {line_no}, column {}: expected a quoted message or variable after 'say'", tokens[say_index + 1].column)),
                            };
                            statements.push(Statement::IfCompareSay { variable, operator, expression, message, line: line_no, column: first.column });
                        } else {
                            blocks.push((line_no, false));
                            statements.push(Statement::IfCompareBegin { variable, operator, expression, line: line_no, column: first.column });
                        }
                        continue;
                    }
                }
                if tokens.len() != 6 || tokens[2].kind != TokenKind::Equals || tokens[4].kind != TokenKind::Say {
                    return Err(format!("line {line_no}, column {}: expected 'if VARIABLE equals \"VALUE\"' for a block or 'if VARIABLE equals \"VALUE\" say \"MESSAGE\"' for a single-line condition", first.column));
                }
                let variable = match &tokens[1].kind {
                    TokenKind::Identifier(name) if is_valid_identifier(name) => name.clone(),
                    _ => return Err(format!("line {line_no}, column {}: expected a variable name after 'if'", tokens[1].column)),
                };
                let expected = match &tokens[3].kind {
                    TokenKind::String(value) => value.clone(),
                    _ => return Err(format!("line {line_no}, column {}: expected a quoted comparison value after 'equals'", tokens[3].column)),
                };
                let message = match &tokens[5].kind {
                    TokenKind::String(value) => ConditionalMessage::String(value.clone()),
                    TokenKind::Identifier(name) if is_valid_identifier(name) => ConditionalMessage::Variable(name.clone()),
                    _ => return Err(format!("line {line_no}, column {}: expected a quoted message or variable name after 'say'", tokens[5].column)),
                };
                statements.push(Statement::IfEqualsSay { variable, expected, message, line: line_no, column: first.column });
            }
            TokenKind::Else => {
                if tokens.len() != 1 {
                    return Err(format!("line {line_no}, column {}: 'else' must be alone on its line", first.column));
                }
                let Some((_, already_has_else)) = blocks.last_mut() else {
                    return Err(format!("line {line_no}, column {}: 'else' without a matching 'if'", first.column));
                };
                if *already_has_else {
                    return Err(format!("line {line_no}, column {}: duplicate 'else' for this 'if'", first.column));
                }
                *already_has_else = true;
                statements.push(Statement::ElseBlock { line: line_no, column: first.column });
            }
            TokenKind::End => {
                if tokens.len() != 1 {
                    return Err(format!("line {line_no}, column {}: 'end' must be alone on its line", first.column));
                }
                if blocks.pop().is_none() {
                    return Err(format!("line {line_no}, column {}: 'end' without a matching 'if'", first.column));
                }
                statements.push(Statement::EndBlock { line: line_no, column: first.column });
            }
            TokenKind::Button => {
                if tokens.len() != 4 || tokens[2].kind != TokenKind::Say {
                    return Err(format!("line {line_no}, column {}: expected 'button \"LABEL\" say \"MESSAGE\"'", first.column));
                }
                let label = match &tokens[1].kind {
                    TokenKind::String(value) if !value.trim().is_empty() => value.clone(),
                    _ => return Err(format!("line {line_no}, column {}: expected a non-empty quoted button label", tokens[1].column)),
                };
                match &tokens[3].kind {
                    TokenKind::String(message) => statements.push(Statement::Button {
                        label, message: message.clone(), line: line_no, column: first.column,
                    }),
                    TokenKind::Identifier(variable) if is_valid_identifier(variable) => {
                        statements.push(Statement::ButtonVariable {
                            label, variable: variable.clone(), line: line_no, column: first.column,
                        });
                    }
                    _ => return Err(format!("line {line_no}, column {}: expected a quoted message or variable name after button say", tokens[3].column)),
                }
            }
            TokenKind::Open => {
                if tokens.len() != 3 || tokens[1].kind != TokenKind::Window {
                    return Err(format!("line {line_no}, column {}: expected 'open window \"TITLE\"'", first.column));
                }
                let title = match &tokens[2].kind {
                    TokenKind::String(value) if !value.trim().is_empty() => value,
                    _ => return Err(format!("line {line_no}, column {}: expected a non-empty quoted window title", tokens[2].column)),
                };
                statements.push(Statement::OpenWindow { title: title.clone(), line: line_no, column: first.column });
            }
            TokenKind::Say => {
                if tokens.len() != 2 {
                    return Err(format!("line {line_no}, column {}: expected a quoted string or variable name after 'say'", first.column + 4));
                }
                match &tokens[1].kind {
                    TokenKind::String(value) => statements.push(Statement::SayString {
                        value: value.clone(), line: line_no, column: first.column,
                    }),
                    TokenKind::Identifier(name) if is_valid_identifier(name) => statements.push(Statement::SayVariable {
                        name: name.clone(), line: line_no, column: first.column,
                    }),
                    TokenKind::Identifier(_) => return Err(format!("line {line_no}, column {}: invalid variable name", tokens[1].column)),
                    _ => return Err(format!("line {line_no}, column {}: expected a quoted string or variable name after 'say'", tokens[1].column)),
                }
            }
            TokenKind::Set => {
                if tokens.len() == 5 && tokens[1].kind == TokenKind::Window {
                    if tokens[3].kind != TokenKind::To {
                        return Err(format!("line {line_no}, column {}: expected keyword 'to'", tokens[3].column));
                    }
                    let value = match &tokens[4].kind {
                        TokenKind::String(value) => value.clone(),
                        _ => return Err(format!("line {line_no}, column {}: expected a quoted dimension value", tokens[4].column)),
                    };
                    match tokens[2].kind {
                        TokenKind::Width => statements.push(Statement::SetWindowWidth { value, line: line_no, column: first.column }),
                        TokenKind::Height => statements.push(Statement::SetWindowHeight { value, line: line_no, column: first.column }),
                        _ => return Err(format!("line {line_no}, column {}: expected 'width' or 'height' after 'set window'", tokens[2].column)),
                    }
                    continue;
                }
                if tokens.len() < 4 {
                    return Err(format!("line {line_no}, column {}: expected 'set NAME to VALUE' or 'set window width/height to VALUE'", first.column + 4));
                }
                let name = match &tokens[1].kind {
                    TokenKind::Identifier(name) if is_valid_identifier(name) => name,
                    TokenKind::Identifier(_) => return Err(format!("line {line_no}, column {}: invalid variable name", tokens[1].column)),
                    _ => return Err(format!("line {line_no}, column {}: expected a variable name after 'set'", tokens[1].column)),
                };
                if tokens[2].kind != TokenKind::To {
                    return Err(format!("line {line_no}, column {}: expected keyword 'to'", tokens[2].column));
                }
                if matches!(&tokens[3].kind, TokenKind::String(_)) {
                    if tokens.len() != 4 {
                        return Err(format!("line {line_no}, column {}: unexpected token after string value", tokens[4].column));
                    }
                    if let TokenKind::String(value) = &tokens[3].kind {
                        statements.push(Statement::SetString { name: name.clone(), value: value.clone(), line: line_no, column: first.column });
                    }
                } else if tokens.len() == 4 {
                    match &tokens[3].kind {
                        TokenKind::Identifier(source) if is_valid_identifier(source) => statements.push(Statement::SetVariable {
                            name: name.clone(), source: source.clone(), line: line_no, column: first.column,
                        }),
                        TokenKind::Identifier(_) => return Err(format!("line {line_no}, column {}: invalid source variable name", tokens[3].column)),
                        _ => {
                            let mut expression_index = 0;
                            let expression = parse_expression(&tokens[3..], &mut expression_index, line_no)?;
                            if expression_index != tokens.len() - 3 {
                                return Err(format!("line {line_no}, column {}: unexpected token in numeric expression", tokens[3 + expression_index].column));
                            }
                            statements.push(Statement::SetNumber { name: name.clone(), expression, line: line_no, column: first.column });
                        }
                    }
                } else {
                    let mut expression_index = 0;
                    let expression = parse_expression(&tokens[3..], &mut expression_index, line_no)?;
                    if expression_index != tokens.len() - 3 {
                        return Err(format!("line {line_no}, column {}: unexpected token in numeric expression", tokens[3 + expression_index].column));
                    }
                    statements.push(Statement::SetNumber { name: name.clone(), expression, line: line_no, column: first.column });
                }
            }
            _ => return Err(format!("line {line_no}, column {}: unknown statement", first.column)),
        }
    }
    if let Some((line, _)) = blocks.first() {
        return Err(format!("line {line}, column 1: missing 'end' for conditional block"));
    }
    Ok(statements)
}

#[derive(Debug)]
struct ConditionalFrame {
    parent_active: bool,
    condition: bool,
    in_else: bool,
}

fn execute_program(statements: &[Statement], print_output: bool) -> Result<(), String> {
    let mut vars: HashMap<String, String> = HashMap::new();
    let mut window_title: Option<String> = None;
    let mut window_width: usize = 800;
    let mut window_height: usize = 500;
    let mut window_lines: Vec<String> = Vec::new();
    let mut window_buttons: Vec<(String, String)> = Vec::new();
    let mut conditions: Vec<ConditionalFrame> = Vec::new();

    for statement in statements {
        match statement {
            Statement::IfEqualsBegin { variable, expected, line, column } => {
                let parent_active = conditions.iter().all(|frame| {
                    frame.parent_active && if frame.in_else { !frame.condition } else { frame.condition }
                });
                let condition = if parent_active {
                    let actual = vars.get(variable).ok_or_else(|| {
                        format!("line {line}, column {column}: undefined variable '{variable}'")
                    })?;
                    actual == expected
                } else {
                    false
                };
                conditions.push(ConditionalFrame { parent_active, condition, in_else: false });
                continue;
            }
            Statement::IfCompareBegin { variable, operator, expression, line, column } => {
                let parent_active = conditions.iter().all(|frame| {
                    frame.parent_active && if frame.in_else { !frame.condition } else { frame.condition }
                });
                let condition = if parent_active {
                    let actual = vars.get(variable).ok_or_else(|| format!("line {line}, column {column}: undefined variable '{variable}'"))?;
                    let actual = actual.parse::<i64>().map_err(|_| format!("line {line}, column {column}: variable '{variable}' is not an integer"))?;
                    let expected = evaluate_expression(expression, &vars, *line, *column)?;
                    compare_numbers(actual, *operator, expected)
                } else { false };
                conditions.push(ConditionalFrame { parent_active, condition, in_else: false });
                continue;
            }
            Statement::ElseBlock { line, column } => {
                let Some(frame) = conditions.last_mut() else {
                    return Err(format!("line {line}, column {column}: 'else' without a matching 'if'"));
                };
                if frame.in_else {
                    return Err(format!("line {line}, column {column}: duplicate 'else' for this 'if'"));
                }
                frame.in_else = true;
                continue;
            }
            Statement::EndBlock { line, column } => {
                if conditions.pop().is_none() {
                    return Err(format!("line {line}, column {column}: 'end' without a matching 'if'"));
                }
                continue;
            }
            _ => {}
        }

        let active = conditions.iter().all(|frame| {
            frame.parent_active && if frame.in_else { !frame.condition } else { frame.condition }
        });
        if !active {
            continue;
        }

        match statement {
            Statement::IfEqualsBegin { .. } | Statement::IfCompareBegin { .. } | Statement::ElseBlock { .. } | Statement::EndBlock { .. } => unreachable!("handled above"),
            Statement::SayString { value, .. } => {
                if window_title.is_some() {
                    window_lines.push(value.clone());
                } else if print_output {
                    println!("{value}");
                }
            }
            Statement::SayVariable { name, line, column } => {
                let value = vars.get(name)
                    .ok_or_else(|| format!("line {line}, column {column}: undefined variable '{name}'"))?;
                if window_title.is_some() {
                    window_lines.push(value.clone());
                } else if print_output {
                    println!("{value}");
                }
            }
            Statement::SetString { name, value, .. } => {
                vars.insert(name.clone(), value.clone());
            }
            Statement::SetVariable { name, source, line, column } => {
                let value = vars.get(source).cloned().ok_or_else(|| {
                    format!("line {line}, column {column}: undefined variable '{source}'")
                })?;
                vars.insert(name.clone(), value);
            }
            Statement::SetNumber { name, expression, line, column } => {
                let value = evaluate_expression(expression, &vars, *line, *column)?;
                vars.insert(name.clone(), value.to_string());
            }
            Statement::IfEqualsSay { variable, expected, message, line, column } => {
                let actual = vars.get(variable).ok_or_else(|| {
                    format!("line {line}, column {column}: undefined variable '{variable}'")
                })?;
                if actual == expected {
                    let output = match message {
                        ConditionalMessage::String(value) => value.clone(),
                        ConditionalMessage::Variable(name) => vars.get(name).cloned().ok_or_else(|| {
                            format!("line {line}, column {column}: undefined variable '{name}'")
                        })?,
                    };
                    if window_title.is_some() {
                        window_lines.push(output);
                    } else if print_output {
                        println!("{output}");
                    }
                }
            }
            Statement::IfCompareSay { variable, operator, expression, message, line, column } => {
                let actual = vars.get(variable).ok_or_else(|| format!("line {line}, column {column}: undefined variable '{variable}'"))?;
                let actual = actual.parse::<i64>().map_err(|_| format!("line {line}, column {column}: variable '{variable}' is not an integer"))?;
                let expected = evaluate_expression(expression, &vars, *line, *column)?;
                if compare_numbers(actual, *operator, expected) {
                    let output = match message {
                        ConditionalMessage::String(value) => value.clone(),
                        ConditionalMessage::Variable(name) => vars.get(name).cloned().ok_or_else(|| format!("line {line}, column {column}: undefined variable '{name}'"))?,
                    };
                    if window_title.is_some() { window_lines.push(output); }
                    else if print_output { println!("{output}"); }
                }
            }
            Statement::OpenWindow { title, .. } => window_title = Some(title.clone()),
            Statement::Button { label, message, line, column } => {
                if window_title.is_none() {
                    return Err(format!("line {line}, column {column}: button requires an open window"));
                }
                window_buttons.push((label.clone(), message.clone()));
            }
            Statement::ButtonVariable { label, variable, line, column } => {
                if window_title.is_none() {
                    return Err(format!("line {line}, column {column}: button requires an open window"));
                }
                let message = vars.get(variable).ok_or_else(|| {
                    format!("line {line}, column {column}: undefined variable '{variable}'")
                })?;
                window_buttons.push((label.clone(), message.clone()));
            }
            Statement::SetWindowWidth { value, line, column } => {
                let parsed = value.parse::<usize>().map_err(|_| format!("line {line}, column {column}: window width must be a number"))?;
                if !(200..=1920).contains(&parsed) {
                    return Err(format!("line {line}, column {column}: window width must be between 200 and 1920"));
                }
                window_width = parsed;
            }
            Statement::SetWindowHeight { value, line, column } => {
                let parsed = value.parse::<usize>().map_err(|_| format!("line {line}, column {column}: window height must be a number"))?;
                if !(150..=1200).contains(&parsed) {
                    return Err(format!("line {line}, column {column}: window height must be between 150 and 1200"));
                }
                window_height = parsed;
            }
        }
    }

    if print_output {
        if let Some(title) = window_title {
            show_gui(&title, window_width, window_height, &window_lines, &window_buttons)?;
        }
    }
    Ok(())
}

fn draw_text(buffer: &mut [u32], width: usize, height: usize, x: usize, y: usize, text: &str, color: u32) {
    let scale = 2usize;
    let mut cursor_x = x;
    for ch in text.chars().take(width.saturating_sub(x) / (8 * scale)) {
        if let Some(glyph) = BASIC_FONTS.get(ch) {
            for (row, bits) in glyph.iter().enumerate() {
                for col in 0..8 {
                    if bits & (1 << col) != 0 {
                        for sy in 0..scale {
                            for sx in 0..scale {
                                let px = cursor_x + col * scale + sx;
                                let py = y + row * scale + sy;
                                if px < width && py < height {
                                    buffer[py * width + px] = color;
                                }
                            }
                        }
                    }
                }
            }
        }
        cursor_x += 8 * scale;
    }
}

fn show_gui(
    title: &str,
    width: usize,
    height: usize,
    lines: &[String],
    buttons: &[(String, String)],
) -> Result<(), String> {
    let mut window = Window::new(title, width, height, WindowOptions::default())
        .map_err(|e| format!("could not create GUI window: {e}"))?;
    let mut buffer = vec![0x00F4F7FB; width * height];
    let mut status = String::from("Click a button or press ESC to close");
    let mut previous_mouse_down = false;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        buffer.fill(0x00F4F7FB);
        draw_text(&mut buffer, width, height, 28, 28, title, 0x001B2A41);
        let mut y = 72usize;
        for line in lines.iter().take(height.saturating_sub(190) / 28) {
            draw_text(&mut buffer, width, height, 28, y, line, 0x002B3748);
            y += 28;
        }
        let button_top = (y + 16).max(120);
        let mouse_down = window.get_mouse_down(MouseButton::Left);
        if mouse_down && !previous_mouse_down {
            if let Some((mx, my)) = window.get_mouse_pos(MouseMode::Discard) {
                for (index, (_, message)) in buttons.iter().enumerate() {
                    let bx = 28.0_f32;
                    let by = (button_top + index * 52) as f32;
                    if mx >= bx && mx <= bx + 260.0 && my >= by && my <= by + 38.0 {
                        status = message.clone();
                        break;
                    }
                }
            }
        }
        previous_mouse_down = mouse_down;

        for (index, (label, _)) in buttons.iter().enumerate() {
            let bx = 28usize;
            let by = button_top + index * 52;
            if by + 38 < height.saturating_sub(45) {
                for py in by..by + 38 {
                    for px in bx..(bx + 260).min(width) {
                        buffer[py * width + px] = 0x00DCEAFE;
                    }
                }
                draw_text(&mut buffer, width, height, bx + 12, by + 10, label, 0x001D4ED8);
            }
        }
        draw_text(&mut buffer, width, height, 28, height.saturating_sub(32), &status, 0x006B7280);
        window.update_with_buffer(&buffer, width, height)
            .map_err(|e| format!("could not update GUI window: {e}"))?;
    }
    Ok(())
}

fn validate_and_execute(source: &str, print_output: bool) -> Result<(), String> {
    let program = parse_program(source)?;
    execute_program(&program, print_output)
}

fn is_safe_package_path(value: &str) -> bool {
    if value.is_empty()
        || value.starts_with('/')
        || value.contains(':')
        || value.bytes().any(|b| b == 92)
        || value.chars().any(char::is_control)
    {
        return false;
    }
    value.split('/').all(|part| !part.is_empty() && part != "." && part != "..")
}

fn validate_manifest(manifest: &Manifest) -> Result<(), String> {
    if manifest.format != "UNX" || manifest.format_version != 1 {
        return Err("unsupported UNX package format/version".into());
    }
    if !is_safe_package_path(&manifest.entry) || manifest.entry == MANIFEST_NAME {
        return Err("unsafe entry path in manifest".into());
    }
    if manifest.id.trim().is_empty() || manifest.name.trim().is_empty() || manifest.version.trim().is_empty() {
        return Err("manifest id, name, and version must not be empty".into());
    }
    Ok(())
}

fn reject_duplicate_zip_entries(path: &Path) -> Result<(), String> {
    // zip::ZipArchive may expose duplicate central-directory names as a single
    // logical entry, so inspect the central directory before opening the archive.
    let bytes = fs::read(path).map_err(|e| format!("cannot read package: {e}"))?;
    if bytes.len() < 22 {
        return Err("invalid ZIP package: missing end-of-central-directory record".into());
    }
    let search_start = bytes.len().saturating_sub(22 + u16::MAX as usize);
    // A signature can occur inside the ZIP comment, so only accept an EOCD
    // whose declared comment length reaches exactly to the end of the file.
    let eocd = (search_start..=bytes.len() - 22)
        .rev()
        .find(|&offset| {
            bytes.get(offset..offset + 4) == Some(b"PK\x05\x06")
                && offset + 22 + u16::from_le_bytes([bytes[offset + 20], bytes[offset + 21]]) as usize == bytes.len()
        })
        .ok_or_else(|| "invalid ZIP package: missing or malformed end-of-central-directory record".to_string())?;
    let disk_number = u16::from_le_bytes([bytes[eocd + 4], bytes[eocd + 5]]);
    let central_disk = u16::from_le_bytes([bytes[eocd + 6], bytes[eocd + 7]]);
    let entries_on_disk = u16::from_le_bytes([bytes[eocd + 8], bytes[eocd + 9]]) as usize;
    let entries = u16::from_le_bytes([bytes[eocd + 10], bytes[eocd + 11]]) as usize;
    if disk_number != 0 || central_disk != 0 || entries_on_disk != entries {
        return Err("multi-disk ZIP packages are not supported".into());
    }
    let central_size = u32::from_le_bytes(bytes[eocd + 12..eocd + 16].try_into().unwrap()) as usize;
    let central_offset = u32::from_le_bytes(bytes[eocd + 16..eocd + 20].try_into().unwrap()) as usize;
    if entries == u16::MAX as usize || central_size == u32::MAX as usize || central_offset == u32::MAX as usize {
        return Err("ZIP64 package directories are not supported".into());
    }
    let central_end = central_offset.checked_add(central_size)
        .filter(|&end| end <= eocd)
        .ok_or_else(|| "invalid ZIP package: central directory is out of bounds".to_string())?;
    if central_end != eocd {
        return Err("invalid ZIP package: central directory does not end at the EOCD record".into());
    }
    let mut cursor = central_offset;
    let mut names = std::collections::HashSet::new();
    for _ in 0..entries {
        if cursor + 46 > central_end || bytes.get(cursor..cursor + 4) != Some(b"PK\x01\x02") {
            return Err("invalid ZIP package: malformed central directory".into());
        }
        let name_len = u16::from_le_bytes([bytes[cursor + 28], bytes[cursor + 29]]) as usize;
        let extra_len = u16::from_le_bytes([bytes[cursor + 30], bytes[cursor + 31]]) as usize;
        let comment_len = u16::from_le_bytes([bytes[cursor + 32], bytes[cursor + 33]]) as usize;
        let name_start = cursor + 46;
        let name_end = name_start.checked_add(name_len)
            .filter(|&end| end <= central_end)
            .ok_or_else(|| "invalid ZIP package: truncated entry name".to_string())?;
        let name = bytes[name_start..name_end].to_vec();
        if !names.insert(name.clone()) {
            let display_name = String::from_utf8_lossy(&name);
            return Err(format!("duplicate ZIP entry '{display_name}'"));
        }
        cursor = name_end.checked_add(extra_len).and_then(|n| n.checked_add(comment_len))
            .filter(|&end| end <= central_end)
            .ok_or_else(|| "invalid ZIP package: truncated central directory entry".to_string())?;
    }
    if cursor != central_end {
        return Err("invalid ZIP package: central directory size mismatch".into());
    }
    Ok(())
}

fn load_package(path: &Path) -> Result<(Manifest, String), String> {
    let metadata = fs::metadata(path).map_err(|e| format!("cannot inspect package: {e}"))?;
    if metadata.len() > MAX_PACKAGE_BYTES {
        return Err("package exceeds the 20 MiB size limit".into());
    }
    reject_duplicate_zip_entries(path)?;
    let file = File::open(path).map_err(|e| format!("cannot open package: {e}"))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("invalid ZIP package: {e}"))?;
    let mut names = std::collections::HashSet::new();
    for index in 0..archive.len() {
        let entry = archive.by_index(index).map_err(|e| format!("invalid ZIP entry: {e}"))?;
        let name = entry.name().to_string();
        if !names.insert(name.clone()) {
            return Err(format!("duplicate ZIP entry '{name}'"));
        }
        if entry.is_dir() {
            return Err(format!("directory entries are not allowed: '{name}'"));
        }
        if name != MANIFEST_NAME && !is_safe_package_path(&name) {
            return Err(format!("unsafe ZIP entry path '{name}'"));
        }
        if let Some(mode) = entry.unix_mode() {
            if mode & 0o170000 == 0o120000 {
                return Err(format!("symbolic-link ZIP entries are not allowed: '{name}'"));
            }
        }
    }
    if !names.contains(MANIFEST_NAME) {
        return Err("package is missing manifest.json".into());
    }

    if names.len() != MAX_PACKAGE_ENTRIES {
        return Err("package must contain exactly manifest.json and one entry source file".into());
    }

    let manifest: Manifest = {
        let mut entry = archive.by_name(MANIFEST_NAME).map_err(|_| "package is missing manifest.json".to_string())?;
        if entry.size() > MAX_MANIFEST_BYTES {
            return Err("manifest exceeds the 1 MiB size limit".into());
        }
        let mut text = String::new();
        entry.read_to_string(&mut text).map_err(|e| e.to_string())?;
        serde_json::from_str(&text).map_err(|e| format!("invalid manifest: {e}"))?
    };
    validate_manifest(&manifest)?;
    if !names.contains(&manifest.entry) {
        return Err(format!("package is missing entry file '{}'", manifest.entry));
    }
    if names.len() != 2 || manifest.entry == MANIFEST_NAME {
        return Err("package contains unexpected ZIP entries".into());
    }

    let mut entry = archive.by_name(&manifest.entry)
        .map_err(|_| format!("package is missing entry file '{}'", manifest.entry))?;
    if entry.size() > MAX_SOURCE_BYTES {
        return Err("entry source exceeds the 10 MiB size limit".into());
    }
    let mut source = String::new();
    entry.read_to_string(&mut source).map_err(|e| e.to_string())?;
    Ok((manifest, source))
}

fn command_run(path: &Path) -> Result<(), String> {
    if path.extension().and_then(|x| x.to_str()) == Some("unx") {
        let mut header = [0u8; 4];
        if let Ok(mut f) = File::open(path) {
            if f.read_exact(&mut header).is_ok() && &header == b"PK\x03\x04" {
                let (manifest, source) = load_package(path)?;
                eprintln!("Running {} {} ({})", manifest.name, manifest.version, manifest.id);
                return validate_and_execute(&source, true);
            }
        }
    }
    let source = fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    validate_and_execute(&source, true)
}

fn command_check(path: &Path) -> Result<(), String> {
    let source = fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    validate_and_execute(&source, false)?;
    println!("OK: source is valid");
    Ok(())
}

fn command_pack(project: &Path, output: &Path) -> Result<(), String> {
    let manifest_path = project.join(MANIFEST_NAME);
    let manifest_metadata = fs::symlink_metadata(&manifest_path)
        .map_err(|e| format!("cannot inspect manifest {}: {e}", manifest_path.display()))?;
    if !manifest_metadata.is_file() {
        return Err("manifest.json must be a regular file".into());
    }
    if manifest_metadata.len() > MAX_MANIFEST_BYTES {
        return Err("manifest exceeds the 1 MiB size limit".into());
    }
    let manifest_text = fs::read_to_string(&manifest_path).map_err(|e| format!("cannot read {}: {e}", manifest_path.display()))?;
    let manifest: Manifest = serde_json::from_str(&manifest_text).map_err(|e| format!("invalid manifest JSON: {e}"))?;
    validate_manifest(&manifest)?;
    let source_path = project.join(&manifest.entry);
    let source_metadata = fs::symlink_metadata(&source_path).map_err(|e| format!("cannot read entry {}: {e}", source_path.display()))?;
    if !source_metadata.is_file() {
        return Err("manifest entry must refer to a file".into());
    }
    if source_metadata.len() > MAX_SOURCE_BYTES {
        return Err("entry source exceeds the 10 MiB size limit".into());
    }
    let source = fs::read_to_string(&source_path).map_err(|e| format!("cannot read entry {}: {e}", source_path.display()))?;
    validate_and_execute(&source, false)?;
    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() { fs::create_dir_all(parent).map_err(|e| e.to_string())?; }
    }
    let file = File::create(output).map_err(|e| format!("cannot create output: {e}"))?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .last_modified_time(zip::DateTime::default())
        .unix_permissions(0o644);
    zip.start_file(MANIFEST_NAME, options).map_err(|e| e.to_string())?;
    zip.write_all(manifest_text.as_bytes()).map_err(|e| e.to_string())?;
    zip.start_file(&manifest.entry, options).map_err(|e| e.to_string())?;
    zip.write_all(source.as_bytes()).map_err(|e| e.to_string())?;
    zip.finish().map_err(|e| e.to_string())?;
    println!("Created {}", output.display());
    Ok(())
}

fn command_inspect(path: &Path) -> Result<(), String> {
    let (manifest, source) = load_package(path)?;
    println!("{}", serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?);
    println!("\n--- Entry source: {} ---\n{}", manifest.entry, source);
    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let result = match args.get(1).map(String::as_str) {
        Some("--help" | "-h" | "help") if args.len() == 2 => { usage(); return; }
        Some("--version" | "-V" | "version") if args.len() == 2 => { println!("unx {}", env!("CARGO_PKG_VERSION")); return; }
        Some("run") if args.len() == 3 => command_run(Path::new(&args[2])),
        Some("check") if args.len() == 3 => command_check(Path::new(&args[2])),
        Some("pack") if args.len() == 4 => command_pack(Path::new(&args[2]), Path::new(&args[3])),
        Some("inspect") if args.len() == 3 => command_inspect(Path::new(&args[2])),
        _ => {
            usage();
            std::process::exit(2);
        }
    };
    if let Err(error) = result {
        eprintln!("unx: {error}");
        std::process::exit(1);
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_quoted_strings_and_escapes() {
        assert_eq!(parse_quoted("\"hello\""), Some("hello".to_string()));
        assert_eq!(parse_quoted("\"a\\\\nb\""), Some("a\\nb".to_string()));
        assert_eq!(parse_quoted("\"line\\nnext\""), Some("line\nnext".to_string()));
        assert_eq!(parse_quoted("\"say \\\"hi\\\"\""), Some("say \"hi\"".to_string()));
        assert_eq!(parse_quoted("\"bad\\qescape\""), None);
        assert_eq!(parse_quoted("\"bad\"quote\""), None);
        assert_eq!(parse_quoted("not a string"), None);
    }

    #[test]
    fn tokenizer_preserves_unicode_inside_string_literals() {
        let tokens = tokenize_line("say \"Hello 🌳 世界\"", 1, 1).unwrap();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[1].kind, TokenKind::String("Hello 🌳 世界".into()));
    }

    #[test]
    fn parser_keeps_unicode_string_values_in_ast() {
        let program = parse_program("say \"こんにちは 🌳\"").unwrap();
        assert_eq!(program, vec![Statement::SayString {
            value: "こんにちは 🌳".into(), line: 1, column: 1,
        }]);
    }

    #[test]
    fn tokenizer_recognizes_keywords_identifiers_and_strings() {
        let tokens = tokenize_line("set name to \"UNX\"", 1, 1).unwrap();
        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0].kind, TokenKind::Set);
        assert_eq!(tokens[1].kind, TokenKind::Identifier("name".into()));
        assert_eq!(tokens[2].kind, TokenKind::To);
        assert_eq!(tokens[3].kind, TokenKind::String("UNX".into()));
    }

    #[test]
    fn tokenizer_rejects_adjacent_text_after_string() {
        let error = tokenize_line("say \"hello\"world", 1, 1).unwrap_err();
        assert!(error.contains("line 1, column 12"));
        assert!(error.contains("whitespace after string literal"));
    }

    #[test]
    fn tokenizer_rejects_incomplete_comparison_operators() {
        for operator in ["=", "!"] {
            let error = tokenize_line(operator, 1, 1).unwrap_err();
            assert!(error.contains("unexpected token"), "operator {operator:?}: {error}");
        }
    }

    #[test]
    fn parser_builds_numeric_expression_with_expected_precedence() {
        let program = parse_program("set total to 2 + 3 * 4").unwrap();
        assert_eq!(program.len(), 1);
        assert_eq!(
            program[0],
            Statement::SetNumber {
                name: "total".into(),
                expression: Expr::Binary(
                    Box::new(Expr::Number(2)),
                    ArithmeticOperator::Add,
                    Box::new(Expr::Binary(
                        Box::new(Expr::Number(3)),
                        ArithmeticOperator::Multiply,
                        Box::new(Expr::Number(4)),
                    )),
                ),
                line: 1,
                column: 1,
            }
        );
    }

    #[test]
    fn parser_builds_statements_with_source_lines() {
        let program = parse_program("# comment\nset name to \"UNX\"\nsay name\nsay \"hello\"").unwrap();
        assert_eq!(program.len(), 3);
        assert_eq!(program[0], Statement::SetString { name: "name".into(), value: "UNX".into(), line: 2, column: 1 });
        assert_eq!(program[1], Statement::SayVariable { name: "name".into(), line: 3, column: 1 });
        assert_eq!(program[2], Statement::SayString { value: "hello".into(), line: 4, column: 1 });
    }

    #[test]
    fn parses_if_equals_statement() {
        let program = parse_program("set name to \"UNX\"\nif name equals \"UNX\" say \"matched\"").unwrap();
        assert_eq!(program[1], Statement::IfEqualsSay {
            variable: "name".into(),
            expected: "UNX".into(),
            message: ConditionalMessage::String("matched".into()),
            line: 2,
            column: 1,
        });
        assert!(execute_program(&program, false).is_ok());
    }

    #[test]
    fn if_equals_rejects_undefined_condition_variable() {
        let program = parse_program("if missing equals \"UNX\" say \"matched\"").unwrap();
        assert!(execute_program(&program, false).unwrap_err().contains("undefined variable 'missing'"));
    }

    #[test]
    fn conditions_example_parses_and_validates() {
        let source = include_str!("../examples/conditions/main.unxc");
        assert!(validate_and_execute(source, false).is_ok());
    }

    #[test]
    fn multiline_if_runs_only_the_matching_branch() {
        let true_program = parse_program(
            "set name to \"UNX\"\nif name equals \"UNX\"\n  say missing\nelse\n  say \"not matched\"\nend"
        ).unwrap();
        assert!(execute_program(&true_program, false).unwrap_err().contains("undefined variable 'missing'"));

        let false_program = parse_program(
            "set name to \"other\"\nif name equals \"UNX\"\n  say missing\nelse\n  say \"matched else\"\nend"
        ).unwrap();
        assert!(execute_program(&false_program, false).is_ok());
    }

    #[test]
    fn multiline_if_supports_nested_blocks() {
        let program = parse_program(
            "set outer to \"yes\"\nset inner to \"yes\"\nif outer equals \"yes\"\nif inner equals \"yes\"\nsay missing\nend\nelse\nsay missing_outer\nend"
        ).unwrap();
        let error = execute_program(&program, false).unwrap_err();
        assert!(error.contains("undefined variable 'missing'"));
    }

    #[test]
    fn parser_rejects_unmatched_or_duplicate_block_markers() {
        assert!(parse_program("else").unwrap_err().contains("else' without a matching 'if"));
        assert!(parse_program("end").unwrap_err().contains("end' without a matching 'if"));
        assert!(parse_program("if name equals \"yes\"\nelse\nelse\nend").unwrap_err().contains("duplicate 'else'"));
        assert!(parse_program("if name equals \"yes\"\nsay \"open\"").unwrap_err().contains("missing 'end'"));
    }

    #[test]
    fn set_can_copy_an_existing_variable() {
        let program = parse_program("set original to \"UNX\"\nset copy to original\nsay copy").unwrap();
        assert_eq!(program[1], Statement::SetVariable {
            name: "copy".into(), source: "original".into(), line: 2, column: 1,
        });
        assert!(execute_program(&program, false).is_ok());
    }

    #[test]
    fn set_rejects_undefined_source_variable() {
        let program = parse_program("set copy to missing").unwrap();
        assert!(execute_program(&program, false).unwrap_err().contains("undefined variable 'missing'"));
    }

    #[test]
    fn parser_rejects_invalid_string_escape_with_line_number() {
        let error = parse_program("say \"ok\"\nset name to \"bad\\qescape\"").unwrap_err();
        assert!(error.contains("line 2, column"));
        assert!(error.contains("valid quoted string"));
    }

    #[test]
    fn reports_column_for_malformed_string_value() {
        let error = parse_program("set name to \"bad\\qescape\"").unwrap_err();
        assert!(error.contains("line 1, column 13"));
    }

    #[test]
    fn parser_accepts_tabs_and_extra_whitespace() {
        let source = "\tset   name\tto   \"UNX\"  \n  say\tname \n";
        let program = parse_program(source).unwrap();
        assert_eq!(program.len(), 2);
        assert_eq!(program[0], Statement::SetString {
            name: "name".into(), value: "UNX".into(), line: 1, column: 2,
        });
        assert_eq!(program[1], Statement::SayVariable {
            name: "name".into(), line: 2, column: 3,
        });
    }

    #[test]
    fn parser_rejects_reserved_words_as_variable_names() {
        for keyword in ["say", "set", "to"] {
            let source = format!("set {keyword} to \"value\"");
            let error = parse_program(&source).unwrap_err();
            assert!(error.contains("variable name"), "unexpected error for {keyword}: {error}");
        }
    }

    #[test]
    fn parser_keywords_are_case_sensitive() {
        let error = parse_program("Say \"hello\"").unwrap_err();
        assert!(error.contains("unknown statement"));
    }

    #[test]
    fn parser_rejects_unsupported_punctuation() {
        let punctuation = parse_program("say name!").unwrap_err();
        assert!(punctuation.contains("unexpected token"));
    }

    #[test]
    fn parser_accepts_inline_comments_but_preserves_hashes_in_strings() {
        let program = parse_program("say \"hello\" # trailing comment\nsay \"value # not a comment\"").unwrap();
        assert_eq!(program, vec![
            Statement::SayString { value: "hello".into(), line: 1, column: 1 },
            Statement::SayString { value: "value # not a comment".into(), line: 2, column: 1 },
        ]);
    }

    #[test]
    fn parser_ignores_blank_lines_and_full_line_comments() {
        let program = parse_program("\n  # comment\n\tsay \"ok\"  \n").unwrap();
        assert_eq!(program, vec![Statement::SayString {
            value: "ok".into(), line: 3, column: 2,
        }]);
    }

    #[test]
    fn parses_button_label_and_click_message() {
        let program = parse_program("open window \"Test\"\nbutton \"Click me\" say \"Button clicked!\"").unwrap();
        assert_eq!(program[1], Statement::Button {
            label: "Click me".into(), message: "Button clicked!".into(), line: 2, column: 1,
        });
    }

    #[test]
    fn button_can_display_a_variable_value() {
        let program = parse_program("set greeting to \"Hello from UNX!\"\nopen window \"Test\"\nbutton \"Greet\" say greeting").unwrap();
        assert_eq!(program[2], Statement::ButtonVariable {
            label: "Greet".into(), variable: "greeting".into(), line: 3, column: 1,
        });
        assert!(execute_program(&program, false).is_ok());
    }

    #[test]
    fn button_rejects_undefined_variable() {
        let program = parse_program("open window \"Test\"\nbutton \"Greet\" say missing").unwrap();
        assert!(execute_program(&program, false).unwrap_err().contains("undefined variable 'missing'"));
    }

    #[test]
    fn rejects_button_without_open_window() {
        let program = parse_program("button \"Click me\" say \"Clicked\"").unwrap();
        assert!(execute_program(&program, false).unwrap_err().contains("button requires an open window"));
    }

    #[test]
    fn parses_gui_window_title_and_dimensions() {
        let source = "open window \"UNX GUI Test\"\nset window width to \"800\"\nset window height to \"500\"\nsay \"Welcome!\"";
        let program = parse_program(source).unwrap();
        assert_eq!(program.len(), 4);
        assert_eq!(program[0], Statement::OpenWindow { title: "UNX GUI Test".into(), line: 1, column: 1 });
        assert_eq!(program[1], Statement::SetWindowWidth { value: "800".into(), line: 2, column: 1 });
        assert_eq!(program[2], Statement::SetWindowHeight { value: "500".into(), line: 3, column: 1 });
        assert_eq!(program[3], Statement::SayString { value: "Welcome!".into(), line: 4, column: 1 });
    }

    #[test]
    fn rejects_gui_statement_with_invalid_syntax() {
        assert!(parse_program("open window").unwrap_err().contains("expected 'open window"));
        assert!(parse_program("set window depth to \"500\"").unwrap_err().contains("expected 'width' or 'height'"));
    }

    #[test]
    fn accepts_basic_program() {
        let source = "set name to \"UNX\"\nsay \"Hello\"\nsay name\n";
        assert!(validate_and_execute(source, false).is_ok());
    }

    #[test]
    fn rejects_unknown_statement_with_line_number() {
        let error = validate_and_execute("say \"ok\"\nlaunch rocket", false).unwrap_err();
        assert!(error.contains("line 2"));
    }

    #[test]
    fn rejects_undefined_variable_with_column() {
        let error = validate_and_execute("  say missing", false).unwrap_err();
        assert!(error.contains("line 1, column 3"));
        assert!(error.contains("undefined variable"));
    }

    #[test]
    fn reports_column_for_unknown_statement_and_invalid_identifier() {
        let unknown = parse_program("   launch rocket").unwrap_err();
        assert!(unknown.contains("line 1, column 4"));
        let invalid_name = parse_program("set 9name to \"nope\"").unwrap_err();
        assert!(invalid_name.contains("line 1, column 5"));
    }

    #[test]
    fn accepts_nested_package_paths_and_dot_names() {
        assert!(is_safe_package_path("src/main.unx"));
        assert!(is_safe_package_path("assets/logo.png"));
        assert!(is_safe_package_path("release..notes/main.unx"));
    }

    #[test]
    fn rejects_unsafe_package_paths() {
        for path in ["", "../main.unx", "src/../main.unx", "/etc/passwd", "//server/share", "src\\\\main.unx", "C:/Windows/file", ".", "src//main.unx", "main\n.unx", "main\0.unx"] {
            assert!(!is_safe_package_path(path), "expected unsafe path to be rejected: {path}");
        }
    }

    fn unique_temp_dir(label: &str) -> std::path::PathBuf {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("unx-{label}-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn pack_then_load_package_round_trip() {
        let dir = unique_temp_dir("round-trip");
        let project = dir.join("project");
        fs::create_dir_all(&project).unwrap();
        fs::write(
            project.join(MANIFEST_NAME),
            r#"{"format":"UNX","format_version":1,"id":"org.example.test","name":"Test App","version":"0.1.0","entry":"main.unx","permissions":[]}"#,
        ).unwrap();
        fs::write(project.join("main.unx"), "set name to \"UNX\"\nsay name\n").unwrap();

        let package = dir.join("test.unx");
        command_pack(&project, &package).unwrap();
        let (manifest, source) = load_package(&package).unwrap();
        assert_eq!(manifest.id, "org.example.test");
        assert!(source.contains("say name"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn pack_rejects_oversized_manifest_before_parsing() {
        let dir = unique_temp_dir("oversized-manifest");
        let manifest_path = dir.join(MANIFEST_NAME);
        fs::write(&manifest_path, vec![b' '; MAX_MANIFEST_BYTES as usize + 1]).unwrap();

        let error = command_pack(&dir, &dir.join("oversized.unx")).unwrap_err();
        assert!(error.contains("manifest exceeds the 1 MiB size limit"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn pack_rejects_parent_traversal_entry() {
        let dir = unique_temp_dir("traversal");
        fs::write(
            dir.join(MANIFEST_NAME),
            r#"{"format":"UNX","format_version":1,"id":"org.example.test","name":"Test App","version":"0.1.0","entry":"../outside.unx","permissions":[]}"#,
        ).unwrap();
        let error = command_pack(&dir, &dir.join("bad.unx")).unwrap_err();
        assert!(error.contains("unsafe entry path"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn package_reader_rejects_zip_without_manifest() {
        let dir = unique_temp_dir("missing-manifest");
        let package = dir.join("bad.unx");
        let file = File::create(&package).unwrap();
        let mut archive = zip::ZipWriter::new(file);
        archive.start_file("main.unx", zip::write::SimpleFileOptions::default()).unwrap();
        archive.write_all(b"say \"hi\"").unwrap();
        archive.finish().unwrap();

        let error = load_package(&package).unwrap_err();
        assert!(error.contains("missing manifest.json"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn package_reader_rejects_unsafe_archive_entry_paths() {
        let dir = unique_temp_dir("unsafe-archive-path");
        let package = dir.join("unsafe.unx");
        let file = File::create(&package).unwrap();
        let mut archive = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        archive.start_file(MANIFEST_NAME, options).unwrap();
        archive.write_all(br#"{"format":"UNX","format_version":1,"id":"org.example.test","name":"Test","version":"0.1.0","entry":"main.unx","permissions":[]}"#).unwrap();
        archive.start_file("../main.unx", options).unwrap();
        archive.write_all(b"say \"hi\"").unwrap();
        archive.finish().unwrap();

        let error = load_package(&package).unwrap_err();
        assert!(error.contains("unsafe ZIP entry path"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn package_reader_rejects_oversized_manifest() {
        let dir = unique_temp_dir("oversized-archive-manifest");
        let package = dir.join("oversized-manifest.unx");
        let file = File::create(&package).unwrap();
        let mut archive = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        archive.start_file(MANIFEST_NAME, options).unwrap();
        let oversized_manifest = vec![b' '; MAX_MANIFEST_BYTES as usize + 1];
        archive.write_all(&oversized_manifest).unwrap();
        archive.start_file("main.unx", options).unwrap();
        archive.write_all(b"say \"hi\"").unwrap();
        archive.finish().unwrap();

        let error = load_package(&package).unwrap_err();
        assert!(error.contains("manifest exceeds the 1 MiB size limit"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn package_reader_rejects_oversized_uncompressed_source() {
        let dir = unique_temp_dir("oversized-source");
        let package = dir.join("oversized.unx");
        let file = File::create(&package).unwrap();
        let mut archive = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        archive.start_file(MANIFEST_NAME, options).unwrap();
        archive.write_all(br#"{"format":"UNX","format_version":1,"id":"org.example.test","name":"Test","version":"0.1.0","entry":"main.unx","permissions":[]}"#).unwrap();
        archive.start_file("main.unx", options).unwrap();
        let oversized_source = vec![b'a'; MAX_SOURCE_BYTES as usize + 1];
        archive.write_all(&oversized_source).unwrap();
        archive.finish().unwrap();

        let error = load_package(&package).unwrap_err();
        assert!(error.contains("entry source exceeds the 10 MiB size limit"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn rejects_manifest_with_empty_required_metadata() {
        let mut manifest = Manifest {
            format: "UNX".into(),
            format_version: 1,
            id: "org.example.app".into(),
            name: "Example".into(),
            version: "0.1.0".into(),
            entry: "main.unx".into(),
            permissions: vec![],
        };

        manifest.id = "  ".into();
        assert!(validate_manifest(&manifest).unwrap_err().contains("must not be empty"));

        manifest.id = "org.example.app".into();
        manifest.name = "".into();
        assert!(validate_manifest(&manifest).unwrap_err().contains("must not be empty"));

        manifest.name = "Example".into();
        manifest.version = "\t".into();
        assert!(validate_manifest(&manifest).unwrap_err().contains("must not be empty"));
    }

    #[test]
    fn package_reader_rejects_unexpected_archive_entries() {
        let dir = unique_temp_dir("unexpected-entry");
        let package = dir.join("extra.unx");
        let file = File::create(&package).unwrap();
        let mut archive = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        archive.start_file(MANIFEST_NAME, options).unwrap();
        archive.write_all(br#"{"format":"UNX","format_version":1,"id":"org.example.test","name":"Test","version":"0.1.0","entry":"main.unx","permissions":[]}"#).unwrap();
        archive.start_file("main.unx", options).unwrap();
        archive.write_all(b"say \"hi\"").unwrap();
        archive.start_file("extra.txt", options).unwrap();
        archive.write_all(b"unexpected").unwrap();
        archive.finish().unwrap();

        let error = load_package(&package).unwrap_err();
        assert!(error.contains("exactly manifest.json and one entry source file"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn manifest_cannot_use_metadata_filename_as_entry_source() {
        let manifest = Manifest {
            format: "UNX".into(),
            format_version: 1,
            id: "org.example.test".into(),
            name: "Test".into(),
            version: "0.1.0".into(),
            entry: MANIFEST_NAME.into(),
            permissions: vec![],
        };
        assert!(validate_manifest(&manifest).unwrap_err().contains("unsafe entry path"));
    }

    #[test]
    fn rejects_unsupported_manifest_format_and_version() {
        let mut manifest = Manifest {
            format: "OTHER".into(),
            format_version: 1,
            id: "org.example.app".into(),
            name: "Example".into(),
            version: "0.1.0".into(),
            entry: "main.unx".into(),
            permissions: vec![],
        };
        assert!(validate_manifest(&manifest).unwrap_err().contains("unsupported"));

        manifest.format = "UNX".into();
        manifest.format_version = 99;
        assert!(validate_manifest(&manifest).unwrap_err().contains("unsupported"));
    }

    #[test]
    fn validates_manifest_fields_and_version() {
        let manifest = Manifest {
            format: "UNX".into(),
            format_version: 1,
            id: "org.example.hello".into(),
            name: "Hello".into(),
            version: "0.1.0".into(),
            entry: "main.unx".into(),
            permissions: vec![],
        };
        assert!(validate_manifest(&manifest).is_ok());

        let mut invalid = manifest;
        invalid.entry = "../escape.unx".into();
        assert!(validate_manifest(&invalid).is_err());
    }

    #[test]
    fn package_paths_reject_ambiguous_or_platform_specific_forms() {
        for path in ["", ".", "./main.unx", "src//main.unx", "src/../main.unx", "../main.unx", "/main.unx", "C:/main.unx", "src\\\\main.unx", "main.unx\\n"] {
            assert!(!is_safe_package_path(path), "path should be rejected: {path:?}");
        }

        for path in ["main.unx", "src/main.unx", "my..app/main.unx", "main-file_v2.unx"] {
            assert!(is_safe_package_path(path), "path should be accepted: {path:?}");
        }
    }

    fn crc32(bytes: &[u8]) -> u32 {
        let mut crc = !0u32;
        for byte in bytes {
            crc ^= u32::from(*byte);
            for _ in 0..8 {
                crc = if crc & 1 == 1 {
                    (crc >> 1) ^ 0xedb8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }

    fn push_u16(output: &mut Vec<u8>, value: u16) {
        output.extend_from_slice(&value.to_le_bytes());
    }

    fn push_u32(output: &mut Vec<u8>, value: u32) {
        output.extend_from_slice(&value.to_le_bytes());
    }

    fn write_zip_with_duplicate_names(path: &Path) {
        // Emit a minimal stored ZIP directly: ZipWriter intentionally rejects
        // duplicate names, but hostile archives can contain them.
        let entries: [(&str, &[u8]); 2] = [
            ("manifest.json", b"{}"),
            ("manifest.json", b"{}"),
        ];
        let mut bytes = Vec::new();
        let mut central = Vec::new();

        for (name, contents) in entries {
            let name_bytes = name.as_bytes();
            let crc = crc32(contents);
            let offset = bytes.len() as u32;

            push_u32(&mut bytes, 0x0403_4b50);
            push_u16(&mut bytes, 20);
            push_u16(&mut bytes, 0);
            push_u16(&mut bytes, 0);
            push_u16(&mut bytes, 0);
            push_u16(&mut bytes, 0);
            push_u32(&mut bytes, crc);
            push_u32(&mut bytes, contents.len() as u32);
            push_u32(&mut bytes, contents.len() as u32);
            push_u16(&mut bytes, name_bytes.len() as u16);
            push_u16(&mut bytes, 0);
            bytes.extend_from_slice(name_bytes);
            bytes.extend_from_slice(contents);

            push_u32(&mut central, 0x0201_4b50);
            push_u16(&mut central, 20);
            push_u16(&mut central, 20);
            push_u16(&mut central, 0);
            push_u16(&mut central, 0);
            push_u16(&mut central, 0);
            push_u16(&mut central, 0);
            push_u32(&mut central, crc);
            push_u32(&mut central, contents.len() as u32);
            push_u32(&mut central, contents.len() as u32);
            push_u16(&mut central, name_bytes.len() as u16);
            push_u16(&mut central, 0);
            push_u16(&mut central, 0);
            push_u16(&mut central, 0);
            push_u16(&mut central, 0);
            push_u32(&mut central, 0);
            push_u32(&mut central, offset);
            central.extend_from_slice(name_bytes);
        }

        let central_offset = bytes.len() as u32;
        bytes.extend_from_slice(&central);
        let central_size = central.len() as u32;
        push_u32(&mut bytes, 0x0605_4b50);
        push_u16(&mut bytes, 0);
        push_u16(&mut bytes, 0);
        push_u16(&mut bytes, entries.len() as u16);
        push_u16(&mut bytes, entries.len() as u16);
        push_u32(&mut bytes, central_size);
        push_u32(&mut bytes, central_offset);
        push_u16(&mut bytes, 0);
        std::fs::write(path, bytes).expect("write handcrafted duplicate-entry ZIP");
    }

    #[test]
    fn package_reader_rejects_duplicate_zip_entry_names() {
        let dir = unique_temp_dir("duplicate-entry");
        let package = dir.join("duplicate.unx");
        write_zip_with_duplicate_names(&package);

        let error = load_package(&package).unwrap_err();
        assert!(error.contains("duplicate ZIP entry"), "unexpected error: {error}");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn package_reader_rejects_malformed_eocd_comment_length() {
        let dir = unique_temp_dir("bad-eocd-comment");
        let package = dir.join("bad-comment.unx");
        write_zip_with_duplicate_names(&package);

        let mut bytes = fs::read(&package).unwrap();
        let eocd = (0..=bytes.len() - 4)
            .rev()
            .find(|&offset| bytes.get(offset..offset + 4) == Some(b"PK\x05\x06"))
            .expect("EOCD in handcrafted ZIP");
        bytes[eocd + 20] = 1;
        bytes[eocd + 21] = 0;
        fs::write(&package, bytes).unwrap();

        let error = reject_duplicate_zip_entries(&package).unwrap_err();
        assert!(error.contains("missing or malformed end-of-central-directory"), "unexpected error: {error}");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn package_reader_rejects_multi_disk_zip_metadata() {
        let dir = unique_temp_dir("multi-disk");
        let package = dir.join("multi-disk.unx");
        write_zip_with_duplicate_names(&package);

        let mut bytes = fs::read(&package).unwrap();
        let eocd = (0..=bytes.len() - 4)
            .rev()
            .find(|&offset| bytes.get(offset..offset + 4) == Some(b"PK\x05\x06"))
            .expect("EOCD in handcrafted ZIP");
        bytes[eocd + 4] = 1;
        bytes[eocd + 5] = 0;
        fs::write(&package, bytes).unwrap();

        let error = reject_duplicate_zip_entries(&package).unwrap_err();
        assert!(error.contains("multi-disk ZIP packages are not supported"), "unexpected error: {error}");
        fs::remove_dir_all(&dir).unwrap();
    }

}
