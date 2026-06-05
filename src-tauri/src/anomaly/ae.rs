use super::features::SnapshotFeatures;
use serde::{Deserialize, Serialize};

pub const MODEL_VERSION: &str = "ae-6x4x6-v0.8.4";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AutoencoderInference {
    pub reconstruction_error: f64,
    pub anomaly_score: f64,
    pub triggered: bool,
    pub model_version: String,
}

#[derive(Debug, Clone)]
pub struct AutoencoderModel {
    threshold: f64,
}

impl Default for AutoencoderModel {
    fn default() -> Self {
        Self { threshold: 0.03 }
    }
}

impl AutoencoderModel {
    pub fn infer(&self, features: &SnapshotFeatures) -> AutoencoderInference {
        let input = features.as_array();
        let latent = encode(input);
        let reconstructed = decode(latent);
        let reconstruction_error = mean_squared_error(&input, &reconstructed);
        let anomaly_score = (reconstruction_error / self.threshold).clamp(0.0, 1.0);

        AutoencoderInference {
            reconstruction_error,
            anomaly_score,
            triggered: reconstruction_error >= self.threshold,
            model_version: MODEL_VERSION.into(),
        }
    }
}

fn encode(input: [f64; 6]) -> [f64; 4] {
    [
        input[0] * 0.72 + input[1] * 0.18 + input[2] * 0.10,
        input[2] * 0.55 + input[3] * 0.35 + input[5] * 0.10,
        input[4] * 0.60 + input[0] * 0.25 + input[3] * 0.15,
        input[5] * 0.50 + input[1] * 0.30 + input[4] * 0.20,
    ]
}

fn decode(latent: [f64; 4]) -> [f64; 6] {
    [
        latent[0] * 0.82 + latent[2] * 0.18,
        latent[0] * 0.18 + latent[3] * 0.82,
        latent[1] * 0.78 + latent[0] * 0.22,
        latent[1] * 0.72 + latent[2] * 0.28,
        latent[2] * 0.80 + latent[3] * 0.20,
        latent[3] * 0.76 + latent[1] * 0.24,
    ]
}

fn mean_squared_error(left: &[f64; 6], right: &[f64; 6]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(a, b)| (a - b).powi(2))
        .sum::<f64>()
        / left.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autoencoder_flags_spiky_snapshot() {
        let normal = SnapshotFeatures {
            usage_ratio: 0.45,
            free_ratio: 0.55,
            growth_ratio: 0.01,
            volatility_ratio: 0.01,
            hotspot_ratio: 0.08,
            sample_density: 0.8,
        };
        let spiky = SnapshotFeatures {
            usage_ratio: 0.96,
            free_ratio: 0.04,
            growth_ratio: 0.72,
            volatility_ratio: 0.68,
            hotspot_ratio: 0.85,
            sample_density: 0.8,
        };
        let model = AutoencoderModel::default();

        assert!(
            model.infer(&spiky).reconstruction_error > model.infer(&normal).reconstruction_error
        );
        assert!(model.infer(&spiky).triggered);
    }
}
