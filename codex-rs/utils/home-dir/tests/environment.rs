//! Exercise environment decoding in a subprocess so parallel tests keep their own environment.

use std::ffi::OsString;
use std::process::Command;

#[test]
fn resolve_home_in_subprocess() {
    let Some(mode) = std::env::var_os("CODEX_HOME_TEST_MODE") else {
        return;
    };
    if mode == "invalid" {
        let error = codex_utils_home_dir::find_codex_home().expect_err("invalid CODEX_HOME");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        assert!(error.to_string().contains("CODEX_HOME"));
    } else {
        let expected = std::env::var_os("CODEX_HOME").expect("test home");
        assert_eq!(
            codex_utils_home_dir::find_codex_home()
                .expect("valid CODEX_HOME")
                .as_path(),
            std::fs::canonicalize(expected).expect("canonical test home")
        );
    }
}

fn check_home(value: OsString, mode: &str) -> std::io::Result<()> {
    let output = Command::new(std::env::current_exe()?)
        .args(["--exact", "resolve_home_in_subprocess", "--nocapture"])
        .env("CODEX_HOME", value)
        .env("CODEX_HOME_TEST_MODE", mode)
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn valid_home_is_honored() {
    let home = tempfile::tempdir().expect("temporary home");
    check_home(home.path().as_os_str().to_owned(), "valid").expect("run resolver subprocess");
}

#[cfg(unix)]
#[test]
fn non_unicode_home_is_rejected() {
    use std::os::unix::ffi::OsStringExt;
    check_home(
        OsString::from_vec(b"/tmp/codex-home-\xff".to_vec()),
        "invalid",
    )
    .expect("run resolver subprocess");
}

#[cfg(windows)]
#[test]
fn non_unicode_home_is_rejected() {
    use std::os::windows::ffi::OsStringExt;
    check_home(OsString::from_wide(&[0xD800]), "invalid").expect("run resolver subprocess");
}
