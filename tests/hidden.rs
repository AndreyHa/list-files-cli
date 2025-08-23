use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::process::Command;

#[test]
fn respects_gitignore_by_default() {
    let temp = assert_fs::TempDir::new().unwrap();

    let src = temp.child("src");
    src.create_dir_all().unwrap();
    src.child("main.rs").write_str("fn main(){}\n").unwrap();
    let node = temp.child("node_modules");
    node.create_dir_all().unwrap();
    node.child("pkg.json").write_str("{}\n").unwrap();
    temp.child(".gitignore").write_str("node_modules\n").unwrap();
    let mut cmd = Command::cargo_bin("lf").unwrap();
    cmd.current_dir(&temp)
        .arg("**/*")
        .arg("--no-clipboard");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("src/main.rs"))
        .stdout(predicate::str::contains("node_modules").not());

    temp.close().unwrap();
}

#[test]
fn no_gitignore_flag_includes_ignored_files() {
    let temp = assert_fs::TempDir::new().unwrap();

    temp.child(".gitignore").write_str("secret.txt\n").unwrap();
    temp.child("visible.txt").write_str("ok\n").unwrap();
    temp.child("secret.txt").write_str("shh\n").unwrap();

    let mut cmd = Command::cargo_bin("lf").unwrap();
    cmd.current_dir(&temp)
        .arg("**/*")
        .arg("--no-clipboard")
        .arg("--no-gitignore");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("visible.txt"))
        .stdout(predicate::str::contains("secret.txt"));

    temp.close().unwrap();
}

#[test]
fn explicit_hidden_dir_can_be_included() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child(".obsidian").create_dir_all().unwrap();
    temp.child(".obsidian/config").write_str("x\n").unwrap();

    let mut cmd = Command::cargo_bin("lf").unwrap();
    cmd.current_dir(&temp)
        .arg(".obsidian")
        .arg("--no-clipboard");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(".obsidian/config"));

    temp.close().unwrap();
}
