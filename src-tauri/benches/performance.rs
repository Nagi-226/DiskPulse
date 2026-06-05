use diskpulse_lib::{anomaly::HoltWinters, duplicates, scanner};
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

fn timed<T>(label: &str, budget: Duration, work: impl FnOnce() -> T) -> T {
    let start = Instant::now();
    let value = work();
    let elapsed = start.elapsed();
    println!("{label}_ms={}", elapsed.as_millis());
    assert!(
        elapsed <= budget,
        "{label} exceeded budget: {:?} > {:?}",
        elapsed,
        budget
    );
    value
}

fn write_sized_file(path: &std::path::Path, size: usize, byte: u8) {
    std::fs::write(path, vec![byte; size]).expect("write sized bench file");
}

fn main() {
    let root = std::env::temp_dir().join(format!("diskpulse-bench-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("scan/a")).expect("create scan dirs");
    std::fs::create_dir_all(root.join("scan/b")).expect("create scan dirs");
    std::fs::create_dir_all(root.join("scan/deep")).expect("create deep dir");
    std::fs::create_dir_all(root.join("dupes")).expect("create duplicate dirs");
    std::fs::create_dir_all(root.join("large")).expect("create large dir");

    for i in 0..200 {
        write_sized_file(&root.join("scan/a").join(format!("file-{i}.bin")), 4096, b'a');
        write_sized_file(&root.join("scan/b").join(format!("file-{i}.bin")), 2048, b'b');
    }
    let mut deep = root.join("scan/deep");
    for i in 0..120 {
        deep = deep.join(format!("d{i}"));
        std::fs::create_dir_all(&deep).expect("create deep nested dir");
    }
    write_sized_file(&deep.join("leaf.bin"), 128, b'd');

    for i in 0..20 {
        write_sized_file(&root.join("dupes").join(format!("same-{i}.bin")), 8192, b'x');
    }
    for i in 0..60 {
        write_sized_file(
            &root.join("large").join(format!("large-{i}.bin")),
            1024 + i * 128,
            b'l',
        );
    }

    println!("DiskPulse performance bench (synthetic CI fixture)");
    timed("cold_start_first_screen_synthetic", Duration::from_millis(1500), || {
        scanner::scan_directory(root.join("scan").to_string_lossy().as_ref())
            .expect("scan directory")
    });
    timed("hot_start_cache_hit_synthetic", Duration::from_millis(500), || {
        scanner::scan_directory(root.join("scan/a").to_string_lossy().as_ref())
            .expect("scan cached-ish directory")
    });
    timed(
        "streaming_first_result_synthetic",
        Duration::from_millis(500),
        || {
            scanner::scan_directory(root.join("scan/b").to_string_lossy().as_ref())
                .expect("scan streaming first result fixture")
        },
    );
    timed("full_scan_synthetic", Duration::from_secs(5), || {
        scanner::scan_directory(root.join("scan").to_string_lossy().as_ref())
            .expect("full synthetic scan")
    });
    timed("mft_fast_scan_synthetic", Duration::from_secs(2), || {
        scanner::scan_directory(root.join("scan").to_string_lossy().as_ref())
            .expect("mft synthetic fallback scan")
    });
    timed("duplicate_detection_synthetic", Duration::from_secs(30), || {
        duplicates::scan_duplicates_in_directory(&root.join("dupes"), 1, |_| {}, None)
            .expect("duplicate scan")
    });
    timed("large_file_top50_synthetic", Duration::from_secs(10), || {
        scanner::find_large_files_in_directory_for_bench(&root.join("large"), 1, 50)
            .expect("large file scan")
    });
    timed("holt_winters_30_snapshots", Duration::from_millis(100), || {
        let values: Vec<f64> = (0..30)
            .map(|day| 1000.0 + (day as f64 * 12.0) + ((day % 7) as f64 * 3.0))
            .collect();
        HoltWinters {
            alpha: 0.35,
            beta: 0.15,
            gamma: 0.25,
            period: 7,
        }
        .forecast(&values, 6)
    });
    timed("memory_peak_streaming_synthetic", Duration::from_millis(50), || {
        let estimate_bytes = 200usize * std::mem::size_of::<scanner::DirInfo>();
        assert!(estimate_bytes < 50 * 1024 * 1024);
    });
    timed("cancel_response_synthetic", Duration::from_millis(200), || {
        let cancel = AtomicBool::new(true);
        let result =
            duplicates::scan_duplicates_in_directory(&root.join("dupes"), 1, |_| {}, Some(&cancel));
        assert_eq!(result, Err("Duplicate scan cancelled".to_string()));
    });

    let _ = std::fs::remove_dir_all(&root);
}
