use super::features::SnapshotFeatures;

pub const SYNTHETIC_AE_TRAINING_SAMPLES: usize = 5_400;

pub fn synthetic_training_samples() -> Vec<SnapshotFeatures> {
    (0..SYNTHETIC_AE_TRAINING_SAMPLES)
        .map(|index| {
            let phase = index as f64 / SYNTHETIC_AE_TRAINING_SAMPLES as f64;
            let seasonal = (phase * std::f64::consts::TAU).sin().abs();
            SnapshotFeatures {
                usage_ratio: (0.35 + seasonal * 0.35).clamp(0.0, 1.0),
                free_ratio: (0.65 - seasonal * 0.35).clamp(0.0, 1.0),
                growth_ratio: (0.01 + phase * 0.03).clamp(0.0, 1.0),
                volatility_ratio: (0.01 + seasonal * 0.04).clamp(0.0, 1.0),
                hotspot_ratio: (0.05 + seasonal * 0.12).clamp(0.0, 1.0),
                sample_density: 1.0,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_more_than_five_thousand_samples() {
        let samples = synthetic_training_samples();

        assert!(samples.len() >= 5_000);
        assert!(samples.iter().all(|sample| sample.sample_density > 0.0));
    }
}
