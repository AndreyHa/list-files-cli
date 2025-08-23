#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[test]
    fn detects_binary_extensions() {
        let d = tempdir().unwrap();
        let exe = d.path().join("app.exe");
        fs::write(&exe, "x").unwrap();
        assert!(is_binary_file(&exe));
        let jar = d.path().join("lib.jar");
        fs::write(&jar, "x").unwrap();
        assert!(is_binary_file(&jar));
    }

    #[test]
    fn formats_binary_info() {
        let d = tempdir().unwrap();
        let f = d.path().join("file.bin");
        fs::write(&f, vec![0u8; 2048]).unwrap();
        let s = get_binary_file_info(&f).unwrap();
        assert!(s.contains("Binary file") || s.contains("Document file") || s.contains("Archive file") || s.contains("Image file") || s.contains("Audio file") || s.contains("Video file"));
    }
}