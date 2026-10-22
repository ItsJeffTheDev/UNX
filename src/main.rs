use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::{Read, Write},
    path::Path,
};

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
    if s.len() < 2 || !s.starts_with('"') || !s.ends_with('"') { return None; }
    let inner = &s[1..s.len() - 1];
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next()? {
                'n' => out.push('\n'), 't' => out.push('\t'),
                '"' => out.push('"'), '\\' => out.push('\\'),
                other => return Some(format!("\\{other}")),
            }
        } else { out.push(c); }
    }
    Some(out)
}

fn validate_and_execute(source: &str, print_output: bool) -> Result<(), String> {
    let mut vars: HashMap<String, String> = HashMap::new();
    for (index, raw) in source.lines().enumerate() {
        let line_no = index + 1;
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        if let Some(rest) = line.strip_prefix("say ") {
            let value = if let Some(text) = parse_quoted(rest) { text }
                else if let Some(value) = vars.get(rest.trim()) { value.clone() }
                else { return Err(format!("line {line_no}: expected a quoted string or a defined variable after 'say'")); };
            if print_output { println!("{value}"); }
            continue;
        }
        if let Some(rest) = line.strip_prefix("set ") {
            let (name, value_expr) = rest.split_once(" to ")
                .ok_or_else(|| format!("line {line_no}: expected 'set NAME to VALUE'"))?;
            let name = name.trim();
            if name.is_empty() || !name.chars().enumerate().all(|(i,c)| c == '_' || (c.is_ascii_alphanumeric() && (i > 0 || !c.is_ascii_digit()))) {
                return Err(format!("line {line_no}: invalid variable name '{name}'"));
            }
            let value = parse_quoted(value_expr)
                .ok_or_else(|| format!("line {line_no}: expected a quoted string after 'to'"))?;
            vars.insert(name.to_string(), value);
            continue;
        }
        return Err(format!("line {line_no}: unknown statement"));
    }
    Ok(())
}

fn load_package(path: &Path) -> Result<(Manifest, String), String> {
    let file = File::open(path).map_err(|e| format!("cannot open package: {e}"))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("invalid ZIP package: {e}"))?;
    let manifest: Manifest = {
        let mut entry = archive.by_name(MANIFEST_NAME).map_err(|_| "package is missing manifest.json".to_string())?;
        let mut text = String::new();
        entry.read_to_string(&mut text).map_err(|e| e.to_string())?;
        serde_json::from_str(&text).map_err(|e| format!("invalid manifest: {e}"))?
    };
    if manifest.format != "UNX" || manifest.format_version != 1 {
        return Err("unsupported UNX package format/version".into());
    }
    if manifest.entry.starts_with('/') || manifest.entry.contains("..") || manifest.entry.contains('\\') {
        return Err("unsafe entry path in manifest".into());
    }
    let mut entry = archive.by_name(&manifest.entry)
        .map_err(|_| format!("package is missing entry file '{}'", manifest.entry))?;
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
    if manifest.format != "UNX" || manifest.format_version != 1 { return Err("manifest must declare format='UNX' and format_version=1".into()); }
    if manifest.entry.starts_with('/') || manifest.entry.contains("..") || manifest.entry.contains('\\') { return Err("unsafe entry path in manifest".into()); }
    let source_path = project.join(&manifest.entry);
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
}
