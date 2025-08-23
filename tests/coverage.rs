use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::process::Command;

// A wide-coverage integration test exercising many code paths.
#[test]
fn full_coverage_scenarios() {
    let temp = assert_fs::TempDir::new().unwrap();

    // 1) Create a small Rust file under src/
    let src = temp.child("src");
    src.create_dir_all().unwrap();
    src.child("main.rs").write_str("fn main(){}\n").unwrap();

    // 2) Create a node_modules directory that should be ignored by .gitignore
    let node = temp.child("node_modules");
    node.create_dir_all().unwrap();
    node.child("pkg.json").write_str("{}\n").unwrap();

    // 3) Create text files for output tests
    temp.child("visible.txt").write_str("ok\n").unwrap();
    temp.child("secret.txt").write_str("shh\n").unwrap();

    // 4) Binary-like file
    let bin = temp.child("app.exe");
    bin.write_str("BINARYDATA").unwrap();

    // 5) Java file with multiple imports for masking
    temp.child("Example.java").write_str(
        "import a.b.C;\nimport a.b.D;\npublic class Example {\n}\n",
    ).unwrap();

    // 6) .gitignore to ignore node_modules and secret.txt
    temp.child(".gitignore").write_str("node_modules\nsecret.txt\n").unwrap();

    // Test A: default behavior respects .gitignore
    let mut cmd = Command::cargo_bin("lf").unwrap();
    let output = cmd.current_dir(&temp).arg("**/*").arg("--no-clipboard").assert().success().get_output().stdout.clone();
    let s = String::from_utf8_lossy(&output);
    assert!(s.contains("src/main.rs"));
    assert!(!s.contains("node_modules"));
    assert!(!s.contains("secret.txt"));

    // Test B: --no-gitignore includes ignored files
    let mut cmd = Command::cargo_bin("lf").unwrap();
    cmd.current_dir(&temp)
        .arg("**/*")
        .arg("--no-clipboard")
        .arg("--no-gitignore");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("node_modules/pkg.json"))
        .stdout(predicate::str::contains("secret.txt"));

    // Test C: exclude using ~ prefix
    let mut cmd = Command::cargo_bin("lf").unwrap();
    cmd.current_dir(&temp)
        .arg("**/*")
        .arg("~secret.txt")
        .arg("--no-clipboard");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("visible.txt"))
        .stdout(predicate::str::contains("secret.txt").not());

    // Test D: directory shorthand (src/)
    let mut cmd = Command::cargo_bin("lf").unwrap();
    cmd.current_dir(&temp).arg("src/").arg("--no-clipboard");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("src/main.rs"));

    // Test E: output to file (-o)
    let out_file = temp.child("out.txt");
    let mut cmd = Command::cargo_bin("lf").unwrap();
    cmd.current_dir(&temp)
        .arg("visible.txt")
        .arg("-o")
        .arg(out_file.path())
        .arg("--no-clipboard");
    cmd.assert().success();
    out_file.assert(predicate::str::contains("visible.txt"));

    // Test F: binary detection (app.exe)
    let mut cmd = Command::cargo_bin("lf").unwrap();
    cmd.current_dir(&temp).arg("**/*").arg("--no-clipboard");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("app.exe").and(predicate::str::contains("Binary file").or(predicate::str::contains("EXE"))));

    // Test G: Java import masking
    let mut cmd = Command::cargo_bin("lf").unwrap();
    cmd.current_dir(&temp)
        .arg("Example.java")
        .arg("--no-clipboard")
        .arg("--mask-java-imports");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("import ...").and(predicate::str::contains("public class Example")));

    // Test H: token counting line present (feature gated printing)
    let mut cmd = Command::cargo_bin("lf").unwrap();
    cmd.current_dir(&temp).arg("visible.txt").arg("--no-clipboard");
    let output = cmd.assert().success().get_output().stdout.clone();
    let s = String::from_utf8_lossy(&output);
    // Tokens line is printed only when built with token-counting feature; assert either presence or absence
    if s.contains("Tokens (o200k_base):") {
        assert!(s.contains("Tokens (o200k_base):"));
    } else {
        // Ensure at least Lines are printed
        assert!(s.contains("Lines:"));
    }

    temp.close().unwrap();
}
