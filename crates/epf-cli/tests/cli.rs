use std::{io::Write, process::{Command, Stdio}};

fn epf() -> Command { Command::new(env!("CARGO_BIN_EXE_epf")) }

#[test]
fn rejects_invalid_ports_and_requires_noninteractive_consent() {
    for port in ["0", "65536", "abc", "-1"] {
        let output = epf().args(["open", port, "--yes"]).output().unwrap();
        assert!(!output.status.success());
    }
    let output = epf().args(["open", "25565"]).stdin(Stdio::null()).output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("Confirm this operation"));
}

#[test]
fn profiles_round_trip_without_network_access() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    let output = epf().arg("--config").arg(&path).args(["--json", "profiles", "save", "25565", "--protocol", "both", "--name", "My game"]).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let profile: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let id = profile["id"].as_str().unwrap();
    let output = epf().arg("--config").arg(&path).args(["--json", "list"]).output().unwrap();
    let list: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(list["rules"].as_array().unwrap().is_empty());
    let output = epf().arg("--config").arg(&path).args(["--json", "profiles", "list"]).output().unwrap();
    let list: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(list["profiles"][0]["request"]["protocol"], "both");
    assert!(epf().arg("--config").arg(&path).args(["profiles", "delete", id]).output().unwrap().status.success());
}

#[test]
fn corrupted_config_is_not_overwritten() {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(b"{broken").unwrap();
    let output = epf().arg("--config").arg(file.path()).args(["--json", "profiles", "save", "80"]).output().unwrap();
    assert!(!output.status.success());
    assert_eq!(std::fs::read(file.path()).unwrap(), b"{broken");
    let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert!(error["error"].as_str().unwrap().contains("kept for recovery"));
}
