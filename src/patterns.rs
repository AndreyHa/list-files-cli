use anyhow::Result;
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::path::Path;

fn is_hidden_glob(glob: &str) -> bool {
    let g = glob.trim_start_matches("./");
    (g.starts_with('.') && g.len() > 1) || g.contains("/.")
}

fn normalize_pattern(p: &str) -> String {
    match p {
        "." | "./" => "**/*".to_string(),
        _ => {
            if p.ends_with('/') { format!("{}**/*", p) }
            else if p.starts_with('.') && !p.contains(['*','/']) { format!("{}/**", p) }
            else if !p.contains(['*','/','.']) { format!("{}/**", p) }
            else { p.to_string() }
        }
    }
}

fn gitignore_line_to_glob(p: &str) -> String {
    let mut pat = p.trim().to_string();
    if pat.starts_with('/') { pat = pat.trim_start_matches('/').to_string(); }
    if pat.ends_with('/') { pat.pop(); }
    if pat.contains('*') { return pat; }
    if pat.contains('/') {
        return format!("**/{}", pat);
    }
    if pat.contains('.') {
        return format!("**/{}", pat);
    }
    format!("**/{}/**", pat)
}

pub fn build_glob_sets(patterns: &[String], honor_gitignore: bool) -> Result<(GlobSet, GlobSet, GlobSet)> {
    let mut vis_inc = GlobSetBuilder::new();
    let mut hid_inc = GlobSetBuilder::new();
    let mut exc = GlobSetBuilder::new();

    for p in patterns {
        if let Some(raw) = p.strip_prefix('~') { exc.add(Glob::new(raw)?); continue; }
        let norm = normalize_pattern(p);
        if is_hidden_glob(&norm) { hid_inc.add(Glob::new(&norm)?); } else { vis_inc.add(Glob::new(&norm)?); }
    }

    if honor_gitignore {
        let mut add_ignore_file = |path: &Path| -> Result<()> {
            if path.exists() {
                let s = std::fs::read_to_string(path)?;
                for line in s.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.starts_with('#') { continue; }
                    if trimmed.starts_with('!') { continue; }
                    if let Some(rest) = trimmed.strip_prefix('/') {
                        let dir_pat = if rest.ends_with('/') { format!("{}**/*", rest) } else { format!("{}/**", rest) };
                        let any_pat = if rest.ends_with('/') { format!("**/{}**/*", rest) } else { format!("**/{}/**", rest) };
                        exc.add(Glob::new(&dir_pat)?);
                        exc.add(Glob::new(&any_pat)?);
                    } else {
                        let glob_pat = gitignore_line_to_glob(trimmed);
                        exc.add(Glob::new(&glob_pat)?);
                    }
                }
            }
            Ok(())
        };
        let cwd = Path::new(".");
        let _ = add_ignore_file(&cwd.join(".gitignore"));
        let _ = add_ignore_file(&cwd.join(".git").join("info").join("exclude"));
        if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
            let h = Path::new(&home);
            let _ = add_ignore_file(&h.join(".gitignore_global"));
            let _ = add_ignore_file(&h.join(".config").join("git").join("ignore"));
        }
    }

    Ok((vis_inc.build()?, hid_inc.build()?, exc.build()?))
}

pub fn path_matches(path: &Path, include_set: &GlobSet, hidden_include_set: &GlobSet, exclude_set: &GlobSet) -> bool {
    let path_str = path.to_string_lossy().replace('\\', "/");
    let stripped = path_str.strip_prefix("./").unwrap_or(&path_str);
    let file = path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
    let hidden = stripped.split('/').any(|c| c.starts_with('.') && c != "." && c != "..");
    let inc = if hidden {
        hidden_include_set.is_match(&path_str) || hidden_include_set.is_match(stripped) || hidden_include_set.is_match(&file)
    } else {
        include_set.is_match(&path_str) || include_set.is_match(stripped) || include_set.is_match(&file)
    };
    inc && !exclude_set.is_match(&path_str) && !exclude_set.is_match(stripped) && !exclude_set.is_match(&file)
}