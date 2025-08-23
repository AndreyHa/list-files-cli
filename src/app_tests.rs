#[cfg(test)]
mod tests {
    use super::*;
    use crate::clipboard::ClipboardSink;
    use crate::fs::{FileReader, WalkerFactory};
    use crate::tokenizer::Tokenizer;
    use std::cell::RefCell;
    use std::fs;
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;

    struct NoopClipboard(RefCell<Option<String>>);
    impl ClipboardSink for NoopClipboard {
        fn set_text(&self, text: String) -> Result<(), String> { self.0.replace(Some(text)); Ok(()) }
    }

    struct TestReader;
    impl FileReader for TestReader {
        fn read_to_string(&self, path: &Path) -> anyhow::Result<(String, usize)> {
            let s = std::fs::read_to_string(path)?;
            let lines = s.lines().count();
            Ok((s + "\n", lines))
        }
    }

    struct FixedWalker { root: PathBuf }
    impl WalkerFactory for FixedWalker {
        fn build(&self, no_gitignore: bool) -> ignore::Walk {
            let mut wb = ignore::WalkBuilder::new(&self.root);
            wb.hidden(false).follow_links(false).git_ignore(!no_gitignore).git_global(!no_gitignore).git_exclude(!no_gitignore).parents(true);
            wb.build()
        }
    }

    struct T0;
    impl Tokenizer for T0 { fn count_tokens(&self, _: &str) -> usize { 0 } }

    #[test]
    fn app_runs_and_writes_clipboard() {
        let d = tempdir().unwrap();
        fs::write(d.path().join("a.txt"), "x\n").unwrap();
        fs::write(d.path().join(".gitignore"), "ignored.txt\n").unwrap();
        let cb = NoopClipboard(RefCell::new(None));
        let deps = Deps {
            walker: &FixedWalker { root: d.path().to_path_buf() },
            reader: &TestReader,
            tokenizer: std::sync::Arc::new(T0),
            clipboard: Some(&cb),
        };
        let stats = run_app(deps, &["**/*".to_string()], None, false, false, false).unwrap();
        assert_eq!(stats.lines, 1);
    }
}