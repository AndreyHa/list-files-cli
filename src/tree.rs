// src/tree.rs
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

pub struct Tree {
    pub text: String,
    pub ordered_paths: Vec<PathBuf>,
}

pub fn build_tree(files: &[PathBuf]) -> Tree {
    if files.is_empty() {
        return Tree {
            text: ".\n".to_string(),
            ordered_paths: vec![],
        };
    }

    let mut tree: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut all_dirs: BTreeSet<String> = BTreeSet::new();
    let mut file_map: BTreeMap<String, PathBuf> = BTreeMap::new();

    // Find the common root by path components (after normalizing separators)
    let first_path = files[0].to_string_lossy().replace('\\', "/");
    let mut common_parts: Vec<String> = first_path.split('/').map(|s| s.to_string()).collect();
    for file in files.iter().skip(1) {
        let file_path = file.to_string_lossy().replace('\\', "/");
        let parts: Vec<String> = file_path.split('/').map(|s| s.to_string()).collect();
        let mut new_common = Vec::new();
        for (a, b) in common_parts.iter().zip(parts.iter()) {
            if a == b {
                new_common.push(a.clone());
            } else {
                break;
            }
        }
        common_parts = new_common;
        if common_parts.is_empty() {
            break;
        }
    }

    let root_path = if common_parts.is_empty()
        || (common_parts.len() == 1 && common_parts[0].is_empty())
    {
        "".to_string()
    } else {
        common_parts.join("/")
    };

    // Build a relative map under the common root; keys in `tree` are relative ("" at top)
    for file in files {
        let path_str = file.to_string_lossy().replace('\\', "/");
        let path_str = path_str.strip_prefix("./").unwrap_or(&path_str);
        let relative_path = if root_path.is_empty() {
            path_str.to_string()
        } else {
            path_str
                .strip_prefix(&format!("{}/", root_path))
                .unwrap_or(&path_str)
                .to_string()
        };

        file_map.insert(relative_path.clone(), file.clone());

        let parts: Vec<&str> = relative_path.split('/').collect();

        if parts.len() == 1 {
            tree.entry("".to_string())
                .or_insert_with(BTreeSet::new)
                .insert(parts[0].to_string());
        } else {
            let mut current = "".to_string();
            for i in 0..parts.len() - 1 {
                let parent = current.clone();
                if !current.is_empty() {
                    current.push('/');
                }
                current.push_str(parts[i]);
                all_dirs.insert(current.clone());
                tree.entry(parent)
                    .or_insert_with(BTreeSet::new)
                    .insert(parts[i].to_string());
            }
            tree.entry(current)
                .or_insert_with(BTreeSet::new)
                .insert(parts.last().unwrap().to_string());
        }
    }

    fn build_tree_recursive(
        dir: &str,
        tree: &BTreeMap<String, BTreeSet<String>>,
        all_dirs: &BTreeSet<String>,
        prefix: &str,
        ordered_files: &mut Vec<String>,
    ) -> String {
        let mut output = String::new();
        let items = tree.get(dir).cloned().unwrap_or_default();

        // Separate into files and directories
        let mut dirs = Vec::new();
        let mut files = Vec::new();
        for item in items {
            let full_path = if dir.is_empty() {
                item.clone()
            } else {
                format!("{}/{}", dir, item)
            };
            if all_dirs.contains(&full_path) {
                dirs.push(item);
            } else {
                files.push(item);
            }
        }

        // Sort both groups lexicographically
        files.sort();
        dirs.sort();

        // Combine: FILES FIRST, then DIRECTORIES (matches integration tests)
        let sorted_items = files
            .into_iter()
            .chain(dirs.into_iter())
            .collect::<Vec<_>>();

        for (i, item) in sorted_items.iter().enumerate() {
            let is_last_item = i == sorted_items.len() - 1;
            let full_path = if dir.is_empty() {
                item.clone()
            } else {
                format!("{}/{}", dir, item)
            };

            // Print the prefix and branch
            output.push_str(prefix);
            if is_last_item {
                output.push_str("└── ");
            } else {
                output.push_str("├── ");
            }
            output.push_str(item);
            output.push('\n');

            // If it's a directory, recurse
            if all_dirs.contains(&full_path) {
                let new_prefix = if prefix.is_empty() {
                    if is_last_item {
                        "    ".to_string()
                    } else {
                        "│   ".to_string()
                    }
                } else {
                    format!(
                        "{}{}",
                        prefix,
                        if is_last_item { "    " } else { "│   " }
                    )
                };
                output.push_str(&build_tree_recursive(
                    &full_path,
                    tree,
                    all_dirs,
                    &new_prefix,
                    ordered_files,
                ));
            } else {
                // It's a file, add to ordered list in the exact tree order
                ordered_files.push(full_path);
            }
        }
        output
    }

    let mut ordered_files: Vec<String> = Vec::new();
    let mut output = String::new();

    // Always show "." as the root header for consistent UX
    output.push_str(".\n");
    // IMPORTANT: recurse from the top-level key "" (not root_path),
    // because our internal map is relative to `root_path`.
    output.push_str(&build_tree_recursive(
        "",
        &tree,
        &all_dirs,
        "",
        &mut ordered_files,
    ));

    let ordered_paths: Vec<PathBuf> = ordered_files
        .into_iter()
        .map(|p| file_map[&p].clone())
        .collect();

    Tree {
        text: output,
        ordered_paths,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_build_tree_simple_files_first() {
        let files = vec![
            PathBuf::from("a.txt"),
            PathBuf::from("b.txt"),
            PathBuf::from("subdir/c.txt"),
        ];

        let tree = build_tree(&files);
        println!("Actual tree text:\n{}", tree.text);

        // Header should be "."
        assert!(tree.text.starts_with(".\n"));

        // Files should be listed before directories
        assert!(tree.text.contains("├── a.txt"));
        assert!(tree.text.contains("├── b.txt"));
        assert!(tree.text.contains("└── subdir"));
        assert!(tree.text.contains("    └── c.txt"));

        // Ordered paths should reflect the printed order: a, b, then subdir/c
        assert_eq!(tree.ordered_paths.len(), 3);
        assert_eq!(tree.ordered_paths[0], PathBuf::from("a.txt"));
        assert_eq!(tree.ordered_paths[1], PathBuf::from("b.txt"));
        assert_eq!(tree.ordered_paths[2], PathBuf::from("subdir/c.txt"));
    }

    #[test]
    fn test_build_tree_files_first_policy() {
        let files = vec![
            PathBuf::from("file.txt"),
            PathBuf::from("dir1/file2.txt"),
            PathBuf::from("dir2/file3.txt"),
        ];

        let tree = build_tree(&files);
        let lines: Vec<&str> = tree.text.lines().collect();

        // Ensure we see the file listed in the top-level block before directories are listed
        let mut saw_file = false;
        for line in lines {
            if line.contains("file.txt") {
                saw_file = true;
            }
            // When dir headings appear at the top-level, we should have already seen the file line.
            if line.starts_with("├── dir1") || line.starts_with("└── dir1")
                || line.starts_with("├── dir2") || line.starts_with("└── dir2")
            {
                assert!(saw_file, "Files must appear before directories at the top level");
            }
        }

        // ordered_paths includes only the actual files, in tree order.
        assert_eq!(tree.ordered_paths.len(), 3);
    }

    #[test]
    fn test_build_tree_empty() {
        let files = vec![];
        let tree = build_tree(&files);
        assert_eq!(tree.text, ".\n");
        assert!(tree.ordered_paths.is_empty());
    }

    #[test]
    fn test_build_tree_with_absolute_like_paths() {
        // Simulate absolute or rooted paths (works cross-platform as strings)
        let files = vec![
            PathBuf::from("C:/tmp/proj/a.txt"),
            PathBuf::from("C:/tmp/proj/subdir/c.txt"),
        ];

        let tree = build_tree(&files);

        // Still prints "." header and relative-like entries
        assert!(tree.text.starts_with(".\n"));
        assert!(tree.text.contains("├── a.txt"));
        assert!(tree.text.contains("└── subdir"));
        assert!(tree.text.contains("    └── c.txt"));

        // ordered_paths length equals input file count
        assert_eq!(tree.ordered_paths.len(), 2);
    }
}
