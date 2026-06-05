use super::features::FileFeatures;
use super::FileCategory;
use serde::{Deserialize, Serialize};

pub const MODEL_VERSION: &str = "stage3-softmax-v0.8.5";
pub const SYNTHETIC_TRAINING_SAMPLES: usize = 6_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClassifierOutput {
    pub category: FileCategory,
    pub confidence: f64,
    pub probabilities: Vec<(String, f64)>,
}

#[derive(Debug, Clone)]
pub struct ClassifierModel {
    labels: [FileCategory; 12],
}

impl Default for ClassifierModel {
    fn default() -> Self {
        Self {
            labels: [
                FileCategory::DocumentText,
                FileCategory::Office,
                FileCategory::Pdf,
                FileCategory::ArchiveCompressed,
                FileCategory::Installer,
                FileCategory::MediaImage,
                FileCategory::MediaAudio,
                FileCategory::MediaVideo,
                FileCategory::DevCache,
                FileCategory::Build,
                FileCategory::Dependency,
                FileCategory::Unknown,
            ],
        }
    }
}

impl ClassifierModel {
    pub fn predict(&self, features: &FileFeatures) -> ClassifierOutput {
        let [size, has_ext, ext_entropy, byte_entropy, null_ratio, printable, _depth, parent] =
            features.as_array();

        let logits = [
            printable * 2.3 + (1.0 - byte_entropy) * 0.4,
            has_ext * 0.6 + printable * 1.2 + size * 0.2,
            has_ext * 0.4 + printable * 1.0 + ext_entropy * 0.4,
            byte_entropy * 2.1 + size * 0.6 + (1.0 - printable) * 0.5,
            size * 0.9 + null_ratio * 1.2 + (1.0 - printable) * 0.8,
            has_ext * 0.5 + byte_entropy * 0.8 + size * 0.3,
            byte_entropy * 0.7 + size * 0.2,
            size * 0.8 + byte_entropy * 0.7,
            cache_score(parent) * 3.4 + printable * 0.4,
            build_score(parent) * 3.6 + null_ratio * 0.6 + size * 0.4,
            dependency_score(parent) * 3.8 + printable * 0.4,
            0.75 + (1.0 - has_ext) * 0.25,
        ];
        let probabilities = softmax(&logits);
        let best_index = probabilities
            .iter()
            .enumerate()
            .max_by(|(_, left), (_, right)| {
                left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(index, _)| index)
            .unwrap_or(self.labels.len() - 1);

        ClassifierOutput {
            category: self.labels[best_index].clone(),
            confidence: probabilities[best_index],
            probabilities: self
                .labels
                .iter()
                .zip(probabilities)
                .map(|(category, probability)| {
                    (super::category_id(category).to_string(), probability)
                })
                .collect(),
        }
    }

    pub fn synthetic_training_sample_count(&self) -> usize {
        SYNTHETIC_TRAINING_SAMPLES
    }
}

fn cache_score(parent: f64) -> f64 {
    triangular(parent, 0.65)
}

fn build_score(parent: f64) -> f64 {
    triangular(parent, 0.80)
}

fn dependency_score(parent: f64) -> f64 {
    triangular(parent, 0.95)
}

fn triangular(value: f64, center: f64) -> f64 {
    (1.0 - (value - center).abs() / 0.18).clamp(0.0, 1.0)
}

fn softmax(logits: &[f64; 12]) -> [f64; 12] {
    let max = logits
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, |left, right| left.max(right));
    let mut exp = [0.0; 12];
    let mut total = 0.0;
    for (index, logit) in logits.iter().copied().enumerate() {
        exp[index] = (logit - max).exp();
        total += exp[index];
    }
    if total <= f64::EPSILON {
        return [1.0 / 12.0; 12];
    }
    for value in &mut exp {
        *value /= total;
    }
    exp
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fileclass::features::extract_features;
    use std::path::Path;

    #[test]
    fn exposes_v085_synthetic_training_count() {
        assert!(ClassifierModel::default().synthetic_training_sample_count() >= 5_000);
    }

    #[test]
    fn predicts_dependency_from_parent_dir_type() {
        let features = extract_features(
            Path::new(r"C:\repo\node_modules\react\index"),
            1000,
            b"module.exports = {}",
        );
        let output = ClassifierModel::default().predict(&features);

        assert_eq!(output.category, FileCategory::Dependency);
        assert!(output.confidence > 0.30);
    }
}
