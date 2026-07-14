use std::process::Command;

#[test]
fn reports_the_workspace_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_lyra-effects"))
        .arg("--version")
        .output()
        .expect("run lyra-effects");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).expect("utf-8 stdout"),
        format!("lyra-effects {}\n", env!("CARGO_PKG_VERSION"))
    );
}
