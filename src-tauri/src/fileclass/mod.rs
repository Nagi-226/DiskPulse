use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FileCategory {
    DocumentText,
    Office,
    Pdf,
    ArchiveCompressed,
    Installer,
    MediaImage,
    MediaAudio,
    MediaVideo,
    DevCache,
    Build,
    Dependency,
    SystemDb,
    Config,
    Temp,
    Unknown,
}

pub fn classify_path(path: &Path) -> FileCategory {
    classify_by_extension(path).unwrap_or(FileCategory::Unknown)
}

pub fn category_id(category: &FileCategory) -> &'static str {
    match category {
        FileCategory::DocumentText => "document_text",
        FileCategory::Office => "office",
        FileCategory::Pdf => "pdf",
        FileCategory::ArchiveCompressed => "archive_compressed",
        FileCategory::Installer => "installer",
        FileCategory::MediaImage => "media_image",
        FileCategory::MediaAudio => "media_audio",
        FileCategory::MediaVideo => "media_video",
        FileCategory::DevCache => "dev_cache",
        FileCategory::Build => "build",
        FileCategory::Dependency => "dependency",
        FileCategory::SystemDb => "system_db",
        FileCategory::Config => "config",
        FileCategory::Temp => "temp",
        FileCategory::Unknown => "unknown",
    }
}

fn classify_by_extension(path: &Path) -> Option<FileCategory> {
    let ext = path.extension()?.to_string_lossy().to_ascii_lowercase();
    Some(match ext.as_str() {
        "txt" | "md" | "log" | "csv" | "json" | "xml" => FileCategory::DocumentText,
        "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" => FileCategory::Office,
        "pdf" => FileCategory::Pdf,
        "zip" | "7z" | "rar" | "tar" | "gz" => FileCategory::ArchiveCompressed,
        "msi" | "exe" | "dmg" | "pkg" | "deb" | "appimage" => FileCategory::Installer,
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "ico" => FileCategory::MediaImage,
        "mp3" | "wav" | "flac" | "aac" | "ogg" => FileCategory::MediaAudio,
        "mp4" | "mov" | "mkv" | "avi" | "webm" => FileCategory::MediaVideo,
        "sqlite" | "db" | "wal" => FileCategory::SystemDb,
        "toml" | "yaml" | "yml" | "ini" | "conf" => FileCategory::Config,
        "tmp" | "temp" => FileCategory::Temp,
        _ => return None,
    })
}

pub fn classify_magic(bytes: &[u8]) -> FileCategory {
    if bytes.starts_with(b"%PDF") {
        FileCategory::Pdf
    } else if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        FileCategory::MediaImage
    } else if bytes.starts_with(b"PK\x03\x04") {
        FileCategory::ArchiveCompressed
    } else if bytes.starts_with(b"MZ") {
        FileCategory::Installer
    } else {
        FileCategory::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_classifies_common_files() {
        assert_eq!(classify_path(Path::new("report.pdf")), FileCategory::Pdf);
        assert_eq!(classify_path(Path::new("cache.tmp")), FileCategory::Temp);
        assert_eq!(classify_path(Path::new("movie.mp4")), FileCategory::MediaVideo);
    }

    #[test]
    fn magic_classifies_header_when_extension_missing() {
        assert_eq!(classify_magic(b"%PDF-1.7"), FileCategory::Pdf);
        assert_eq!(
            classify_magic(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a]),
            FileCategory::MediaImage
        );
    }
}
