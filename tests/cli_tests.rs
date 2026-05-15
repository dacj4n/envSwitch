use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

fn binary() -> PathBuf {
    std::env::current_dir()
        .unwrap()
        .join("target")
        .join("debug")
        .join("envswitch")
}

fn test_home(name: &str) -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("ev_cli_{}_{}", name, n));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(args: &[&str], home: &PathBuf) -> std::process::Output {
    Command::new(binary())
        .args(args)
        .env("ENVSWITCH_HOME", home)
        .output()
        .expect("Failed to execute envswitch")
}

fn ok(args: &[&str], home: &PathBuf) -> String {
    let output = run(args, home);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !output.status.success() {
        panic!(
            "Command failed (exit {:?})\nargs: {:?}\nstdout: {}\nstderr: {}",
            output.status.code(),
            args,
            stdout,
            stderr
        );
    }
    stdout
}

fn err(args: &[&str], home: &PathBuf) -> String {
    let output = run(args, home);
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(!output.status.success(), "Expected error but got success");
    stderr
}

// ── Tests ───────────────────────────────────────────────────────────

#[test]
fn test_help() {
    let home = test_home("help");
    let out = ok(&["--help"], &home);
    assert!(out.contains("envSwitch"));
    assert!(out.contains("list"));
    assert!(out.contains("install"));
    assert!(out.contains("cover"));
    assert!(out.contains("uncover"));
    assert!(out.contains("start"));
    assert!(out.contains("stop"));
    assert!(out.contains("init"));
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn test_version() {
    let home = test_home("ver");
    let out = ok(&["--version"], &home);
    assert!(out.contains("envswitch"));
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn test_list_empty() {
    let home = test_home("list_empty");
    let out = ok(&["list"], &home);
    assert!(out.contains("jdk"));
    assert!(out.contains("go"));
    assert!(out.contains("mysql"));
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn test_list_unknown_module() {
    let home = test_home("list_unknown");
    let e = err(&["list", "foobar"], &home);
    assert!(e.contains("Unknown module"), "Got: {}", e);
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn test_status_empty() {
    let home = test_home("status_empty");
    let out = ok(&["status"], &home);
    assert!(out.contains("No active covers"));
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn test_cover_uninstalled() {
    let home = test_home("cover_uninstalled");
    let e = err(&["cover", "jdk", "99"], &home);
    assert!(e.contains("not installed"), "Got: {}", e);
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn test_init_project() {
    let home = test_home("init_proj");
    let project_dir = home.join("myproject");
    fs::create_dir_all(&project_dir).unwrap();

    let output = std::process::Command::new(binary())
        .args(["init-project"])
        .env("ENVSWITCH_HOME", &home)
        .current_dir(&project_dir)
        .output()
        .expect("Failed to execute");

    assert!(output.status.success(),
        "Failed: {}", String::from_utf8_lossy(&output.stderr));
    assert!(project_dir.join(".envswitchrc").exists());
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn test_cover_uncover_flow() {
    let home = test_home("flow");

    // Create mock install: jdk 21
    let jdk_dir = home.join("envs").join("jdk").join("21").join("bin");
    fs::create_dir_all(&jdk_dir).unwrap();
    fs::write(jdk_dir.join("java"), b"fake").unwrap();
    // Set exec perms
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&jdk_dir.join("java"), fs::Permissions::from_mode(0o755));
    }

    // Also create go 1.22
    let go_dir = home.join("envs").join("go").join("1.22").join("bin");
    fs::create_dir_all(&go_dir).unwrap();
    fs::write(go_dir.join("go"), b"fake").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&go_dir.join("go"), fs::Permissions::from_mode(0o755));
    }

    // cover jdk 21
    let out = ok(&["cover", "jdk", "21"], &home);
    assert!(out.contains("JAVA_HOME"));
    assert!(out.contains("jdk/21"));

    // cover go 1.22
    let out2 = ok(&["cover", "go", "1.22"], &home);
    assert!(out2.contains("GOROOT"));
    assert!(out2.contains("go/1.22"));
    // Should still have JAVA_HOME
    assert!(out2.contains("JAVA_HOME"));

    // status
    let st = ok(&["status"], &home);
    assert!(st.contains("jdk"));
    assert!(st.contains("go"));

    // uncover jdk
    let out3 = ok(&["uncover", "jdk"], &home);
    // Should NOT contain jdk/21 anymore
    assert!(!out3.contains("jdk/21"));
    // Should still have go
    assert!(out3.contains("go/1.22"));

    // uncover all
    let out4 = ok(&["uncover", "--all"], &home);
    assert!(!out4.contains("jdk/21"));
    assert!(!out4.contains("go/1.22"));

    let _ = fs::remove_dir_all(&home);
}

#[test]
fn test_cover_already_covered() {
    let home = test_home("already");

    let jdk_dir = home.join("envs").join("jdk").join("21").join("bin");
    fs::create_dir_all(&jdk_dir).unwrap();
    fs::write(jdk_dir.join("java"), b"fake").unwrap();

    ok(&["cover", "jdk", "21"], &home);
    // Second cover of same version should succeed (not error)
    let out = ok(&["cover", "jdk", "21"], &home);
    // Should still output valid script
    assert!(out.contains("JAVA_HOME"));

    let _ = fs::remove_dir_all(&home);
}

#[test]
fn test_init_command() {
    let home = test_home("init_cmd");
    let out = ok(&["init", "zsh"], &home);
    // Should create init.sh
    assert!(home.join("init.sh").exists());
    // Should output the source line
    let init_path = home.join("init.sh");
    assert!(out.contains(&init_path.to_string_lossy().to_string()));
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn test_cover_unknown_module() {
    let home = test_home("cover_unk");
    let e = err(&["cover", "foobar", "1.0"], &home);
    assert!(e.contains("Unknown module"), "Got: {}", e);
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn test_uncover_not_covered() {
    let home = test_home("uncover_not");
    let jdk_dir = home.join("envs").join("jdk").join("21").join("bin");
    fs::create_dir_all(&jdk_dir).unwrap();
    fs::write(jdk_dir.join("java"), b"fake").unwrap();

    // uncover not-covered module → succeeds with no-op + message on stderr
    let output = run(&["uncover", "jdk"], &home);
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not currently covered"), "Got: {}", stderr);
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn test_cover_to_uncover_all() {
    let home = test_home("cover2all");

    let jdk_dir = home.join("envs").join("jdk").join("21").join("bin");
    fs::create_dir_all(&jdk_dir).unwrap();
    fs::write(jdk_dir.join("java"), b"fake").unwrap();

    // cover jdk
    ok(&["cover", "jdk", "21"], &home);
    // status shows it
    let st = ok(&["status"], &home);
    assert!(st.contains("jdk"));
    // uncover --all
    let out = ok(&["uncover", "--all"], &home);
    // should clear all env vars
    assert!(out.contains("unset"));
    // status empty
    let st2 = ok(&["status"], &home);
    assert!(st2.contains("No active covers"));

    let _ = fs::remove_dir_all(&home);
}
