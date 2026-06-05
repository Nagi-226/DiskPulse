use super::FileCategory;

pub fn classify_magic(bytes: &[u8]) -> FileCategory {
    if bytes.starts_with(b"%PDF") {
        FileCategory::Pdf
    } else if bytes.starts_with(&[0x89, b'P', b'N', b'G']) || bytes.starts_with(&[0xff, 0xd8, 0xff])
    {
        FileCategory::MediaImage
    } else if bytes.starts_with(b"ID3") || bytes.starts_with(b"fLaC") {
        FileCategory::MediaAudio
    } else if bytes.get(4..8) == Some(b"ftyp") {
        FileCategory::MediaVideo
    } else if bytes.starts_with(b"PK\x03\x04")
        || bytes.starts_with(&[0x1f, 0x8b])
        || bytes.starts_with(b"7z\xbc\xaf\x27\x1c")
    {
        FileCategory::ArchiveCompressed
    } else if bytes.starts_with(b"MZ") {
        FileCategory::Installer
    } else if bytes.starts_with(b"SQLite format 3") {
        FileCategory::SystemDb
    } else {
        FileCategory::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_extended_signatures() {
        assert_eq!(classify_magic(b"SQLite format 3\0"), FileCategory::SystemDb);
        assert_eq!(classify_magic(b"ID3\x04\x00\x00"), FileCategory::MediaAudio);
        assert_eq!(
            classify_magic(b"\0\0\0\x18ftypmp42"),
            FileCategory::MediaVideo
        );
    }
}
