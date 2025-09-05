use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::process::Command;
#[test]
fn full_coverage_scenarios() {
    let temp = assert_fs::TempDir::new().unwrap();

    let src = temp.child("src");
    src.create_dir_all().unwrap();
    src.child("main.rs").write_str("fn main(){}\n").unwrap();
    let node = temp.child("node_modules");
    node.create_dir_all().unwrap();
    node.child("pkg.json").write_str("{}\n").unwrap();
    temp.child("visible.txt").write_str("ok\n").unwrap();
    temp.child("secret.txt").write_str("shh\n").unwrap();
    let bin = temp.child("app.exe");
    bin.write_str("BINARYDATA").unwrap();
    temp.child("Example.java").write_str(
        "import a.b.C;\nimport a.b.D;\npublic class Example {\n}\n",
    ).unwrap();
    temp.child(".gitignore").write_str("node_modules\nsecret.txt\n").unwrap();
    let mut cmd = Command::cargo_bin("lf").unwrap();
    let output = cmd.current_dir(&temp).arg("**/*").arg("--no-clipboard").assert().success().get_output().stdout.clone();
    let s = String::from_utf8_lossy(&output);
    assert!(s.contains("src/main.rs"));
    assert!(!s.contains("node_modules"));
    assert!(!s.contains("secret.txt"));
    let mut cmd = Command::cargo_bin("lf").unwrap();
    cmd.current_dir(&temp)
        .arg("**/*")
        .arg("--no-clipboard")
        .arg("--no-gitignore");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("node_modules/pkg.json"))
        .stdout(predicate::str::contains("secret.txt"));
    let mut cmd = Command::cargo_bin("lf").unwrap();
    cmd.current_dir(&temp)
        .arg("**/*")
        .arg("~secret.txt")
        .arg("--no-clipboard");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("visible.txt"))
        .stdout(predicate::str::contains("secret.txt").not());
    let mut cmd = Command::cargo_bin("lf").unwrap();
    cmd.current_dir(&temp).arg("src/").arg("--no-clipboard");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("src/main.rs"));
    let out_file = temp.child("out.txt");
    let mut cmd = Command::cargo_bin("lf").unwrap();
    cmd.current_dir(&temp)
        .arg("visible.txt")
        .arg("-o")
        .arg(out_file.path())
        .arg("--no-clipboard");
    cmd.assert().success();
    out_file.assert(predicate::str::contains("visible.txt"));
    let mut cmd = Command::cargo_bin("lf").unwrap();
    cmd.current_dir(&temp).arg("**/*").arg("--no-clipboard");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("app.exe").and(predicate::str::contains("Binary file").or(predicate::str::contains("EXE"))));
    let mut cmd = Command::cargo_bin("lf").unwrap();
    cmd.current_dir(&temp)
        .arg("Example.java")
        .arg("--no-clipboard")
        .arg("--mask-java-imports");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("import ...").and(predicate::str::contains("public class Example")));
    let mut cmd = Command::cargo_bin("lf").unwrap();
    cmd.current_dir(&temp).arg("visible.txt").arg("--no-clipboard");
    let output = cmd.assert().success().get_output().stdout.clone();
    let s = String::from_utf8_lossy(&output);
    if s.contains("Tokens (o200k_base):") {
        assert!(s.contains("Tokens (o200k_base):"));
    } else {
        assert!(s.contains("Lines:"));
    }

    temp.close().unwrap();
}

#[test]
fn tree_flag_outputs_tree_and_files() {
    let temp = assert_fs::TempDir::new().unwrap();

    temp.child("a.txt").write_str("content a\n").unwrap();
    temp.child("b.txt").write_str("content b\n").unwrap();
    let sub = temp.child("subdir");
    sub.create_dir_all().unwrap();
    sub.child("c.txt").write_str("content c\n").unwrap();

    let mut cmd = Command::cargo_bin("lf").unwrap();
    let output = cmd.current_dir(&temp)
        .arg("**/*")
        .arg("--tree")
        .arg("--no-clipboard")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8_lossy(&output);

    // Check that tree is present
    assert!(s.contains("File Tree:"));
    assert!(s.contains("├── a.txt"));
    assert!(s.contains("├── b.txt"));
    assert!(s.contains("└── subdir"));
    assert!(s.contains("    └── c.txt"));

    // Check ordering: tree first, then files in tree order
    let pos_tree = s.find("File Tree:").unwrap();
    let pos_a = s.find("\na.txt\n").unwrap();
    let pos_b = s.find("\nb.txt\n").unwrap();
    let pos_c = s.find("\nsubdir/c.txt\n").unwrap();
    assert!(pos_tree < pos_a);
    assert!(pos_a < pos_b);
    assert!(pos_b < pos_c);

    // Check that files are included after tree
    assert!(s.contains("a.txt"));
    assert!(s.contains("content a"));
    assert!(s.contains("b.txt"));
    assert!(s.contains("content b"));
    assert!(s.contains("subdir/c.txt"));
    assert!(s.contains("content c"));

    temp.close().unwrap();
}

#[test]
fn tree_flag_empty_selection_outputs_minimal_tree() {
    let temp = assert_fs::TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("lf").unwrap();
    let output = cmd.current_dir(&temp)
        .arg("**/*")
        .arg("--tree")
        .arg("--no-clipboard")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8_lossy(&output);

    // Should contain minimal tree
    assert!(s.contains("File Tree:\n.\n"));
    // Should have Lines: 0
    assert!(s.contains("Lines: 0"));
    // Should not have "No files found"
    assert!(!s.contains("No files found"));

    temp.close().unwrap();
}
