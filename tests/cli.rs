use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

type ArchiveFixture<'a> = (&'a str, Vec<(&'a str, &'a [u8])>, &'a str);

const MANIFEST: &str = r#"{"format":"UNX","format_version":1,"id":"org.example.integration","name":"Integration App","version":"0.1.0","entry":"main.unx","permissions":[]}"#;

fn temp_dir(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after UNIX epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "unx-cli-{label}-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("create temporary directory");
    path
}

fn unx(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_unx"))
        .args(args)
        .output()
        .expect("run UNX CLI")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("CLI output should be UTF-8")
}

fn write_project(project: &Path, source: &str) {
    fs::create_dir_all(project).expect("create project directory");
    fs::write(project.join("manifest.json"), MANIFEST).expect("write manifest");
    fs::write(project.join("main.unx"), source).expect("write source");
}

#[test]
fn check_accepts_valid_source_and_rejects_invalid_source() {
    let dir = temp_dir("check");
    let valid = dir.join("valid.unx");
    let invalid = dir.join("invalid.unx");
    fs::write(&valid, "say \"valid\"\n").unwrap();
    fs::write(&invalid, "say missing\n").unwrap();

    let valid_result = unx(&["check", valid.to_str().unwrap()]);
    assert!(valid_result.status.success(), "{}", text(&valid_result.stderr));
    assert!(text(&valid_result.stdout).contains("OK: source is valid"));

    let invalid_result = unx(&["check", invalid.to_str().unwrap()]);
    assert!(!invalid_result.status.success());
    assert!(text(&invalid_result.stderr).contains("undefined variable"));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn run_executes_source_and_prints_output() {
    let dir = temp_dir("run");
    let source = dir.join("hello.unx");
    fs::write(&source, "set name to \"UNX\"\nsay \"Hello\"\nsay name\n").unwrap();

    let result = unx(&["run", source.to_str().unwrap()]);
    assert!(result.status.success(), "{}", text(&result.stderr));
    assert_eq!(text(&result.stdout), "Hello\nUNX\n");

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn pack_inspect_and_run_work_end_to_end() {
    let dir = temp_dir("package-flow");
    let project = dir.join("project");
    let package = dir.join("app.unx");
    write_project(&project, "say \"PACKAGE_OUTPUT\"\n");

    let pack = unx(&[
        "pack",
        project.to_str().unwrap(),
        package.to_str().unwrap(),
    ]);
    assert!(pack.status.success(), "{}", text(&pack.stderr));
    assert!(package.is_file());

    let inspect = unx(&["inspect", package.to_str().unwrap()]);
    assert!(inspect.status.success(), "{}", text(&inspect.stderr));
    let inspected = text(&inspect.stdout);
    assert!(inspected.contains("\\\"id\\\": \\\"org.example.integration\\\""));
    assert!(inspected.contains("--- Entry source: main.unx ---"));
    assert!(inspected.contains("say \"PACKAGE_OUTPUT\""));
    assert!(!text(&inspect.stderr).contains("Running Integration App"));

    let run = unx(&["run", package.to_str().unwrap()]);
    assert!(run.status.success(), "{}", text(&run.stderr));
    assert_eq!(text(&run.stdout), "PACKAGE_OUTPUT\n");
    assert!(text(&run.stderr).contains("Running Integration App"));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn packing_identical_project_twice_produces_identical_bytes() {
    let dir = temp_dir("deterministic-pack");
    let project = dir.join("project");
    let first = dir.join("first.unx");
    let second = dir.join("second.unx");
    write_project(&project, "say \"REPRODUCIBLE\"\n");

    for output in [&first, &second] {
        let result = unx(&[
            "pack",
            project.to_str().unwrap(),
            output.to_str().unwrap(),
        ]);
        assert!(result.status.success(), "{}", text(&result.stderr));
    }

    assert_eq!(
        fs::read(&first).unwrap(),
        fs::read(&second).unwrap(),
        "packing the same project twice should produce byte-identical packages"
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn pack_rejects_invalid_source_without_creating_package() {
    let dir = temp_dir("invalid-pack");
    let project = dir.join("project");
    let package = dir.join("bad.unx");
    write_project(&project, "say missing\n");

    let result = unx(&[
        "pack",
        project.to_str().unwrap(),
        package.to_str().unwrap(),
    ]);
    assert!(!result.status.success());
    assert!(text(&result.stderr).contains("undefined variable"));
    assert!(!package.exists());

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn help_and_version_are_available() {
    let help = unx(&["--help"]);
    assert!(help.status.success(), "{}", text(&help.stderr));
    assert!(text(&help.stderr).contains("Usage:"));

    let version = unx(&["--version"]);
    assert!(version.status.success(), "{}", text(&version.stderr));
    assert!(text(&version.stdout).contains("unx 0.1.0"));
}

#[test]
fn invalid_cli_invocations_return_usage_error_status() {
    for args in [
        &[][..],
        &["unknown-command"][..],
        &["run"][..],
        &["--help", "extra"][..],
    ] {
        let result = unx(args);
        assert_eq!(result.status.code(), Some(2), "args: {args:?}");
        assert!(text(&result.stderr).contains("Usage:"), "args: {args:?}");
    }
}

#[test]
fn pack_rejects_parent_traversal_entry_paths() {
    let dir = temp_dir("traversal");
    let project = dir.join("project");
    fs::create_dir_all(&project).unwrap();
    let manifest = r#"{"format":"UNX","format_version":1,"id":"org.example.bad","name":"Bad App","version":"0.1.0","entry":"../outside.unx","permissions":[]}"#;
    fs::write(project.join("manifest.json"), manifest).unwrap();
    let package = dir.join("bad.unx");

    let result = unx(&[
        "pack",
        project.to_str().unwrap(),
        package.to_str().unwrap(),
    ]);
    assert!(!result.status.success());
    assert!(text(&result.stderr).contains("unsafe entry path"));
    assert!(!package.exists());

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn inspect_rejects_non_zip_input_without_panicking() {
    let dir = temp_dir("malformed-zip");
    let package = dir.join("broken.unx");
    fs::write(&package, b"not a zip archive").unwrap();

    let result = unx(&["inspect", package.to_str().unwrap()]);
    assert!(!result.status.success());
    assert!(text(&result.stderr).contains("invalid ZIP package"));

    fs::remove_dir_all(dir).unwrap();
}

fn write_archive(path: &Path, entries: &[(&str, &[u8])]) {
    let file = fs::File::create(path).expect("create test ZIP");
    let mut archive = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (name, contents) in entries {
        archive.start_file(*name, options).expect("start ZIP entry");
        use std::io::Write;
        archive.write_all(contents).expect("write ZIP entry");
    }
    archive.finish().expect("finish test ZIP");
}

#[test]
fn inspect_rejects_invalid_package_fixtures() {
    let dir = temp_dir("invalid-fixtures");
    let valid_manifest = br#"{"format":"UNX","format_version":1,"id":"org.example.test","name":"Test App","version":"0.1.0","entry":"main.unx","permissions":[]}"#;
    let unsupported_manifest = br#"{"format":"UNX","format_version":99,"id":"org.example.test","name":"Test App","version":"0.1.0","entry":"main.unx","permissions":[]}"#;
    let invalid_json = b"{ definitely not JSON";
    let source = b"say \"safe\"\n";
    let invalid_utf8 = [0xff, 0xfe, 0xfd];

    let fixtures: Vec<ArchiveFixture<'_>> = vec![
        ("missing-manifest", vec![("main.unx", source)], "missing manifest.json"),
        ("invalid-json", vec![("manifest.json", invalid_json), ("main.unx", source)], "invalid manifest"),
        ("unsupported-version", vec![("manifest.json", unsupported_manifest), ("main.unx", source)], "unsupported UNX package format/version"),
        ("missing-entry", vec![("manifest.json", valid_manifest), ("other.unx", source)], "missing entry file"),
        ("invalid-utf8", vec![("manifest.json", valid_manifest), ("main.unx", &invalid_utf8)], "UTF-8"),
        ("unsafe-zip-path", vec![("manifest.json", valid_manifest), ("main.unx", source), ("../outside.unx", source)], "unsafe ZIP entry path"),
    ];

    for (label, entries, expected_error) in fixtures {
        let package = dir.join(format!("{label}.unx"));
        write_archive(&package, &entries);
        let result = unx(&["inspect", package.to_str().unwrap()]);
        assert!(
            !result.status.success(),
            "fixture {label} should fail validation"
        );
        let stderr = text(&result.stderr);
        assert!(
            stderr
                .to_lowercase()
                .contains(&expected_error.to_lowercase()),
            "fixture {label} expected error containing {expected_error:?}, got: {stderr}"
        );
    }

    fs::remove_dir_all(dir).unwrap();
}
