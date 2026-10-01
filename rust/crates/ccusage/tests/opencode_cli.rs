use std::process::{Command, Output};

use ccusage_test_support::{Fixture, fs_fixture};
use serde_json::Value;

fn fixture() -> Fixture {
    fs_fixture!({
        "opencode/storage/message/ses_target/msg-a.json": r#"{"id":"msg-a","sessionID":"ses_target","providerID":"openai","modelID":"gpt-5","time":{"created":1767312000000},"tokens":{"input":100,"output":50,"cache":{"read":10,"write":5}},"cost":0.25}"#,
        "opencode/storage/message/ses_other/msg-b.json": r#"{"id":"msg-b","sessionID":"ses_other","providerID":"openai","modelID":"gpt-5","time":{"created":1767312000000},"tokens":{"input":900,"output":100},"cost":1}"#,
    })
}

fn run(fixture: &Fixture, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ccusage"))
        .args([
            "opencode",
            "session",
            "--offline",
            "--mode",
            "display",
            "--no-color",
            "--timezone",
            "UTC",
        ])
        .args(args)
        .env("OPENCODE_DATA_DIR", fixture.path("opencode"))
        .env("HOME", fixture.path("empty-home"))
        .env("XDG_CONFIG_HOME", fixture.path("empty-config"))
        .env("LOG_LEVEL", "0")
        .env("COLUMNS", "240")
        .output()
        .expect("ccusage CLI should run")
}

#[test]
fn selected_opencode_session_json_equals_full_scan_row() {
    let fixture = fixture();
    let full = run(&fixture, &["--json"]);
    let selected = run(&fixture, &["--json", "--id", "ses_target"]);
    assert!(full.status.success());
    assert!(
        selected.status.success(),
        "{}",
        String::from_utf8_lossy(&selected.stderr)
    );
    let full: Value = serde_json::from_slice(&full.stdout).unwrap();
    let selected: Value = serde_json::from_slice(&selected.stdout).unwrap();
    let row = full["sessions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["sessionId"] == "ses_target")
        .unwrap();
    assert_eq!(&selected, row);
    assert_eq!(selected["inputTokens"], 100);
    assert_eq!(selected["totalCost"], 0.25);
    assert!(selected.get("sessions").is_none());
    assert!(selected.get("totals").is_none());
}

#[test]
fn selected_opencode_session_table_excludes_other_sessions() {
    let fixture = fixture();
    let output = run(&fixture, &["-i", "ses_target"]);
    assert!(output.status.success());
    let table = String::from_utf8(output.stdout).unwrap();
    assert!(table.contains("ses_target"));
    assert!(!table.contains("ses_other"));
}

#[test]
fn missing_opencode_session_is_an_error_even_in_json_mode() {
    let fixture = fixture();
    for args in [vec!["--id", "missing"], vec!["--id", "missing", "--json"]] {
        let output = run(&fixture, &args);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(
            String::from_utf8(output.stderr)
                .unwrap()
                .contains("No OpenCode session found with ID: missing")
        );
    }
}

#[test]
fn selected_opencode_session_supports_no_cost() {
    let fixture = fixture();
    let output = run(&fixture, &["--id", "ses_target", "--json", "--no-cost"]);
    assert!(output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(value.get("totalCost").is_none());
}

#[test]
fn selected_opencode_session_rejects_path_traversal() {
    let fixture = fixture();
    let output = run(&fixture, &["--id", "../ses_other", "--json"]);
    assert!(!output.status.success());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("Invalid OpenCode session ID")
    );
}
