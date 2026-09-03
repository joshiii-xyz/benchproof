use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_benchproof"))
}

fn temp_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!("benchproof-cli-{label}-{}", std::process::id()))
}

#[test]
fn version_and_help_are_available() {
    let version = Command::new(binary()).arg("--version").output().unwrap();
    assert!(version.status.success());
    assert!(String::from_utf8_lossy(&version.stdout).contains("benchproof"));
    let help = Command::new(binary()).arg("--help").output().unwrap();
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("compare"));
}

#[test]
fn run_records_warmup_and_repetitions() {
    let output = Command::new(binary())
        .args([
            "run",
            "--cache-policy",
            "warm",
            "--warmup",
            "1",
            "--repetitions",
            "2",
            "--",
            "/bin/sh",
            "-c",
            "printf fixture",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["policy"]["cache_policy"], "warm");
    assert_eq!(report["warmup_samples"].as_array().unwrap().len(), 1);
    assert_eq!(report["samples"].as_array().unwrap().len(), 2);
    assert_eq!(report["summary"]["successful"], 2);
}

#[test]
fn failed_command_returns_one() {
    let output = Command::new(binary())
        .args(["run", "--repetitions", "1", "--", "/bin/sh", "-c", "exit 4"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["samples"][0]["outcome"], "nonzero");
}

#[test]
fn inspect_and_compare_saved_reports() {
    let path = temp_path("run.json");
    let output = Command::new(binary())
        .args(["run", "--repetitions", "1", "--", "/bin/sh", "-c", "true"])
        .output()
        .unwrap();
    assert!(output.status.success());
    fs::write(&path, &output.stdout).unwrap();

    let inspect = Command::new(binary())
        .args(["inspect", path.to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    assert!(inspect.status.success());
    let inspected: Value = serde_json::from_slice(&inspect.stdout).unwrap();
    assert_eq!(inspected["schema_version"], 1);

    let compare = Command::new(binary())
        .args([
            "compare",
            path.to_str().unwrap(),
            path.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert!(compare.status.success());
    let compared: Value = serde_json::from_slice(&compare.stdout).unwrap();
    assert_eq!(compared["comparable"], true);
    fs::remove_file(path).unwrap();
}
