use crate::binary::{get_binary_file_info, is_binary_file};
use crate::clipboard::ClipboardSink;
use crate::fs::{FileReader, WalkerFactory};
use crate::output::{BufferedOutputSink, OutputSink};
use crate::patterns::{build_glob_sets, path_matches};
use crate::tokenizer::Tokenizer;
use crate::tree::build_tree;
use anyhow::{Context, Result};
use globset::GlobSet;
use rayon::prelude::*;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

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
}pub fn run_app(deps: Deps, patterns: &[String], output_path: Option<&Path>, no_clipboard: bool, mask_java_imports: bool, no_gitignore: bool, tree: bool) -> Result<Stats> {
    let (include_set, hidden_include_set, exclude_set) = build_glob_sets(patterns, !no_gitignore)?;
    let mut files = collect_matching_files(deps.walker, &include_set, &hidden_include_set, &exclude_set, no_gitignore);

    if files.is_empty() && !tree {
        println!("No files found matching the patterns.");
        return Ok(Stats { lines: 0, tokens: 0 });
    }

    // Sort files for consistent tree order
    files.sort_by(|a, b| a.to_string_lossy().cmp(&b.to_string_lossy()));

    let total_lines = Arc::new(AtomicUsize::new(0));
    let total_tokens = Arc::new(AtomicUsize::new(0));
    let use_clipboard = !no_clipboard && output_path.is_none();

    // Create output sink
    let mut output_sink = if let Some(p) = output_path {
        let f = std::fs::File::create(p).with_context(|| format!("Failed to create output file: {}", p.display()))?;
        BufferedOutputSink::new(OutputSink::File(BufWriter::new(f)))
    } else if use_clipboard {
        if let Some(cb) = deps.clipboard {
            BufferedOutputSink::new(OutputSink::Clipboard(cb))
        } else {
            BufferedOutputSink::new(OutputSink::Stdout)
        }
    } else {
        BufferedOutputSink::new(OutputSink::Stdout)
    };

    let tokenizer = deps.tokenizer.clone();
    let reader = deps.reader;

    // If tree mode, generate and output tree first
    let ordered_files = if tree {
        let tree = build_tree(&files);
        let tree_entry = format!("File Tree:\n{}\n\n", tree.text);
        output_sink.write_all(&tree_entry)?;
        tree.ordered_paths
    } else {
        files
    };

    // Process files in parallel with indices for deterministic ordering
    let results: Result<Vec<(usize, PathBuf, String, usize, usize)>> = ordered_files
        .iter()
        .enumerate()
        .par_bridge()
        .map(|(idx, p)| {
            let (content, lines, tokens) = process_file(p, reader, tokenizer.as_ref(), mask_java_imports)?;
            Ok((idx, p.clone(), content, lines, tokens))
        })
        .collect();

    let mut results = results?;
    // Sort by index to ensure deterministic order
    results.sort_by_key(|(idx, ..)| *idx);

    // Output files in order
    for (_idx, path, content, lines, tokens) in results {
        total_lines.fetch_add(lines, Ordering::Relaxed);
        total_tokens.fetch_add(tokens, Ordering::Relaxed);
        let out = format_entry(&path, &content);
        output_sink.write_all(&out)?;
    }

    // Finish output
    output_sink.finish()?;

    let lines = total_lines.load(Ordering::Relaxed);
    let tokens = total_tokens.load(Ordering::Relaxed);
    println!("Lines: {}", lines);
    #[cfg(feature = "token-counting")]
    println!("Tokens (o200k_base): {}", tokens);
    Ok(Stats { lines, tokens })
}