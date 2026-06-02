use diskpulse_lib::{duplicates, scanner};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

fn main() {
    let root = std::env::temp_dir().join(format!("diskpulse-bench-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("scan/a")).expect("create scan dirs");
    std::fs::create_dir_all(root.join("scan/b")).expect("create scan dirs");
    std::fs::create_dir_all(root.join("dupes")).expect("create duplicate dirs");

    for i in 0..200 {
        std::fs::write(
            root.join("scan/a").join(format!("file-{i}.bin")),
            vec![b'a'; 4096],
        )
        .expect("write scan file");
        std::fs::write(
            root.join("scan/b").join(format!("file-{i}.bin")),
            vec![b'b'; 2048],
        )
        .expect("write scan file");
    }
    for i in 0..20 {
        std::fs::write(
            root.join("dupes").join(format!("same-{i}.bin")),
            vec![b'x'; 8192],
        )
        .expect("write duplicate file");
    }

    let scan_start = Instant::now();
    let scan_dirs = scanner::scan_directory(root.join("scan").to_string_lossy().as_ref())
        .expect("scan directory");
    let scan_elapsed = scan_start.elapsed();

    let duplicate_start = Instant::now();
    let duplicate_groups =
        duplicates::scan_duplicates_in_directory(&root.join("dupes"), 1, |_| {}, None)
            .expect("duplicate scan");
    let duplicate_elapsed = duplicate_start.elapsed();

    let cancel = AtomicBool::new(true);
    let cancel_start = Instant::now();
    let cancel_result =
        duplicates::scan_duplicates_in_directory(&root.join("dupes"), 1, |_| {}, Some(&cancel));
    let cancel_elapsed = cancel_start.elapsed();
    assert_eq!(cancel_result, Err("Duplicate scan cancelled".to_string()));
    assert!(cancel_elapsed < Duration::from_millis(500));

    println!("DiskPulse performance bench (synthetic local dataset)");
    println!("full_scan_synthetic_ms={}", scan_elapsed.as_millis());
    println!("full_scan_dirs={}", scan_dirs.len());
    println!(
        "duplicate_detection_synthetic_ms={}",
        duplicate_elapsed.as_millis()
    );
    println!("duplicate_groups={}", duplicate_groups.len());
    println!("cancel_response_ms={}", cancel_elapsed.as_millis());

    cancel.store(false, Ordering::Relaxed);
    let _ = std::fs::remove_dir_all(&root);
}
