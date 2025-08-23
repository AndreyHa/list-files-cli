use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::process::Command;

#[test]
fn gitignore_directory_pattern_excludes_dir() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("node_modules").create_dir_all().unwrap();
    temp.child("node_modules").child("pkg.json").write_str("{}\n").unwrap();
    temp.child("src").create_dir_all().unwrap();
    temp.child("src").child("main.rs").write_str("fn main(){}\n").unwrap();
    temp.child(".gitignore").write_str("node_modules\n").unwrap();

    let mut cmd = Command::cargo_bin("lf").unwrap();
    cmd.current_dir(&temp).arg("**/*").arg("--no-clipboard");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("src/main.rs"))
        .stdout(predicate::str::contains("node_modules").not());

    temp.close().unwrap();
}

#[test]
fn gitignore_file_pattern_excludes_file() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("visible.txt").write_str("ok\n").unwrap();
    temp.child("secret.txt").write_str("shh\n").unwrap();
    temp.child(".gitignore").write_str("secret.txt\n").unwrap();

    let mut cmd = Command::cargo_bin("lf").unwrap();
    cmd.current_dir(&temp).arg("**/*").arg("--no-clipboard");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("visible.txt"))
        .stdout(predicate::str::contains("secret.txt").not());

    temp.close().unwrap();
}

#[test]
fn gitignore_negation_is_ignored_but_common_cases_work() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("build").create_dir_all().unwrap();
    temp.child("build").child("keep.txt").write_str("ok\n").unwrap();
    temp.child("build").child("other.txt").write_str("no\n").unwrap();
    temp.child(".gitignore").write_str("build\n!build/keep.txt\n").unwrap();

    // Our simple parser ignores negations, so keep.txt will be excluded by the
    // simple conversion; assert we at least exclude the directory (other.txt)
    let mut cmd = Command::cargo_bin("lf").unwrap();
    cmd.current_dir(&temp).arg("**/*").arg("--no-clipboard");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("other.txt").not());

    temp.close().unwrap();
}
