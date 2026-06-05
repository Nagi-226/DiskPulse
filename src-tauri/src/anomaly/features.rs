use crate::db;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SnapshotFeatures {
    pub usage_ratio: f64,
    pub free_ratio: f64,
    pub growth_ratio: f64,
    pub volatility_ratio: f64,
    pub hotspot_ratio: f64,
    pub sample_density: f64,
}

impl SnapshotFeatures {
    pub fn as_array(&self) -> [f64; 6] {
        [
            self.usage_ratio,
            self.free_ratio,
            self.growth_ratio,
            self.volatility_ratio,
            self.hotspot_ratio,
            self.sample_density,
        ]
    }
}

pub fn extract_snapshot_features(history: &[db::Snapshot]) -> SnapshotFeatures {
    let Some(latest) = history.last() else {
        return SnapshotFeatures {
            usage_ratio: 0.0,
            free_ratio: 1.0,
            growth_ratio: 0.0,
            volatility_ratio: 0.0,
            hotspot_ratio: 0.0,
            sample_density: 0.0,
        };
    };

    let total = latest.total_bytes.max(1) as f64;
    let usage_ratio = latest.used_bytes as f64 / total;
    let free_ratio = latest.free_bytes as f64 / total;
    let growths: Vec<f64> = history
        .windows(2)
        .map(|pair| pair[1].used_bytes as f64 - pair[0].used_bytes as f64)
        .collect();
    let growth_ratio = growths.last().copied().unwrap_or_default().abs() / total;
    let volatility_ratio = standard_deviation(&growths) / total;
    let hotspot_ratio = parse_largest_dir_ratio(&latest.snapshot_json, total);

    SnapshotFeatures {
        usage_ratio: usage_ratio.clamp(0.0, 1.0),
        free_ratio: free_ratio.clamp(0.0, 1.0),
        growth_ratio: growth_ratio.clamp(0.0, 1.0),
        volatility_ratio: volatility_ratio.clamp(0.0, 1.0),
        hotspot_ratio: hotspot_ratio.clamp(0.0, 1.0),
        sample_density: (history.len().min(60) as f64 / 60.0).clamp(0.0, 1.0),
    }
}

fn parse_largest_dir_ratio(json: &str, total: f64) -> f64 {
    serde_json::from_str::<Vec<crate::scanner::DirInfo>>(json)
        .unwrap_or_default()
        .into_iter()
        .map(|dir| dir.size_bytes as f64 / total)
        .fold(0.0, f64::max)
}

fn standard_deviation(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / (values.len() - 1) as f64;
    variance.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    const GIB: u64 = 1024 * 1024 * 1024;

    fn snapshot(id: i64, used_gib: u64, json: &str) -> db::Snapshot {
        let total = 100 * GIB;
        db::Snapshot {
            id,
            drive_letter: "C".into(),
            total_bytes: total,
            used_bytes: used_gib * GIB,
            free_bytes: total - used_gib * GIB,
            snapshot_json: json.into(),
            created_at: format!("2026-06-{id:02} 00:00:00"),
        }
    }

    #[test]
    fn extracts_six_snapshot_features() {
        let json = r#"[{"name":"Cache","path":"C:\\Cache","size_bytes":10737418240,"file_count":1,"dir_count":0,"risk_level":null,"is_approximate":false}]"#;
        let features = extract_snapshot_features(&[snapshot(1, 50, "[]"), snapshot(2, 55, json)]);

        assert_eq!(features.as_array().len(), 6);
        assert!((features.usage_ratio - 0.55).abs() < 0.01);
        assert!(features.hotspot_ratio > 0.09);
    }
}
