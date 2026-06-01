use crate::risk::{RiskItem, RiskLevel};
use crate::scanner;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
struct RecommendationInput {
    path: String,
    name: String,
    size_bytes: u64,
    risk_level: String,
    safe_to_delete: bool,
    age_days: Option<u64>,
    duplicate_waste_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecommendationItem {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub risk_level: String,
    pub safe_to_delete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Recommendation {
    pub rank: usize,
    pub item: RecommendationItem,
    pub score: f64,
    pub reason: String,
    pub estimated_size: u64,
    pub action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiskHealth {
    pub drive_letter: String,
    pub score: u8,
    pub status: String,
    pub free_percent: f64,
    pub duplicate_waste_bytes: u64,
    pub zombie_bytes: u64,
    pub message: String,
}

#[derive(Debug, Clone)]
struct ScoringWeights {
    risk_factor: f64,
    age_factor: f64,
    duplicate_factor: f64,
    size_factor: f64,
    safety_factor: f64,
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            risk_factor: 0.20,
            age_factor: 0.15,
            duplicate_factor: 0.20,
            size_factor: 0.20,
            safety_factor: 0.25,
        }
    }
}

pub fn get_recommendations(drive: &str) -> Result<Vec<Recommendation>, String> {
    let scan = scanner::scan_drive(drive)?;
    let report = crate::risk::classify_risks(&scan);
    let inputs = report
        .items
        .into_iter()
        .map(input_from_risk_item)
        .collect::<Vec<_>>();
    Ok(rank_recommendations(inputs, &ScoringWeights::default()))
}

pub fn get_disk_health(drive: &str) -> Result<DiskHealth, String> {
    let meta = scanner::scan_drive_meta(drive, None, None)?;
    let free_percent = if meta.total_bytes > 0 {
        (meta.free_bytes as f64 / meta.total_bytes as f64) * 100.0
    } else {
        0.0
    };
    let mut health = calculate_disk_health(free_percent, 0.0, 0, 0);
    health.drive_letter = meta.drive_letter;
    Ok(health)
}

fn input_from_risk_item(item: RiskItem) -> RecommendationInput {
    RecommendationInput {
        path: item.path,
        name: item.name,
        size_bytes: item.size_bytes,
        risk_level: risk_level_to_string(&item.risk_level),
        safe_to_delete: item.safe_to_delete,
        age_days: None,
        duplicate_waste_bytes: 0,
    }
}

fn risk_level_to_string(level: &RiskLevel) -> String {
    match level {
        RiskLevel::Low => "low",
        RiskLevel::Medium => "medium",
        RiskLevel::High => "high",
    }
    .to_string()
}

fn rank_recommendations(
    inputs: Vec<RecommendationInput>,
    weights: &ScoringWeights,
) -> Vec<Recommendation> {
    let mut recommendations = inputs
        .into_iter()
        .map(|input| {
            let score = score_recommendation(&input, weights);
            Recommendation {
                rank: 0,
                estimated_size: input
                    .duplicate_waste_bytes
                    .max(if input.safe_to_delete { input.size_bytes } else { 0 }),
                reason: recommendation_reason(&input),
                action: if input.safe_to_delete {
                    "preview_cleanup".into()
                } else {
                    "review".into()
                },
                item: RecommendationItem {
                    name: input.name,
                    path: input.path,
                    size_bytes: input.size_bytes,
                    risk_level: input.risk_level,
                    safe_to_delete: input.safe_to_delete,
                },
                score,
            }
        })
        .collect::<Vec<_>>();

    recommendations.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for (idx, recommendation) in recommendations.iter_mut().enumerate() {
        recommendation.rank = idx + 1;
    }
    recommendations
}

fn score_recommendation(input: &RecommendationInput, weights: &ScoringWeights) -> f64 {
    let risk = match input.risk_level.as_str() {
        "low" => 100.0,
        "medium" => 50.0,
        _ => 5.0,
    };
    let age = input
        .age_days
        .map(|days| (days as f64 / 365.0 * 100.0).min(100.0))
        .unwrap_or(25.0);
    let duplicate = if input.duplicate_waste_bytes > 0 {
        100.0
    } else {
        0.0
    };
    let size = (input.size_bytes as f64 / 1_000_000_000.0 * 40.0).min(100.0);
    let safety = if input.safe_to_delete { 100.0 } else { 10.0 };

    risk * weights.risk_factor
        + age * weights.age_factor
        + duplicate * weights.duplicate_factor
        + size * weights.size_factor
        + safety * weights.safety_factor
}

fn recommendation_reason(input: &RecommendationInput) -> String {
    if input.duplicate_waste_bytes > 0 {
        return "Duplicate content can be reviewed to reclaim repeated bytes.".into();
    }
    if input.safe_to_delete {
        return "Low-risk whitelisted cleanup candidate.".into();
    }
    "Review manually before cleanup.".into()
}

fn calculate_disk_health(
    free_percent: f64,
    growth_percent_per_day: f64,
    duplicate_waste_bytes: u64,
    zombie_bytes: u64,
) -> DiskHealth {
    let mut score = free_percent.clamp(0.0, 100.0);
    score -= (growth_percent_per_day.max(0.0) * 8.0).min(25.0);
    if duplicate_waste_bytes > 10_000_000_000 {
        score -= 10.0;
    }
    if zombie_bytes > 25_000_000_000 {
        score -= 10.0;
    }
    let score = score.round().clamp(0.0, 100.0) as u8;
    let status = if score >= 75 {
        "healthy"
    } else if score >= 50 {
        "watch"
    } else {
        "warning"
    }
    .to_string();
    let message = match status.as_str() {
        "healthy" => "Disk has comfortable free space.",
        "watch" => "Disk should be watched for growth or cleanup opportunities.",
        _ => "Disk is under pressure; review cleanup recommendations.",
    }
    .to_string();

    DiskHealth {
        drive_letter: String::new(),
        score,
        status,
        free_percent,
        duplicate_waste_bytes,
        zombie_bytes,
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_prefers_large_safe_low_risk_items() {
        let item = RecommendationInput {
            path: "C:\\Temp\\cache".into(),
            name: "cache".into(),
            size_bytes: 2_000_000_000,
            risk_level: "low".into(),
            safe_to_delete: true,
            age_days: Some(300),
            duplicate_waste_bytes: 0,
        };

        let score = score_recommendation(&item, &ScoringWeights::default());

        assert!(score >= 70.0);
    }

    #[test]
    fn recommendations_are_ranked_by_score_descending() {
        let inputs = vec![
            RecommendationInput {
                path: "C:\\Unknown".into(),
                name: "Unknown".into(),
                size_bytes: 1_000,
                risk_level: "high".into(),
                safe_to_delete: false,
                age_days: None,
                duplicate_waste_bytes: 0,
            },
            RecommendationInput {
                path: "C:\\Temp\\cache".into(),
                name: "cache".into(),
                size_bytes: 3_000_000_000,
                risk_level: "low".into(),
                safe_to_delete: true,
                age_days: Some(200),
                duplicate_waste_bytes: 0,
            },
        ];

        let recommendations = rank_recommendations(inputs, &ScoringWeights::default());

        assert_eq!(recommendations[0].rank, 1);
        assert!(recommendations[0].score >= recommendations[1].score);
        assert_eq!(recommendations[0].item.path, "C:\\Temp\\cache");
    }

    #[test]
    fn disk_health_penalizes_low_free_space() {
        let health = calculate_disk_health(5.0, 0.0, 0, 0);

        assert!(health.score < 60);
        assert_eq!(health.status, "warning");
    }
}
