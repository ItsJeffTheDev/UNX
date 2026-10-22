use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

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
    assert!(inspected.contains("\"id\": \"org.example.integration\""));
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
