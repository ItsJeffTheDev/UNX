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
        "UNX prototype 0.1\nUsage:\n  unx run <source.unx|package.unx>\n  unx check <source.unx>\n  unx pack <project-directory> <output.unx>\n  unx inspect <package.unx>\n\nPrototype statements: say \"Hello\", set name to \"UNX\", say name, # comment"
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
    SayString { value: String, line: usize },
    SayVariable { name: String, line: usize },
    SetString { name: String, value: String, line: usize },
}

fn parse_program(source: &str) -> Result<Vec<Statement>, String> {
    let mut statements = Vec::new();
    for (index, raw) in source.lines().enumerate() {
        let line_no = index + 1;
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(rest) = line.strip_prefix("say ") {
            if let Some(value) = parse_quoted(rest) {
                statements.push(Statement::SayString { value, line: line_no });
            } else {
                let name = rest.trim();
                if name.is_empty() || !name.chars().enumerate().all(|(i, c)| {
                    c == '_' || (c.is_ascii_alphanumeric() && (i > 0 || !c.is_ascii_digit()))
                }) {
                    return Err(format!("line {line_no}: expected a quoted string or variable name after 'say'"));
                }
                statements.push(Statement::SayVariable { name: name.to_string(), line: line_no });
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix("set ") {
            let (name, value_expr) = rest.split_once(" to ")
                .ok_or_else(|| format!("line {line_no}: expected 'set NAME to VALUE'"))?;
            let name = name.trim();
            if name.is_empty() || !name.chars().enumerate().all(|(i, c)| {
                c == '_' || (c.is_ascii_alphanumeric() && (i > 0 || !c.is_ascii_digit()))
            }) {
                return Err(format!("line {line_no}: invalid variable name '{name}'"));
            }
            let value = parse_quoted(value_expr)
                .ok_or_else(|| format!("line {line_no}: expected a valid quoted string after 'to'"))?;
            statements.push(Statement::SetString { name: name.to_string(), value, line: line_no });
            continue;
        }

        return Err(format!("line {line_no}: unknown statement"));
    }
    Ok(statements)
}

fn execute_program(statements: &[Statement], print_output: bool) -> Result<(), String> {
    let mut vars: HashMap<String, String> = HashMap::new();
    for statement in statements {
        match statement {
            Statement::SayString { value, .. } => {
                if print_output { println!("{value}"); }
            }
            Statement::SayVariable { name, line } => {
                let value = vars.get(name)
                    .ok_or_else(|| format!("line {line}: undefined variable '{name}'"))?;
                if print_output { println!("{value}"); }
            }
            Statement::SetString { name, value, .. } => {
                vars.insert(name.clone(), value.clone());
            }
        }
    }
    Ok(())
}

fn validate_and_execute(source: &str, print_output: bool) -> Result<(), String> {
    let program = parse_program(source)?;
    execute_program(&program, print_output)
}

fn is_safe_package_path(value: &str) -> bool {
    if value.is_empty() || value.starts_with('/') || value.contains(':') || value.bytes().any(|b| b == 92) {
        return false;
    }
    value.split('/').all(|part| !part.is_empty() && part != "." && part != "..")
}

fn validate_manifest(manifest: &Manifest) -> Result<(), String> {
    if manifest.format != "UNX" || manifest.format_version != 1 {
        return Err("unsupported UNX package format/version".into());
    }
    if !is_safe_package_path(&manifest.entry) {
        return Err("unsafe entry path in manifest".into());
    }
    if manifest.id.trim().is_empty() || manifest.name.trim().is_empty() || manifest.version.trim().is_empty() {
        return Err("manifest id, name, and version must not be empty".into());
    }
    Ok(())
}

fn load_package(path: &Path) -> Result<(Manifest, String), String> {
    let file = File::open(path).map_err(|e| format!("cannot open package: {e}"))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("invalid ZIP package: {e}"))?;
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
    fn rejects_undefined_variable() {
        let error = validate_and_execute("say missing", false).unwrap_err();
        assert!(error.contains("line 1"));
    }

    #[test]
    fn accepts_nested_package_paths_and_dot_names() {
        assert!(is_safe_package_path("src/main.unx"));
        assert!(is_safe_package_path("assets/logo.png"));
        assert!(is_safe_package_path("release..notes/main.unx"));
    }

    #[test]
    fn rejects_unsafe_package_paths() {
        for path in ["", "../main.unx", "src/../main.unx", "/etc/passwd", "//server/share", "src\\\\main.unx", "C:/Windows/file", ".", "src//main.unx"] {
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
