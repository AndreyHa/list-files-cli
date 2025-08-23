use anyhow::{Context, Result};
use ignore::WalkBuilder;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

pub trait FileReader: Send + Sync {
    fn read_to_string(&self, path: &Path) -> Result<(String, usize)>;
}

pub struct StdFileReader;

impl FileReader for StdFileReader {
    fn read_to_string(&self, path: &Path) -> Result<(String, usize)> {
        let file = File::open(path).with_context(|| format!("Failed to open file: {}", path.display()))?;
        let reader = BufReader::new(file);
        let mut content = String::new();
        let mut lines = 0usize;
        for line in reader.lines() {
            let line = line.with_context(|| format!("Failed to read line from: {}", path.display()))?;
            content.push_str(&line);
            content.push('\n');
            lines += 1;
        }
        Ok((content, lines))
    }
}

pub trait WalkerFactory: Send + Sync {
    fn build(&self, no_gitignore: bool) -> ignore::Walk;
}

pub struct StdWalkerFactory;

impl WalkerFactory for StdWalkerFactory {
    fn build(&self, no_gitignore: bool) -> ignore::Walk {
        let mut wb = WalkBuilder::new(".");
        wb.hidden(false)
            .follow_links(false)
            .git_ignore(!no_gitignore)
            .git_global(!no_gitignore)
            .git_exclude(!no_gitignore)
            .parents(true);
        wb.build()
    }
}

pub fn collect_files(factory: &dyn WalkerFactory) -> Vec<PathBuf> {
    factory
        .build(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|ft| ft.is_file()).unwrap_or(false))
        .map(|e| e.into_path())
        .collect()
}