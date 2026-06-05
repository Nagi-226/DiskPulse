pub mod features;
pub mod magic;
pub mod model;

use serde::{Deserialize, Serialize};
use std::path::Path;

pub use features::{extract_features, FileFeatures};
pub use model::{ClassifierModel, ClassifierOutput, MODEL_VERSION};

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ClassificationStage {
    Extension,
    Magic,
    Stage3Model,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClassificationResult {
    pub category: FileCategory,
    pub category_id: String,
    pub stage: ClassificationStage,
    pub confidence: f64,
    pub model_version: Option<String>,
}

pub fn classify_path(path: &Path) -> FileCategory {
    classify_path_with_metadata(path, 0, None).category
}

pub fn classify_path_with_metadata(
    path: &Path,
    size_bytes: u64,
    sample: Option<&[u8]>,
) -> ClassificationResult {
    if let Some(category) = classify_by_extension(path) {
        return result(category, ClassificationStage::Extension, 0.95, None);
    }

    if let Some(bytes) = sample {
        let magic_category = magic::classify_magic(bytes);
        if magic_category != FileCategory::Unknown {
            return result(magic_category, ClassificationStage::Magic, 0.92, None);
        }
    }

    let features = extract_features(path, size_bytes, sample.unwrap_or(&[]));
    let output = ClassifierModel::default().predict(&features);
    if output.confidence >= 0.35 && output.category != FileCategory::Unknown {
        return result(
            output.category,
            ClassificationStage::Stage3Model,
            output.confidence,
            Some(MODEL_VERSION.into()),
        );
    }

    result(
        FileCategory::Unknown,
        ClassificationStage::Unknown,
        output.confidence,
        None,
    )
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

fn result(
    category: FileCategory,
    stage: ClassificationStage,
    confidence: f64,
    model_version: Option<String>,
) -> ClassificationResult {
    ClassificationResult {
        category_id: category_id(&category).into(),
        category,
        stage,
        confidence: confidence.clamp(0.0, 1.0),
        model_version,
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
    magic::classify_magic(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_classifies_common_files() {
        assert_eq!(classify_path(Path::new("report.pdf")), FileCategory::Pdf);
        assert_eq!(classify_path(Path::new("cache.tmp")), FileCategory::Temp);
        assert_eq!(
            classify_path(Path::new("movie.mp4")),
            FileCategory::MediaVideo
        );
    }

    #[test]
    fn magic_classifies_header_when_extension_missing() {
        assert_eq!(classify_magic(b"%PDF-1.7"), FileCategory::Pdf);
        assert_eq!(
            classify_magic(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a]),
            FileCategory::MediaImage
        );
    }

    #[test]
    fn stage3_classifies_dependency_paths_without_extension() {
        let classified = classify_path_with_metadata(
            Path::new(r"C:\repo\node_modules\react\index"),
            48_000,
            Some(b"module.exports = require('react');"),
        );

        assert_eq!(classified.stage, ClassificationStage::Stage3Model);
        assert_eq!(classified.category, FileCategory::Dependency);
        assert!(classified.confidence >= 0.35);
    }

    #[test]
    fn stage3_classifies_cache_build_paths_without_extension() {
        let dev_cache = classify_path_with_metadata(
            Path::new(r"C:\Users\me\AppData\Local\npm-cache\blob"),
            12_000,
            Some(b"{\"integrity\":\"sha512\",\"tarball\":\"pkg\"}"),
        );
        let build = classify_path_with_metadata(
            Path::new(r"C:\repo\target\release\artifact"),
            3_000_000,
            Some(&[0, 1, 2, 3, 4, 5, 6, 7]),
        );

        assert_eq!(dev_cache.category, FileCategory::DevCache);
        assert_eq!(build.category, FileCategory::Build);
    }
}
