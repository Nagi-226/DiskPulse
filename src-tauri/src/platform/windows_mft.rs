#[cfg(all(target_os = "windows", feature = "mft-scanner"))]
pub struct MftStage;

#[cfg(all(target_os = "windows", feature = "mft-scanner"))]
impl crate::platform::DirScanner for MftStage {
    fn name(&self) -> &'static str {
        "mft"
    }

    fn execute(
        &self,
        _ctx: &crate::scanner::ScanContext<'_>,
    ) -> Result<crate::scanner::ScanOutput, String> {
        Err("MFT scanner is a technical reserve and is not wired yet".into())
    }
}
