pub mod ae;
pub mod features;
pub mod synthetic;

use crate::db;
use crate::scanner::DirInfo;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnomalyType {
    RateAnomaly,
    BurstAnomaly,
    HotspotAnomaly,
    PatternDeviation,
    DriftAnomaly,
    AntiSeasonal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnomalySeverity {
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyEvent {
    pub anomaly_type: AnomalyType,
    pub severity: AnomalySeverity,
    pub drive_letter: String,
    pub created_at: String,
    pub metric_value: f64,
    pub modified_z_score: f64,
    pub description: String,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyReport {
    pub drive_letter: String,
    pub sample_count: usize,
    pub events: Vec<AnomalyEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AeStatus {
    Healthy,
    Degraded,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FusionWeights {
    pub holt_winters: f64,
    pub zscore: f64,
    pub autoencoder: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FusionSignal {
    pub name: String,
    pub triggered: bool,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FusionResult {
    pub weights: FusionWeights,
    pub confidence: String,
    pub fused_score: f64,
    pub signals: Vec<FusionSignal>,
}

#[derive(Debug, Clone, Copy)]
pub struct HoltWinters {
    pub alpha: f64,
    pub beta: f64,
    pub gamma: f64,
    pub period: usize,
}

#[derive(Debug, Clone)]
pub struct HoltWintersResult {
    pub forecast: Vec<f64>,
    pub trend: f64,
    pub seasonal: f64,
    pub residuals: Vec<f64>,
    pub confidence_interval: Option<(f64, f64)>,
}

impl HoltWinters {
    pub fn forecast(&self, values: &[f64], steps: usize) -> HoltWintersResult {
        if values.is_empty() {
            return HoltWintersResult {
                forecast: Vec::new(),
                trend: 0.0,
                seasonal: 0.0,
                residuals: Vec::new(),
                confidence_interval: None,
            };
        }

        let period = self.period.max(1).min(values.len());
        let alpha = self.alpha.clamp(0.0, 1.0);
        let beta = self.beta.clamp(0.0, 1.0);
        let gamma = self.gamma.clamp(0.0, 1.0);
        let first_level = values.iter().take(period).sum::<f64>() / period as f64;
        let second_level = if values.len() >= period * 2 {
            values.iter().skip(period).take(period).sum::<f64>() / period as f64
        } else {
            values[values.len() - 1]
        };
        let mut level = first_level;
        let mut trend = if values.len() >= period * 2 {
            (second_level - first_level) / period as f64
        } else if values.len() > 1 {
            (values[values.len() - 1] - values[0]) / (values.len() - 1) as f64
        } else {
            0.0
        };
        let mut seasonal: Vec<f64> = (0..period)
            .map(|i| values.get(i).copied().unwrap_or(first_level) - first_level)
            .collect();
        let mut residuals = Vec::with_capacity(values.len());

        for (i, value) in values.iter().copied().enumerate() {
            let slot = i % period;
            let season = seasonal[slot];
            let predicted = level + trend + season;
            residuals.push(value - predicted);

            let previous_level = level;
            level = alpha * (value - season) + (1.0 - alpha) * (level + trend);
            trend = beta * (level - previous_level) + (1.0 - beta) * trend;
            seasonal[slot] = gamma * (value - level) + (1.0 - gamma) * season;
        }

        let forecast: Vec<f64> = (1..=steps)
            .map(|m| {
                let season = seasonal[(values.len() + m - 1) % period];
                level + m as f64 * trend + season
            })
            .collect();
        let residual_std = standard_deviation(&residuals);
        let confidence_interval = forecast.first().map(|next| {
            let width = residual_std * 1.96;
            (next - width, next + width)
        });
        let seasonal_component = seasonal[(values.len().saturating_sub(1)) % period];

        HoltWintersResult {
            forecast,
            trend,
            seasonal: seasonal_component,
            residuals,
            confidence_interval,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ModifiedZScore {
    pub threshold: f64,
}

impl ModifiedZScore {
    pub fn scores(&self, values: &[f64]) -> Vec<f64> {
        if values.is_empty() {
            return Vec::new();
        }

        let center = median(values);
        let deviations: Vec<f64> = values.iter().map(|value| (value - center).abs()).collect();
        let mad = median(&deviations);

        values
            .iter()
            .map(|value| {
                if mad <= f64::EPSILON {
                    let diff = value - center;
                    if diff.abs() <= f64::EPSILON {
                        0.0
                    } else {
                        diff.signum() * f64::INFINITY
                    }
                } else {
                    0.6745 * (value - center) / mad
                }
            })
            .collect()
    }
}

pub struct AnomalyDetector {
    pub holt_winters: HoltWinters,
    pub modified_z_score: ModifiedZScore,
}

impl Default for AnomalyDetector {
    fn default() -> Self {
        Self {
            holt_winters: HoltWinters {
                alpha: 0.35,
                beta: 0.15,
                gamma: 0.25,
                period: 7,
            },
            modified_z_score: ModifiedZScore { threshold: 3.5 },
        }
    }
}

pub fn fusion_weights(status: AeStatus) -> FusionWeights {
    match status {
        AeStatus::Healthy => FusionWeights {
            holt_winters: 0.30,
            zscore: 0.30,
            autoencoder: 0.40,
        },
        AeStatus::Degraded => FusionWeights {
            holt_winters: 0.45,
            zscore: 0.45,
            autoencoder: 0.10,
        },
        AeStatus::Disabled => FusionWeights {
            holt_winters: 0.50,
            zscore: 0.50,
            autoencoder: 0.0,
        },
    }
}

pub fn fuse_anomaly_signals(status: AeStatus, signals: Vec<FusionSignal>) -> FusionResult {
    let weights = fusion_weights(status);
    let weight_for = |name: &str| match name {
        "holt_winters" => weights.holt_winters,
        "zscore" => weights.zscore,
        "autoencoder" => weights.autoencoder,
        _ => 0.0,
    };
    let fused_score = signals
        .iter()
        .filter(|signal| signal.triggered)
        .map(|signal| signal.score.clamp(0.0, 1.0) * weight_for(&signal.name))
        .sum::<f64>()
        .clamp(0.0, 1.0);
    let triggered = signals.iter().filter(|signal| signal.triggered).count();
    let confidence = if triggered >= 2 {
        "high"
    } else if triggered == 1 {
        "low"
    } else {
        "none"
    }
    .to_string();

    FusionResult {
        weights,
        confidence,
        fused_score,
        signals,
    }
}

impl AnomalyDetector {
    pub fn detect(&self, history: &[db::Snapshot]) -> Vec<AnomalyEvent> {
        let mut sorted = history.to_vec();
        sorted.sort_by(|a, b| a.created_at.cmp(&b.created_at));

        let mut events = Vec::new();
        self.detect_rate_anomalies(&sorted, &mut events);
        self.detect_hotspots(&sorted, &mut events);
        self.detect_pattern_deviation(&sorted, &mut events);
        events
    }

    fn detect_rate_anomalies(&self, history: &[db::Snapshot], events: &mut Vec<AnomalyEvent>) {
        if history.len() < 3 {
            return;
        }

        let rates: Vec<f64> = history
            .windows(2)
            .map(|pair| pair[1].used_bytes as f64 - pair[0].used_bytes as f64)
            .collect();
        let scores = self.modified_z_score.scores(&rates);

        for ((snapshot, rate), score) in history.iter().skip(1).zip(rates.iter()).zip(scores.iter())
        {
            let abs_score = score.abs();
            if abs_score > 10.0 {
                events.push(AnomalyEvent {
                    anomaly_type: AnomalyType::BurstAnomaly,
                    severity: AnomalySeverity::Critical,
                    drive_letter: snapshot.drive_letter.clone(),
                    created_at: snapshot.created_at.clone(),
                    metric_value: *rate,
                    modified_z_score: *score,
                    description: format!(
                        "Burst growth detected: {} changed in one sample.",
                        format_bytes(rate.abs())
                    ),
                    path: None,
                });
            } else if abs_score > self.modified_z_score.threshold {
                events.push(AnomalyEvent {
                    anomaly_type: AnomalyType::RateAnomaly,
                    severity: AnomalySeverity::Warning,
                    drive_letter: snapshot.drive_letter.clone(),
                    created_at: snapshot.created_at.clone(),
                    metric_value: *rate,
                    modified_z_score: *score,
                    description: format!(
                        "Unusual disk growth rate: {} changed in one sample.",
                        format_bytes(rate.abs())
                    ),
                    path: None,
                });
            }
        }
    }

    fn detect_hotspots(&self, history: &[db::Snapshot], events: &mut Vec<AnomalyEvent>) {
        for pair in history.windows(2) {
            let previous = parse_dirs(&pair[0].snapshot_json);
            let current = parse_dirs(&pair[1].snapshot_json);
            if previous.is_empty() || current.is_empty() {
                continue;
            }

            let previous_by_path: HashMap<String, u64> = previous
                .into_iter()
                .map(|dir| (dir.path, dir.size_bytes))
                .collect();
            let growths: Vec<(DirInfo, f64)> = current
                .into_iter()
                .filter_map(|dir| {
                    previous_by_path.get(&dir.path).map(|old_size| {
                        let growth = dir.size_bytes.saturating_sub(*old_size) as f64;
                        (dir, growth)
                    })
                })
                .filter(|(_, growth)| *growth > 0.0)
                .collect();
            if growths.is_empty() {
                continue;
            }

            let values: Vec<f64> = growths.iter().map(|(_, growth)| *growth).collect();
            let scores = self.modified_z_score.scores(&values);

            for (index, ((dir, growth), score)) in
                growths.into_iter().zip(scores.into_iter()).enumerate()
            {
                let peer_growths: Vec<f64> = values
                    .iter()
                    .enumerate()
                    .filter_map(|(peer_index, value)| (peer_index != index).then_some(*value))
                    .collect();
                let peer_baseline = median(&peer_growths).max(1.0);
                let large_small_sample_outlier =
                    growth >= 10.0 * 1024.0 * 1024.0 * 1024.0 && growth >= peer_baseline * 4.0;
                if score.abs() > self.modified_z_score.threshold || large_small_sample_outlier {
                    events.push(AnomalyEvent {
                        anomaly_type: AnomalyType::HotspotAnomaly,
                        severity: if score.abs() > 10.0 || large_small_sample_outlier {
                            AnomalySeverity::Critical
                        } else {
                            AnomalySeverity::Warning
                        },
                        drive_letter: pair[1].drive_letter.clone(),
                        created_at: pair[1].created_at.clone(),
                        metric_value: growth,
                        modified_z_score: score,
                        description: format!(
                            "{} grew unusually by {}.",
                            dir.name,
                            format_bytes(growth)
                        ),
                        path: Some(dir.path),
                    });
                }
            }
        }
    }

    fn detect_pattern_deviation(&self, history: &[db::Snapshot], events: &mut Vec<AnomalyEvent>) {
        if history.len() < self.holt_winters.period * 2 {
            return;
        }

        let values: Vec<f64> = history
            .iter()
            .map(|snapshot| snapshot.used_bytes as f64)
            .collect();
        let result = self.holt_winters.forecast(&values, 1);
        let scores = self.modified_z_score.scores(&result.residuals);
        let Some((latest, score)) = history.last().zip(scores.last()) else {
            return;
        };

        if score.abs() > self.modified_z_score.threshold {
            events.push(AnomalyEvent {
                anomaly_type: AnomalyType::PatternDeviation,
                severity: if score.abs() > 10.0 {
                    AnomalySeverity::Critical
                } else {
                    AnomalySeverity::Warning
                },
                drive_letter: latest.drive_letter.clone(),
                created_at: latest.created_at.clone(),
                metric_value: latest.used_bytes as f64,
                modified_z_score: *score,
                description: "Usage deviated from the learned seasonal pattern.".into(),
                path: None,
            });
        }
    }
}

pub fn detect_anomalies(drive: &str) -> Result<AnomalyReport, String> {
    let history = db::get_snapshot_history(drive, 365)?;
    let events = AnomalyDetector::default().detect(&history);
    Ok(AnomalyReport {
        drive_letter: drive.to_uppercase(),
        sample_count: history.len(),
        events,
    })
}

fn parse_dirs(json: &str) -> Vec<DirInfo> {
    serde_json::from_str::<Vec<DirInfo>>(json).unwrap_or_default()
}

fn median(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = sorted.len() / 2;
    if sorted.len().is_multiple_of(2) {
        (sorted[mid - 1] + sorted[mid]) / 2.0
    } else {
        sorted[mid]
    }
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

fn format_bytes(bytes: f64) -> String {
    let abs = bytes.abs();
    let gib = abs / 1024.0 / 1024.0 / 1024.0;
    if gib >= 1.0 {
        format!("{:.1} GB", gib)
    } else {
        format!("{:.0} MB", abs / 1024.0 / 1024.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GIB: u64 = 1024 * 1024 * 1024;

    fn snapshot(id: i64, date: &str, used_gib: u64, dirs_json: &str) -> db::Snapshot {
        let total = 200 * GIB;
        db::Snapshot {
            id,
            drive_letter: "C".into(),
            total_bytes: total,
            used_bytes: used_gib * GIB,
            free_bytes: total.saturating_sub(used_gib * GIB),
            snapshot_json: dirs_json.into(),
            created_at: date.into(),
        }
    }

    #[test]
    fn holt_winters_forecasts_seasonal_growth() {
        let series: Vec<f64> = (0..28)
            .map(|i| 100.0 + i as f64 * 2.0 + [0.0, 5.0, 8.0, 4.0, -2.0, -6.0, -3.0][i % 7])
            .collect();
        let model = HoltWinters {
            alpha: 0.45,
            beta: 0.2,
            gamma: 0.25,
            period: 7,
        };

        let result = model.forecast(&series, 1);

        assert!((result.forecast[0] - 156.0).abs() < 6.0);
        assert!(result.trend > 1.0);
        assert!(result.seasonal.abs() > 0.1);
    }

    #[test]
    fn fusion_weights_degrade_when_autoencoder_unavailable() {
        assert_eq!(fusion_weights(AeStatus::Healthy).autoencoder, 0.40);
        assert_eq!(fusion_weights(AeStatus::Degraded).autoencoder, 0.10);
        assert_eq!(fusion_weights(AeStatus::Disabled).autoencoder, 0.0);
        assert_eq!(fusion_weights(AeStatus::Disabled).holt_winters, 0.50);
    }

    #[test]
    fn fusion_confidence_is_high_when_two_detectors_trigger() {
        let result = fuse_anomaly_signals(
            AeStatus::Healthy,
            vec![
                FusionSignal {
                    name: "holt_winters".into(),
                    triggered: true,
                    score: 0.8,
                },
                FusionSignal {
                    name: "zscore".into(),
                    triggered: true,
                    score: 0.9,
                },
                FusionSignal {
                    name: "autoencoder".into(),
                    triggered: false,
                    score: 0.0,
                },
            ],
        );

        assert_eq!(result.confidence, "high");
        assert!(result.fused_score > 0.4);
    }

    #[test]
    fn modified_z_score_flags_outlier() {
        let detector = ModifiedZScore { threshold: 3.5 };
        let scores = detector.scores(&[10.0, 11.0, 9.5, 10.5, 10.0, 90.0]);

        assert!(scores[5].abs() > detector.threshold);
        assert!(scores[..5]
            .iter()
            .all(|score| score.abs() < detector.threshold));
    }

    #[test]
    fn detects_burst_anomaly_above_threshold() {
        let history = vec![
            snapshot(1, "2026-05-01 00:00:00", 80, "[]"),
            snapshot(2, "2026-05-02 00:00:00", 81, "[]"),
            snapshot(3, "2026-05-03 00:00:00", 82, "[]"),
            snapshot(4, "2026-05-04 00:00:00", 83, "[]"),
            snapshot(5, "2026-05-05 00:00:00", 130, "[]"),
        ];

        let events = AnomalyDetector::default().detect(&history);

        assert!(events
            .iter()
            .any(|event| event.anomaly_type == AnomalyType::BurstAnomaly));
    }

    #[test]
    fn detects_hotspot_directory_growth() {
        let day1 = r#"[{"name":"Cache","path":"C:\\Cache","size_bytes":10737418240,"file_count":1,"dir_count":0,"risk_level":null,"is_approximate":false},{"name":"Logs","path":"C:\\Logs","size_bytes":10737418240,"file_count":1,"dir_count":0,"risk_level":null,"is_approximate":false}]"#;
        let day2 = r#"[{"name":"Cache","path":"C:\\Cache","size_bytes":75161927680,"file_count":1,"dir_count":0,"risk_level":null,"is_approximate":false},{"name":"Logs","path":"C:\\Logs","size_bytes":11811160064,"file_count":1,"dir_count":0,"risk_level":null,"is_approximate":false}]"#;
        let history = vec![
            snapshot(1, "2026-05-01 00:00:00", 80, day1),
            snapshot(2, "2026-05-02 00:00:00", 141, day2),
        ];

        let events = AnomalyDetector::default().detect(&history);

        assert!(events.iter().any(|event| {
            event.anomaly_type == AnomalyType::HotspotAnomaly
                && event.path.as_deref() == Some("C:\\Cache")
        }));
    }

    #[test]
    fn empty_history_returns_no_events() {
        let events = AnomalyDetector::default().detect(&[]);

        assert!(events.is_empty());
    }
}
