#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn normalize_and_hidden_detection() {
        assert_eq!(normalize_pattern("."), "**/*");
        assert!(is_hidden_glob(".env"));
        assert!(is_hidden_glob("dir/.git"));
        assert_eq!(normalize_pattern("src"), "src/**");
        assert_eq!(normalize_pattern("src/"), "src/**/*");
    }

    #[test]
    fn build_globs_respects_gitignore_like() {
        let d = tempdir().unwrap();
        fs::write(d.path().join(".gitignore"), "/build\n/bin\nsecret.txt\n").unwrap();
        let patterns = vec!["**/*".to_string()];
        let (inc, hid, exc) = {
            let cwd = std::env::current_dir().unwrap();
            std::env::set_current_dir(d.path()).unwrap();
            let r = build_glob_sets(&patterns, true).unwrap();
            std::env::set_current_dir(cwd).unwrap();
            r
        };
        assert!(exc.is_match("build/file"));
        assert!(exc.is_match("x/build/file"));
        assert!(exc.is_match("bin/run"));
        assert!(exc.is_match("x/bin/run"));
        assert!(inc.is_match("src/main.rs") || hid.is_match("src/main.rs"));
    }
}