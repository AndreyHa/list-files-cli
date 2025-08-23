use crate::binary::{get_binary_file_info, is_binary_file};
use crate::clipboard::ClipboardSink;
use crate::fs::{FileReader, WalkerFactory};
use crate::patterns::{build_glob_sets, path_matches};
use crate::tokenizer::Tokenizer;
use anyhow::{Context, Result};
use globset::GlobSet;
use rayon::prelude::*;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

pub struct Deps<'a> {
    pub walker: &'a dyn WalkerFactory,
    pub reader: &'a dyn FileReader,
    pub tokenizer: Arc<dyn Tokenizer>,
    pub clipboard: Option<&'a dyn ClipboardSink>,
}

pub struct Stats {
    pub lines: usize,
    pub tokens: usize,
}

fn java_mask(content: &str) -> String {
    let mut out = String::new();
    let mut added = false;
    for line in content.lines() {
        if line.trim_start().starts_with("import ") {
            if !added { out.push_str("import ...\n"); added = true; }
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    if added { out } else { content.to_string() }
}

fn process_file(path: &Path, reader: &dyn FileReader, tokenizer: &dyn Tokenizer, mask_java: bool) -> Result<(String, usize, usize)> {
    if is_binary_file(path) {
        let info = get_binary_file_info(path)?;
        let tokens = tokenizer.count_tokens(&info);
        return Ok((info, 0, tokens));
    }
    let (mut content, lines) = reader.read_to_string(path)?;
    if mask_java {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if ext.eq_ignore_ascii_case("java") { content = java_mask(&content); }
        }
    }
    let tokens = tokenizer.count_tokens(&content);
    Ok((content, lines, tokens))
}

fn format_entry(path: &Path, content: &str) -> String {
    let p = path.to_string_lossy().replace('\\', "/");
    let disp = p.strip_prefix("./").unwrap_or(&p);
    let mut s = String::new();
    s.push_str(disp);
    s.push('\n');
    s.push_str(content);
    s.push_str("\n\n");
    s
}

fn collect_matching_files(walker: &dyn WalkerFactory, include: &GlobSet, hidden_inc: &GlobSet, exclude: &GlobSet, no_gitignore: bool) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for e in walker.build(no_gitignore).into_iter().filter_map(|e| e.ok()) {
        if e.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
            let p = e.into_path();
            if path_matches(&p, include, hidden_inc, exclude) { files.push(p); }
        }
    }
    files
}

pub fn run_app(deps: Deps, patterns: &[String], output_path: Option<&Path>, no_clipboard: bool, mask_java_imports: bool, no_gitignore: bool) -> Result<Stats> {
    let (include_set, hidden_include_set, exclude_set) = build_glob_sets(patterns, !no_gitignore)?;
    let files = collect_matching_files(deps.walker, &include_set, &hidden_include_set, &exclude_set, no_gitignore);
    if files.is_empty() {
        println!("No files found matching the patterns.");
        return Ok(Stats { lines: 0, tokens: 0 });
    }
    let total_lines = Arc::new(AtomicUsize::new(0));
    let total_tokens = Arc::new(AtomicUsize::new(0));
    let use_clipboard = !no_clipboard && output_path.is_none();
    let content_buffer = if use_clipboard { Some(Arc::new(Mutex::new(String::new()))) } else { None };
    let mut output_writer: Option<Box<dyn Write + Send>> = if let Some(p) = output_path {
        let f = std::fs::File::create(p).with_context(|| format!("Failed to create output file: {}", p.display()))?;
        Some(Box::new(BufWriter::new(f)))
    } else if no_clipboard { Some(Box::new(std::io::stdout())) } else { None };
    let tokenizer = deps.tokenizer.clone();
    let reader = deps.reader;
    let results: Result<Vec<(PathBuf, String, usize, usize)>> = files.par_iter().map(|p| {
        let (content, lines, tokens) = process_file(p, reader, tokenizer.as_ref(), mask_java_imports)?;
        Ok((p.clone(), content, lines, tokens))
    }).collect();
    let results = results?;
    for (path, content, lines, tokens) in results {
        total_lines.fetch_add(lines, Ordering::Relaxed);
        total_tokens.fetch_add(tokens, Ordering::Relaxed);
        let out = format_entry(&path, &content);
        if let Some(ref buf) = content_buffer { buf.lock().unwrap().push_str(&out); }
        else if let Some(ref mut w) = output_writer { w.write_all(out.as_bytes()).context("Failed to write to output")?; }
        else { print!("{}", out); }
    }
    if let Some(buf) = content_buffer {
        let content = buf.lock().unwrap().clone();
        if let Some(cb) = deps.clipboard { if cb.set_text(content.clone()).is_err() { print!("{}", content); } } else { print!("{}", content); }
    }
    if let Some(mut w) = output_writer { w.flush().context("Failed to flush final output")?; }
    let lines = total_lines.load(Ordering::Relaxed);
    let tokens = total_tokens.load(Ordering::Relaxed);
    println!("Lines: {}", lines);
    #[cfg(feature = "token-counting")]
    println!("Tokens (o200k_base): {}", tokens);
    Ok(Stats { lines, tokens })
}