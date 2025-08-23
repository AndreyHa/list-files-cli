use anyhow::{Context, Result};
use std::path::Path;

pub fn is_binary_file(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        let ext = ext.to_string_lossy().to_lowercase();
        matches!(ext.as_str(),
            "exe"|"dll"|"so"|"dylib"|"a"|"lib"|"bin"|"o"|"obj"|"rlib"|
            "png"|"jpg"|"jpeg"|"gif"|"bmp"|"tiff"|"tga"|"ico"|"webp"|
            "mp4"|"avi"|"mkv"|"mov"|"wmv"|"flv"|"webm"|
            "mp3"|"wav"|"flac"|"ogg"|"m4a"|"aac"|
            "zip"|"rar"|"7z"|"tar"|"gz"|"bz2"|"xz"|"jar"|
            "pdf"|"doc"|"docx"|"xls"|"xlsx"|"ppt"|"pptx"|
            "pdb"|"sqlite"|"db"|"class"|"pyc"|"d"|
            "idx"|"cache"|"lock"|"tmp"|"temp"
        )
    } else {
        false
    }
}

pub fn get_binary_file_info(path: &Path) -> Result<String> {
    let metadata = std::fs::metadata(path)
        .with_context(|| format!("Failed to get metadata for: {}", path.display()))?;
    let size = metadata.len();
    let size_str = if size < 1024 {
        format!("{} bytes", size)
    } else if size < 1024 * 1024 {
        format!("{:.1} KB", size as f64 / 1024.0)
    } else if size < 1024 * 1024 * 1024 {
        format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", size as f64 / (1024.0 * 1024.0 * 1024.0))
    };
    if let Some(ext) = path.extension() {
        let ext = ext.to_string_lossy().to_lowercase();
        let kind = match ext.as_str() {
            "dll"|"so"|"dylib"|"exe"|"bin"|"jar" => "Binary file",
            "png"|"jpg"|"jpeg"|"gif"|"bmp"|"tiff"|"tga"|"ico"|"webp" => "Image file",
            "mp4"|"avi"|"mkv"|"mov"|"wmv"|"flv"|"webm" => "Video file",
            "mp3"|"wav"|"flac"|"ogg"|"m4a"|"aac" => "Audio file",
            "zip"|"rar"|"7z"|"tar"|"gz"|"bz2"|"xz" => "Archive file",
            "pdf"|"doc"|"docx"|"xls"|"xlsx"|"ppt"|"pptx" => "Document file",
            _ => "Binary file",
        };
        Ok(format!("[{}: {}]", kind, size_str))
    } else {
        Ok(format!("[Binary file - Size: {}]", size_str))
    }
}