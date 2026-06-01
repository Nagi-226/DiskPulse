pub trait DiskInfoProvider {
    fn free_percent(&self, drive: &str) -> Result<f64, String>;
}

pub struct WindowsDiskInfoProvider;

impl DiskInfoProvider for WindowsDiskInfoProvider {
    fn free_percent(&self, drive: &str) -> Result<f64, String> {
        let meta = crate::scanner::scan_drive_meta(drive, None, None)?;
        if meta.total_bytes == 0 {
            return Ok(0.0);
        }
        Ok((meta.free_bytes as f64 / meta.total_bytes as f64) * 100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeProvider;

    impl DiskInfoProvider for FakeProvider {
        fn free_percent(&self, _drive: &str) -> Result<f64, String> {
            Ok(42.0)
        }
    }

    #[test]
    fn disk_info_provider_trait_is_swappable() {
        let provider = FakeProvider;
        assert_eq!(provider.free_percent("C").unwrap(), 42.0);
    }
}
