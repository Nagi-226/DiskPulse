use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FileFeatures {
    pub file_size_log: f64,
    pub has_extension: f64,
    pub extension_entropy: f64,
    pub byte_entropy_256: f64,
    pub null_byte_ratio: f64,
    pub printable_ratio: f64,
    pub path_depth: f64,
    pub parent_dir_type: f64,
}

impl FileFeatures {
    pub fn as_array(&self) -> [f64; 8] {
        [
            self.file_size_log,
            self.has_extension,
            self.extension_entropy,
            self.byte_entropy_256,
            self.null_byte_ratio,
            self.printable_ratio,
            self.path_depth,
            self.parent_dir_type,
        ]
    }
}

pub fn extract_features(path: &Path, size_bytes: u64, sample: &[u8]) -> FileFeatures {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let path_text = path.to_string_lossy();

    FileFeatures {
        file_size_log: ((size_bytes as f64 + 1.0).log10() / 12.0).clamp(0.0, 1.0),
        has_extension: if extension.is_empty() { 0.0 } else { 1.0 },
        extension_entropy: normalized_char_entropy(extension),
        byte_entropy_256: normalized_byte_entropy(sample),
        null_byte_ratio: byte_ratio(sample, 0),
        printable_ratio: printable_ratio(sample),
        path_depth: path_text
            .replace('\\', "/")
            .split('/')
            .filter(|part| !part.is_empty())
            .count()
            .min(16) as f64
            / 16.0,
        parent_dir_type: parent_dir_type(path),
    }
}

fn normalized_char_entropy(value: &str) -> f64 {
    if value.is_empty() {
        return 0.0;
    }
    let mut buckets = [0usize; 128];
    for byte in value.bytes() {
        buckets[usize::from(byte.min(127))] += 1;
    }
    entropy_from_counts(&buckets, value.len()).clamp(0.0, 8.0) / 8.0
}

fn normalized_byte_entropy(sample: &[u8]) -> f64 {
    if sample.is_empty() {
        return 0.0;
    }
    let mut buckets = [0usize; 256];
    for byte in sample.iter().copied().take(256) {
        buckets[usize::from(byte)] += 1;
    }
    entropy_from_counts(&buckets, sample.len().min(256)).clamp(0.0, 8.0) / 8.0
}

fn entropy_from_counts<const N: usize>(counts: &[usize; N], total: usize) -> f64 {
    if total == 0 {
        return 0.0;
    }
    counts
        .iter()
        .copied()
        .filter(|count| *count > 0)
        .map(|count| {
            let p = count as f64 / total as f64;
            -p * p.log2()
        })
        .sum()
}

fn byte_ratio(sample: &[u8], target: u8) -> f64 {
    if sample.is_empty() {
        return 0.0;
    }
    sample
        .iter()
        .copied()
        .take(256)
        .filter(|byte| *byte == target)
        .count() as f64
        / sample.len().min(256) as f64
}

fn printable_ratio(sample: &[u8]) -> f64 {
    if sample.is_empty() {
        return 0.0;
    }
    sample
        .iter()
        .copied()
        .take(256)
        .filter(|byte| matches!(byte, b'\n' | b'\r' | b'\t' | 0x20..=0x7e))
        .count() as f64
        / sample.len().min(256) as f64
}

fn parent_dir_type(path: &Path) -> f64 {
    let normalized = path.to_string_lossy().replace('\\', "/").to_lowercase();
    if normalized.contains("/node_modules/") || normalized.contains("/vendor/") {
        0.95
    } else if normalized.contains("/target/")
        || normalized.contains("/build/")
        || normalized.contains("/dist/")
        || normalized.contains("/.next/")
    {
        0.80
    } else if normalized.contains("/cache/")
        || normalized.contains("npm-cache")
        || normalized.contains("/pip/cache/")
        || normalized.contains("/.cache/")
    {
        0.65
    } else if normalized.contains("/temp/") || normalized.contains("/tmp/") {
        0.50
    } else if normalized.contains("/pictures/") || normalized.contains("/music/") {
        0.35
    } else if normalized.contains("/documents/") || normalized.contains("/downloads/") {
        0.20
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_eight_normalized_features() {
        let features =
            extract_features(Path::new(r"C:\repo\node_modules\pkg\index"), 1024, b"hello");

        assert_eq!(features.as_array().len(), 8);
        assert_eq!(features.has_extension, 0.0);
        assert!(features.parent_dir_type > 0.9);
        assert!(features.printable_ratio > 0.9);
    }
}
