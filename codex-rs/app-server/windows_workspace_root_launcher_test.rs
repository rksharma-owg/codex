use std::path::Path;
use std::process::Command;

#[test]
fn launcher_executes_the_test_binary_after_exporting_runfile_resources() {
    let launcher = codex_utils_cargo_bin::resolve_bazel_runfile(
        Some("codex-rs/app-server"),
        Path::new("app-server-unit-tests.bat"),
    )
    .expect("resolve the generated Windows test launcher");
    let output = Command::new(launcher)
        .arg("--list")
        .output()
        .expect("invoke the generated Windows test launcher");
    assert!(
        output.status.success(),
        "launcher failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .any(|line| line.ends_with(": test")),
        "the launcher must execute the real Rust test binary",
    );
}
