use crate::db;
use serde::{Deserialize, Serialize};

const TARGET_USAGE_PERCENT: f64 = 95.0;
const MIN_GROWTH_BYTES_PER_DAY: f64 = 10.0 * 1024.0 * 1024.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastPoint {
    pub created_at: String,
    pub used_bytes: u64,
    pub free_bytes: u64,
    pub usage_percent: f64,
    pub is_forecast: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prediction {
    pub drive_letter: String,
    pub sample_count: usize,
    pub window_days: u32,
    pub current_usage_percent: f64,
    pub growth_bytes_per_day: f64,
    pub growth_percent_per_day: f64,
    pub days_to_95_percent: Option<f64>,
    pub projected_95_date: Option<String>,
    pub confidence_score: f64,
    pub status: String,
    pub message: String,
    pub forecast: Vec<ForecastPoint>,
}

#[derive(Debug, Clone)]
struct Regression {
    slope: f64,
    r_squared: f64,
}

pub fn predict_disk_usage(drive: &str, days: u32) -> Result<Prediction, String> {
    let snapshots = db::get_snapshot_history(drive, days)?;
    predict_from_snapshots(drive, days, &snapshots)
}

fn predict_from_snapshots(
    drive: &str,
    days: u32,
    snapshots: &[db::Snapshot],
) -> Result<Prediction, String> {
    let mut points: Vec<(i64, &db::Snapshot)> = snapshots
        .iter()
        .filter_map(|snapshot| parse_sqlite_datetime(&snapshot.created_at).map(|ts| (ts, snapshot)))
        .collect();

    points.sort_by_key(|(timestamp, _)| *timestamp);

    let Some((latest_ts, latest)) = points.last().copied() else {
        return Ok(empty_prediction(
            drive,
            days,
            0,
            "No snapshot history yet. Run a drive scan to start building a forecast.",
        ));
    };

    let total = latest.total_bytes.max(1) as f64;
    let current_usage_percent = (latest.used_bytes as f64 / total) * 100.0;

    if points.len() < 3 {
        return Ok(Prediction {
            drive_letter: drive.to_uppercase(),
            sample_count: points.len(),
            window_days: days,
            current_usage_percent,
            growth_bytes_per_day: 0.0,
            growth_percent_per_day: 0.0,
            days_to_95_percent: None,
            projected_95_date: None,
            confidence_score: 0.0,
            status: "insufficient_data".into(),
            message: "Need at least 3 snapshots before DiskPulse can estimate a trend.".into(),
            forecast: points
                .iter()
                .map(|(_, snapshot)| snapshot_point(snapshot, false))
                .collect(),
        });
    }

    let first_ts = points[0].0;
    let x_days: Vec<f64> = points
        .iter()
        .map(|(timestamp, _)| (*timestamp - first_ts) as f64 / 86_400.0)
        .collect();
    let y_used: Vec<f64> = points
        .iter()
        .map(|(_, snapshot)| snapshot.used_bytes as f64)
        .collect();

    let regression = linear_regression(&x_days, &y_used);
    let sample_factor = (points.len() as f64 / 6.0).min(1.0);
    let confidence_score = (regression.r_squared * sample_factor).clamp(0.0, 1.0);
    let growth_percent_per_day = (regression.slope / total) * 100.0;

    let target_used = total * (TARGET_USAGE_PERCENT / 100.0);
    let days_to_95_percent = if latest.used_bytes as f64 >= target_used {
        Some(0.0)
    } else if regression.slope > MIN_GROWTH_BYTES_PER_DAY {
        Some(((target_used - latest.used_bytes as f64) / regression.slope).max(0.0))
    } else {
        None
    };
    let projected_95_date =
        days_to_95_percent.map(|days_until| format_datetime(add_days(latest_ts, days_until)));

    let status = prediction_status(current_usage_percent, regression.slope, days_to_95_percent);
    let message = prediction_message(
        status,
        current_usage_percent,
        regression.slope,
        days_to_95_percent,
    );
    let forecast = build_forecast(&points, regression.slope, latest_ts, latest, days);

    Ok(Prediction {
        drive_letter: drive.to_uppercase(),
        sample_count: points.len(),
        window_days: days,
        current_usage_percent,
        growth_bytes_per_day: regression.slope,
        growth_percent_per_day,
        days_to_95_percent,
        projected_95_date,
        confidence_score,
        status: status.into(),
        message,
        forecast,
    })
}

fn empty_prediction(drive: &str, days: u32, sample_count: usize, message: &str) -> Prediction {
    Prediction {
        drive_letter: drive.to_uppercase(),
        sample_count,
        window_days: days,
        current_usage_percent: 0.0,
        growth_bytes_per_day: 0.0,
        growth_percent_per_day: 0.0,
        days_to_95_percent: None,
        projected_95_date: None,
        confidence_score: 0.0,
        status: "insufficient_data".into(),
        message: message.into(),
        forecast: Vec::new(),
    }
}

fn linear_regression(x: &[f64], y: &[f64]) -> Regression {
    let n = x.len() as f64;
    let mean_x = x.iter().sum::<f64>() / n;
    let mean_y = y.iter().sum::<f64>() / n;

    let mut numerator = 0.0;
    let mut denominator = 0.0;
    for (x_value, y_value) in x.iter().zip(y.iter()) {
        numerator += (x_value - mean_x) * (y_value - mean_y);
        denominator += (x_value - mean_x).powi(2);
    }

    let slope = if denominator.abs() < f64::EPSILON {
        0.0
    } else {
        numerator / denominator
    };
    let intercept = mean_y - slope * mean_x;

    let mut sse = 0.0;
    let mut sst = 0.0;
    for (x_value, y_value) in x.iter().zip(y.iter()) {
        let predicted = intercept + slope * x_value;
        sse += (y_value - predicted).powi(2);
        sst += (y_value - mean_y).powi(2);
    }

    let r_squared = if sst.abs() < f64::EPSILON {
        1.0
    } else {
        (1.0 - sse / sst).clamp(0.0, 1.0)
    };

    Regression { slope, r_squared }
}

fn prediction_status(
    current_usage_percent: f64,
    growth_bytes_per_day: f64,
    days_to_95_percent: Option<f64>,
) -> &'static str {
    if current_usage_percent >= TARGET_USAGE_PERCENT {
        "critical"
    } else if days_to_95_percent.is_some_and(|days| days <= 14.0) {
        "warning"
    } else if growth_bytes_per_day > MIN_GROWTH_BYTES_PER_DAY {
        "growing"
    } else if growth_bytes_per_day < -MIN_GROWTH_BYTES_PER_DAY {
        "shrinking"
    } else {
        "stable"
    }
}

fn prediction_message(
    status: &str,
    current_usage_percent: f64,
    growth_bytes_per_day: f64,
    days_to_95_percent: Option<f64>,
) -> String {
    match (status, days_to_95_percent) {
        ("critical", _) => format!(
            "Drive is already above {:.0}% usage. Free space is critically low.",
            TARGET_USAGE_PERCENT
        ),
        ("warning", Some(days)) => format!(
            "At the current growth rate, this drive may reach {:.0}% usage in about {:.0} days.",
            TARGET_USAGE_PERCENT, days
        ),
        ("growing", Some(days)) => format!(
            "Usage is growing by about {} per day; {:.0}% usage is projected in {:.0} days.",
            format_bytes(growth_bytes_per_day),
            TARGET_USAGE_PERCENT,
            days
        ),
        ("growing", None) => format!(
            "Usage is growing by about {} per day, but the {:.0}% threshold is outside this forecast window.",
            format_bytes(growth_bytes_per_day),
            TARGET_USAGE_PERCENT
        ),
        ("shrinking", _) => format!(
            "Usage is trending down by about {} per day.",
            format_bytes(growth_bytes_per_day.abs())
        ),
        _ => format!(
            "Usage is stable at {:.1}%. No near-term capacity risk detected.",
            current_usage_percent
        ),
    }
}

fn build_forecast(
    points: &[(i64, &db::Snapshot)],
    slope: f64,
    latest_ts: i64,
    latest: &db::Snapshot,
    days: u32,
) -> Vec<ForecastPoint> {
    let mut forecast: Vec<ForecastPoint> = points
        .iter()
        .map(|(_, snapshot)| snapshot_point(snapshot, false))
        .collect();

    let horizon_days = days.clamp(30, 180);
    let step_days = (horizon_days / 6).max(1);
    let total = latest.total_bytes.max(1);

    for step in 1..=6 {
        let future_days = (step * step_days) as f64;
        let used = (latest.used_bytes as f64 + slope * future_days)
            .clamp(0.0, total as f64)
            .round() as u64;
        let free = total.saturating_sub(used);
        forecast.push(ForecastPoint {
            created_at: format_datetime(add_days(latest_ts, future_days)),
            used_bytes: used,
            free_bytes: free,
            usage_percent: (used as f64 / total as f64) * 100.0,
            is_forecast: true,
        });
    }

    forecast
}

fn snapshot_point(snapshot: &db::Snapshot, is_forecast: bool) -> ForecastPoint {
    let total = snapshot.total_bytes.max(1);
    ForecastPoint {
        created_at: snapshot.created_at.clone(),
        used_bytes: snapshot.used_bytes,
        free_bytes: snapshot.free_bytes,
        usage_percent: (snapshot.used_bytes as f64 / total as f64) * 100.0,
        is_forecast,
    }
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

fn parse_sqlite_datetime(value: &str) -> Option<i64> {
    let normalized = value.replace('T', " ");
    let mut parts = normalized.split_whitespace();
    let date = parts.next()?;
    let time = parts.next().unwrap_or("00:00:00");

    let mut date_parts = date.split('-');
    let year = date_parts.next()?.parse::<i32>().ok()?;
    let month = date_parts.next()?.parse::<u32>().ok()?;
    let day = date_parts.next()?.parse::<u32>().ok()?;

    let mut time_parts = time.split(':');
    let hour = time_parts.next()?.parse::<u32>().ok()?;
    let minute = time_parts.next()?.parse::<u32>().ok()?;
    let second = time_parts.next().unwrap_or("0").parse::<u32>().ok()?;

    let days = days_from_civil(year, month, day)?;
    Some(days * 86_400 + hour as i64 * 3_600 + minute as i64 * 60 + second as i64)
}

fn add_days(timestamp: i64, days: f64) -> i64 {
    timestamp + (days * 86_400.0).round() as i64
}

fn format_datetime(timestamp: i64) -> String {
    let days = timestamp.div_euclid(86_400);
    let seconds = timestamp.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = seconds / 3_600;
    let minute = (seconds % 3_600) / 60;
    let second = seconds % 60;
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}")
}

// Howard Hinnant civil calendar algorithms, adapted for UTC day math.
fn days_from_civil(year: i32, month: u32, day: u32) -> Option<i64> {
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }

    let year = year as i64 - if month <= 2 { 1 } else { 0 };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let month = month as i64;
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe - 719_468)
}

fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let days = days + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let doe = days - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = year + if month <= 2 { 1 } else { 0 };
    (year as i32, month as u32, day as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GIB: u64 = 1024 * 1024 * 1024;

    fn snapshot(id: i64, date: &str, used: u64) -> db::Snapshot {
        let total = 100 * GIB;
        db::Snapshot {
            id,
            drive_letter: "C".into(),
            total_bytes: total,
            used_bytes: used,
            free_bytes: total.saturating_sub(used),
            snapshot_json: "[]".into(),
            created_at: date.into(),
        }
    }

    #[test]
    fn date_round_trip_preserves_timestamp() {
        let ts = parse_sqlite_datetime("2026-05-07 12:34:56").expect("parse date");
        assert_eq!(format_datetime(ts), "2026-05-07 12:34:56");
    }

    #[test]
    fn growing_history_projects_threshold() {
        let snapshots = vec![
            snapshot(1, "2026-05-01 00:00:00", 50 * GIB),
            snapshot(2, "2026-05-02 00:00:00", 55 * GIB),
            snapshot(3, "2026-05-03 00:00:00", 60 * GIB),
            snapshot(4, "2026-05-04 00:00:00", 65 * GIB),
        ];

        let prediction = predict_from_snapshots("C", 30, &snapshots).expect("prediction");
        assert_eq!(prediction.status, "warning");
        assert_eq!(prediction.sample_count, 4);
        assert!(prediction.confidence_score > 0.6);
        assert!(prediction
            .days_to_95_percent
            .is_some_and(|days| days > 5.0 && days < 7.0));
        assert!(prediction.forecast.iter().any(|point| point.is_forecast));
    }

    #[test]
    fn insufficient_history_returns_explainer() {
        let snapshots = vec![snapshot(1, "2026-05-01 00:00:00", 50 * GIB)];
        let prediction = predict_from_snapshots("C", 30, &snapshots).expect("prediction");
        assert_eq!(prediction.status, "insufficient_data");
        assert_eq!(prediction.sample_count, 1);
        assert!(prediction.days_to_95_percent.is_none());
    }
}
