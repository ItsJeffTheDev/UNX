use font8x8::{UnicodeFonts, BASIC_FONTS};
use minifb::{Key, Window, WindowOptions};
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
        "UNX prototype 0.1\nUsage:\n  unx run <source.unx|package.unx>\n  unx check <source.unx>\n  unx pack <project-directory> <output.unx>\n  unx inspect <package.unx>\n\nPrototype statements: say \"Hello\", set name to \"UNX\", say name, open window \"Title\", set window width/height to \"800\", # comment"
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
    OpenWindow { title: String, line: usize, column: usize },
    SetWindowWidth { value: String, line: usize, column: usize },
    SetWindowHeight { value: String, line: usize, column: usize },
}

#[derive(Debug, PartialEq)]
enum TokenKind {
    Say,
    Set,
    To,
    Open,
    Window,
    Width,
    Height,
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
        let token_start = offset;
        while offset < line.len() {
            let ch = line[offset..].chars().next().expect("offset is within line");
            if ch == ' ' || ch == '\t' { break; }
            offset += ch.len_utf8();
        }
        let word = &line[token_start..offset];
        let kind = match word {
            "say" => TokenKind::Say,
            "set" => TokenKind::Set,
            "to" => TokenKind::To,
            "open" => TokenKind::Open,
            "window" => TokenKind::Window,
            "width" => TokenKind::Width,
            "height" => TokenKind::Height,
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

fn parse_program(source: &str) -> Result<Vec<Statement>, String> {
    let mut statements = Vec::new();
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
                if tokens.len() != 4 {
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
                let value = match &tokens[3].kind {
                    TokenKind::String(value) => value,
                    _ => return Err(format!("line {line_no}, column {}: expected a quoted string after 'to'", tokens[3].column)),
                };
                statements.push(Statement::SetString {
                    name: name.clone(), value: value.clone(), line: line_no, column: first.column,
                });
            }
            _ => return Err(format!("line {line_no}, column {}: unknown statement", first.column)),
        }
    }
    Ok(statements)
}

fn execute_program(statements: &[Statement], print_output: bool) -> Result<(), String> {
    let mut vars: HashMap<String, String> = HashMap::new();
    let mut window_title: Option<String> = None;
    let mut window_width: usize = 800;
    let mut window_height: usize = 500;
    let mut window_lines: Vec<String> = Vec::new();

    for statement in statements {
        match statement {
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
            Statement::OpenWindow { title, .. } => window_title = Some(title.clone()),
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
            show_gui(&title, window_width, window_height, &window_lines)?;
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

fn show_gui(title: &str, width: usize, height: usize, lines: &[String]) -> Result<(), String> {
    let mut window = Window::new(title, width, height, WindowOptions::default())
        .map_err(|e| format!("could not create GUI window: {e}"))?;
    let mut buffer = vec![0x00F4F7FB; width * height];

    draw_text(&mut buffer, width, height, 28, 28, title, 0x001B2A41);
    let mut y = 72usize;
    for line in lines.iter().take(height.saturating_sub(90) / 28) {
        draw_text(&mut buffer, width, height, 28, y, line, 0x002B3748);
        y += 28;
    }
    draw_text(&mut buffer, width, height, 28, height.saturating_sub(32), "Press ESC to close", 0x006B7280);

    while window.is_open() && !window.is_key_down(Key::Escape) {
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

fn load_package(path: &Path) -> Result<(Manifest, String), String> {
    let metadata = fs::metadata(path).map_err(|e| format!("cannot inspect package: {e}"))?;
    if metadata.len() > MAX_PACKAGE_BYTES {
        return Err("package exceeds the 20 MiB size limit".into());
    }
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
    let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
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
        _ => { usage(); return; }
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
    fn parser_builds_statements_with_source_lines() {
        let program = parse_program("# comment\nset name to \"UNX\"\nsay name\nsay \"hello\"").unwrap();
        assert_eq!(program.len(), 3);
        assert_eq!(program[0], Statement::SetString { name: "name".into(), value: "UNX".into(), line: 2, column: 1 });
        assert_eq!(program[1], Statement::SayVariable { name: "name".into(), line: 3, column: 1 });
        assert_eq!(program[2], Statement::SayString { value: "hello".into(), line: 4, column: 1 });
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
    fn parser_rejects_inline_comments_and_punctuation() {
        let inline_comment = parse_program("say \"hello\" # not supported").unwrap_err();
        assert!(inline_comment.contains("unexpected token"));

        let punctuation = parse_program("say name!").unwrap_err();
        assert!(punctuation.contains("unexpected token"));
    }

    #[test]
    fn parser_ignores_blank_lines_and_full_line_comments() {
        let program = parse_program("\n  # comment\n\tsay \"ok\"  \n").unwrap();
        assert_eq!(program, vec![Statement::SayString {
            value: "ok".into(), line: 3, column: 2,
        }]);
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
}
