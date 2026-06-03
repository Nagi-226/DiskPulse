#[cfg(all(target_os = "windows", feature = "mft-scanner"))]
use std::collections::HashMap;
#[cfg(all(target_os = "windows", feature = "mft-scanner"))]
use std::mem::size_of;
#[cfg(all(target_os = "windows", feature = "mft-scanner"))]
use std::path::Path;
#[cfg(all(target_os = "windows", feature = "mft-scanner"))]
use std::sync::mpsc::{self, Receiver};

#[cfg(all(target_os = "windows", feature = "mft-scanner"))]
use windows::core::PCWSTR;
#[cfg(all(target_os = "windows", feature = "mft-scanner"))]
use windows::Win32::Foundation::{CloseHandle, HANDLE};
#[cfg(all(target_os = "windows", feature = "mft-scanner"))]
use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
#[cfg(all(target_os = "windows", feature = "mft-scanner"))]
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_DELETE,
    FILE_SHARE_MODE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
#[cfg(all(target_os = "windows", feature = "mft-scanner"))]
use windows::Win32::System::Ioctl::{
    FSCTL_ENUM_USN_DATA, FSCTL_GET_NTFS_VOLUME_DATA, MFT_ENUM_DATA_V0, NTFS_VOLUME_DATA_BUFFER,
};
#[cfg(all(target_os = "windows", feature = "mft-scanner"))]
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
#[cfg(all(target_os = "windows", feature = "mft-scanner"))]
use windows::Win32::System::IO::DeviceIoControl;

#[cfg(all(target_os = "windows", feature = "mft-scanner"))]
pub struct MftStage;

#[cfg(all(target_os = "windows", feature = "mft-scanner"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedUsnRecord {
    pub file_reference_number: u64,
    pub parent_file_reference_number: u64,
    pub file_attributes: u32,
    pub name: String,
}

#[cfg(all(target_os = "windows", feature = "mft-scanner"))]
impl ParsedUsnRecord {
    fn is_dir(&self) -> bool {
        self.file_attributes & FILE_ATTRIBUTE_DIRECTORY.0 != 0
    }
}

#[cfg(all(target_os = "windows", feature = "mft-scanner"))]
struct VolumeHandle(HANDLE);

#[cfg(all(target_os = "windows", feature = "mft-scanner"))]
impl Drop for VolumeHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

#[cfg(all(target_os = "windows", feature = "mft-scanner"))]
impl crate::platform::DirScanner for MftStage {
    fn name(&self) -> &'static str {
        "mft"
    }

    fn is_volume_streaming(&self) -> bool {
        true
    }

    fn execute(
        &self,
        ctx: &crate::scanner::ScanContext<'_>,
    ) -> Result<crate::scanner::ScanOutput, String> {
        if drive_letter_from_root(&ctx.root).is_err() {
            let stage = crate::scanner::MeasureStage;
            return crate::scanner::ScanStage::execute(&stage, ctx);
        }
        let dirs = scan_volume_top_dirs(&ctx.root, ctx.cancel)
            .or_else(|_| fallback_jwalk_top_dirs(&ctx.root, ctx.cancel))?;
        Ok(crate::scanner::ScanOutput {
            size_bytes: dirs.iter().map(|dir| dir.size_bytes).sum(),
            file_count: dirs.iter().map(|dir| dir.file_count).sum(),
            dir_count: dirs.iter().map(|dir| dir.dir_count).sum(),
        })
    }

    fn execute_streaming(
        &self,
        ctx: &crate::scanner::ScanContext<'_>,
    ) -> Receiver<crate::scanner::ScanBatch> {
        let (sender, receiver) = mpsc::channel();
        if drive_letter_from_root(&ctx.root).is_err() {
            let stage = crate::scanner::MeasureStage;
            return crate::scanner::ScanStage::execute_streaming(&stage, ctx);
        }
        match scan_volume_top_dirs(&ctx.root, ctx.cancel)
            .or_else(|_| fallback_jwalk_top_dirs(&ctx.root, ctx.cancel))
        {
            Ok(dirs) => {
                let completion_index = dirs.len() as u32;
                for (index, dir) in dirs.into_iter().enumerate() {
                    let _ = sender.send(crate::scanner::ScanBatch {
                        dirs: vec![dir],
                        batch_index: index as u32,
                        is_complete: false,
                    });
                }
                let _ = sender.send(crate::scanner::ScanBatch {
                    dirs: Vec::new(),
                    batch_index: completion_index,
                    is_complete: true,
                });
            }
            Err(_) => {
                // Empty completion lets callers fall back gracefully without blocking.
                let _ = sender.send(crate::scanner::ScanBatch {
                    dirs: Vec::new(),
                    batch_index: 0,
                    is_complete: true,
                });
            }
        }
        receiver
    }
}

#[cfg(all(target_os = "windows", feature = "mft-scanner"))]
pub fn check_admin_privilege() -> bool {
    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }
        let token_guard = VolumeHandle(token);
        let mut elevation = TOKEN_ELEVATION::default();
        let mut returned = 0u32;
        let ok = GetTokenInformation(
            token_guard.0,
            TokenElevation,
            Some((&mut elevation as *mut TOKEN_ELEVATION).cast()),
            size_of::<TOKEN_ELEVATION>() as u32,
            &mut returned,
        )
        .is_ok();
        ok && elevation.TokenIsElevated != 0
    }
}

#[cfg(all(target_os = "windows", feature = "mft-scanner"))]
fn scan_volume_top_dirs(
    root: &Path,
    cancel: Option<&std::sync::atomic::AtomicBool>,
) -> Result<Vec<crate::scanner::DirInfo>, String> {
    if crate::scanner::is_cancelled(cancel) {
        return Err("Scan cancelled".into());
    }
    let drive = drive_letter_from_root(root)?;
    let records = enumerate_usn_records(&drive, cancel)?;
    let dirs = records_to_top_dirs(&drive, &records);
    if dirs.is_empty() {
        return Err("MFT scan returned no top-level directories".into());
    }
    Ok(dirs)
}

#[cfg(all(target_os = "windows", feature = "mft-scanner"))]
fn fallback_jwalk_top_dirs(
    root: &Path,
    cancel: Option<&std::sync::atomic::AtomicBool>,
) -> Result<Vec<crate::scanner::DirInfo>, String> {
    let entries = std::fs::read_dir(root).map_err(|e| format!("Cannot read root: {e}"))?;
    let mut dirs = Vec::new();
    let stage = crate::scanner::MeasureStage;

    for entry in entries.flatten() {
        if crate::scanner::is_cancelled(cancel) {
            return Err("Scan cancelled".into());
        }
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let is_symlink = entry.file_type().map(|ft| ft.is_symlink()).unwrap_or(false);
        if !path.is_dir()
            || is_symlink
            || matches!(name.as_str(), "System Volume Information" | "$Recycle.Bin")
        {
            continue;
        }
        let output = crate::scanner::ScanStage::execute(
            &stage,
            &crate::scanner::ScanContext {
                root: path.clone(),
                cancel,
            },
        )?;
        dirs.push(crate::scanner::DirInfo {
            name,
            path: path.to_string_lossy().to_string(),
            size_bytes: output.size_bytes,
            file_count: output.file_count,
            dir_count: output.dir_count,
            risk_level: None,
            is_approximate: false,
        });
    }

    dirs.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
    Ok(dirs)
}

#[cfg(all(target_os = "windows", feature = "mft-scanner"))]
fn drive_letter_from_root(root: &Path) -> Result<char, String> {
    let text = root.to_string_lossy();
    let mut chars = text.chars();
    let drive = chars
        .next()
        .filter(|ch| ch.is_ascii_alphabetic())
        .ok_or_else(|| format!("MFT scan requires a drive root, got {}", root.display()))?;
    if chars.next() != Some(':') {
        return Err(format!(
            "MFT scan requires a drive root, got {}",
            root.display()
        ));
    }
    let rest: String = chars.collect();
    if !rest.is_empty() && rest.trim_matches(['\\', '/']).len() > 0 {
        return Err(format!(
            "MFT scan requires a drive root, got {}",
            root.display()
        ));
    }
    Ok(drive.to_ascii_uppercase())
}

#[cfg(all(target_os = "windows", feature = "mft-scanner"))]
fn enumerate_usn_records(
    drive: &char,
    cancel: Option<&std::sync::atomic::AtomicBool>,
) -> Result<Vec<ParsedUsnRecord>, String> {
    let handle = open_volume(*drive)?;
    let _ = read_ntfs_volume_data(handle.0);
    let mut enum_data = MFT_ENUM_DATA_V0 {
        StartFileReferenceNumber: 0,
        LowUsn: 0,
        HighUsn: i64::MAX,
    };
    let mut output = vec![0u8; 1024 * 1024];
    let mut records = Vec::new();

    loop {
        if crate::scanner::is_cancelled(cancel) {
            return Err("Scan cancelled".into());
        }

        let mut returned = 0u32;
        let result = unsafe {
            DeviceIoControl(
                handle.0,
                FSCTL_ENUM_USN_DATA,
                Some((&mut enum_data as *mut MFT_ENUM_DATA_V0).cast()),
                size_of::<MFT_ENUM_DATA_V0>() as u32,
                Some(output.as_mut_ptr().cast()),
                output.len() as u32,
                Some(&mut returned),
                None,
            )
        };

        if result.is_err() {
            if records.is_empty() {
                return Err(format!("FSCTL_ENUM_USN_DATA failed: {:?}", result.err()));
            }
            break;
        }
        if returned as usize <= size_of::<u64>() {
            break;
        }

        let returned = returned as usize;
        enum_data.StartFileReferenceNumber =
            u64::from_ne_bytes(output[..size_of::<u64>()].try_into().unwrap());
        records.extend(parse_usn_records(&output[size_of::<u64>()..returned]));
    }

    Ok(records)
}

#[cfg(all(target_os = "windows", feature = "mft-scanner"))]
fn open_volume(drive: char) -> Result<VolumeHandle, String> {
    let path = format!(r"\\.\{}:", drive);
    let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    let share = FILE_SHARE_MODE(FILE_SHARE_READ.0 | FILE_SHARE_WRITE.0 | FILE_SHARE_DELETE.0);
    let handle = unsafe {
        CreateFileW(
            PCWSTR(wide.as_ptr()),
            0,
            share,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            HANDLE::default(),
        )
    }
    .map_err(|e| format!("Open volume {} failed: {e}", path))?;
    Ok(VolumeHandle(handle))
}

#[cfg(all(target_os = "windows", feature = "mft-scanner"))]
fn read_ntfs_volume_data(handle: HANDLE) -> Result<NTFS_VOLUME_DATA_BUFFER, String> {
    let mut data = NTFS_VOLUME_DATA_BUFFER::default();
    let mut returned = 0u32;
    unsafe {
        DeviceIoControl(
            handle,
            FSCTL_GET_NTFS_VOLUME_DATA,
            None,
            0,
            Some((&mut data as *mut NTFS_VOLUME_DATA_BUFFER).cast()),
            size_of::<NTFS_VOLUME_DATA_BUFFER>() as u32,
            Some(&mut returned),
            None,
        )
    }
    .map_err(|e| format!("FSCTL_GET_NTFS_VOLUME_DATA failed: {e}"))?;
    Ok(data)
}

#[cfg(all(target_os = "windows", feature = "mft-scanner"))]
pub fn parse_usn_records(buffer: &[u8]) -> Vec<ParsedUsnRecord> {
    let mut records = Vec::new();
    let mut offset = 0usize;

    while offset + 60 <= buffer.len() {
        let record_len =
            u32::from_ne_bytes(buffer[offset..offset + 4].try_into().unwrap()) as usize;
        if record_len == 0 || offset + record_len > buffer.len() {
            break;
        }
        if let Some(record) = parse_usn_record(&buffer[offset..offset + record_len]) {
            records.push(record);
        }
        offset += record_len;
    }

    records
}

#[cfg(all(target_os = "windows", feature = "mft-scanner"))]
fn parse_usn_record(record: &[u8]) -> Option<ParsedUsnRecord> {
    let major = u16::from_ne_bytes(record.get(4..6)?.try_into().ok()?);
    if major != 2 {
        return None;
    }

    let file_reference_number = u64::from_ne_bytes(record.get(8..16)?.try_into().ok()?);
    let parent_file_reference_number = u64::from_ne_bytes(record.get(16..24)?.try_into().ok()?);
    let file_attributes = u32::from_ne_bytes(record.get(52..56)?.try_into().ok()?);
    let name_len = u16::from_ne_bytes(record.get(56..58)?.try_into().ok()?) as usize;
    let name_offset = u16::from_ne_bytes(record.get(58..60)?.try_into().ok()?) as usize;
    let name_end = name_offset.checked_add(name_len)?;
    let name_bytes = record.get(name_offset..name_end)?;
    if name_bytes.len() % 2 != 0 {
        return None;
    }
    let name_utf16: Vec<u16> = name_bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_ne_bytes([chunk[0], chunk[1]]))
        .collect();
    Some(ParsedUsnRecord {
        file_reference_number,
        parent_file_reference_number,
        file_attributes,
        name: String::from_utf16_lossy(&name_utf16),
    })
}

#[cfg(all(target_os = "windows", feature = "mft-scanner"))]
fn records_to_top_dirs(drive: &char, records: &[ParsedUsnRecord]) -> Vec<crate::scanner::DirInfo> {
    let mut by_frn: HashMap<u64, &ParsedUsnRecord> = HashMap::new();
    for record in records {
        by_frn.insert(record.file_reference_number, record);
    }

    let mut totals: HashMap<u64, crate::scanner::DirInfo> = HashMap::new();
    for record in records {
        if record.file_reference_number == record.parent_file_reference_number {
            continue;
        }
        let Some(top) = top_level_dir(record, &by_frn) else {
            continue;
        };
        let entry = totals.entry(top.file_reference_number).or_insert_with(|| {
            crate::scanner::DirInfo {
                name: top.name.clone(),
                path: format!("{}:\\{}", drive, top.name),
                // USN enumeration does not expose logical file sizes; use zero and mark approximate in UI later.
                size_bytes: 0,
                file_count: 0,
                dir_count: 0,
                risk_level: None,
                is_approximate: true,
            }
        });
        if record.is_dir() {
            if record.file_reference_number != top.file_reference_number {
                entry.dir_count += 1;
            }
        } else {
            entry.file_count += 1;
        }
    }

    let mut dirs: Vec<_> = totals.into_values().collect();
    dirs.sort_by(|a, b| {
        b.file_count
            .cmp(&a.file_count)
            .then_with(|| a.name.cmp(&b.name))
    });
    dirs
}

#[cfg(all(target_os = "windows", feature = "mft-scanner"))]
fn top_level_dir<'a>(
    record: &'a ParsedUsnRecord,
    by_frn: &HashMap<u64, &'a ParsedUsnRecord>,
) -> Option<&'a ParsedUsnRecord> {
    let mut current = if record.is_dir() {
        record
    } else {
        *by_frn.get(&record.parent_file_reference_number)?
    };

    for _ in 0..128 {
        let Some(parent) = by_frn.get(&current.parent_file_reference_number).copied() else {
            return if current.is_dir() {
                Some(current)
            } else {
                None
            };
        };
        if parent.parent_file_reference_number == parent.file_reference_number {
            return Some(current);
        }
        current = parent;
    }
    None
}

#[cfg(all(test, target_os = "windows", feature = "mft-scanner"))]
mod tests {
    use super::*;

    fn synthetic_v2_record(frn: u64, parent: u64, attrs: u32, name: &str) -> Vec<u8> {
        let name: Vec<u16> = name.encode_utf16().collect();
        let record_len = 60 + name.len() * 2;
        let mut bytes = vec![0u8; record_len];
        bytes[0..4].copy_from_slice(&(record_len as u32).to_ne_bytes());
        bytes[4..6].copy_from_slice(&2u16.to_ne_bytes());
        bytes[8..16].copy_from_slice(&frn.to_ne_bytes());
        bytes[16..24].copy_from_slice(&parent.to_ne_bytes());
        bytes[52..56].copy_from_slice(&attrs.to_ne_bytes());
        bytes[56..58].copy_from_slice(&((name.len() * 2) as u16).to_ne_bytes());
        bytes[58..60].copy_from_slice(&60u16.to_ne_bytes());
        for (index, ch) in name.iter().enumerate() {
            bytes[60 + index * 2..62 + index * 2].copy_from_slice(&ch.to_ne_bytes());
        }
        bytes
    }

    #[test]
    fn parses_usn_record_v2_name_and_parent() {
        let bytes = synthetic_v2_record(42, 5, FILE_ATTRIBUTE_DIRECTORY.0, "Users");
        let records = parse_usn_records(&bytes);

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].file_reference_number, 42);
        assert_eq!(records[0].parent_file_reference_number, 5);
        assert_eq!(records[0].name, "Users");
        assert!(records[0].is_dir());
    }

    #[test]
    fn aggregates_records_into_top_level_dirs() {
        let root = ParsedUsnRecord {
            file_reference_number: 1,
            parent_file_reference_number: 1,
            file_attributes: FILE_ATTRIBUTE_DIRECTORY.0,
            name: ".".into(),
        };
        let users = ParsedUsnRecord {
            file_reference_number: 2,
            parent_file_reference_number: 1,
            file_attributes: FILE_ATTRIBUTE_DIRECTORY.0,
            name: "Users".into(),
        };
        let alice = ParsedUsnRecord {
            file_reference_number: 3,
            parent_file_reference_number: 2,
            file_attributes: FILE_ATTRIBUTE_DIRECTORY.0,
            name: "alice".into(),
        };
        let file = ParsedUsnRecord {
            file_reference_number: 4,
            parent_file_reference_number: 3,
            file_attributes: 0,
            name: "payload.bin".into(),
        };

        let dirs = records_to_top_dirs(&'C', &[root, users, alice, file]);

        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, "Users");
        assert_eq!(dirs[0].path, r"C:\Users");
        assert_eq!(dirs[0].file_count, 1);
        assert_eq!(dirs[0].dir_count, 1);
    }

    #[test]
    fn admin_privilege_check_is_callable() {
        let _ = check_admin_privilege();
    }
}
