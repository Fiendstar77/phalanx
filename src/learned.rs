//! Portable inference of the actual PyTorch-trained weights. No runtime service or world authority.
use crate::{ACTION_COUNT, ACTIONS, telemetry::FEATURE_NAMES};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, OnceLock};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub enum Controller {
    #[default]
    Utility,
    Learned,
}
impl Controller {
    pub fn name(self) -> &'static str {
        match self {
            Self::Utility => "Utility",
            Self::Learned => "Learned",
        }
    }
}
#[derive(Debug, Deserialize)]
struct Layer {
    weight: Vec<Vec<f32>>,
    bias: Vec<f32>,
}
#[derive(Debug, Deserialize)]
pub struct LearnedPolicy {
    schema_version: u32,
    pub source_sha256: String,
    features: Vec<String>,
    actions: Vec<String>,
    mean: Vec<f32>,
    std: Vec<f32>,
    layers: Vec<Layer>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Comparison {
    pub tick: u32,
    pub unit: usize,
    pub utility: usize,
    pub learned: Option<usize>,
    pub executed: usize,
    pub confidence: Option<f32>,
    pub controller: Controller,
    pub disagreed: bool,
}
impl LearnedPolicy {
    pub fn from_json(text: &str) -> Result<Self, String> {
        let model: Self = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if model.schema_version != 1
            || model.features != FEATURE_NAMES
            || model.actions != ACTIONS
            || model.mean.len() != 29
            || model.std.len() != 29
            || model.layers.len() != 3
        {
            return Err("policy schema mismatch".into());
        }
        if model.mean.iter().any(|v| !v.is_finite())
            || model.std.iter().any(|v| !v.is_finite() || *v <= 0.0)
        {
            return Err("invalid normalization".into());
        }
        let mut inputs = 29;
        for (layer, width) in model.layers.iter().zip([64, 64, ACTION_COUNT]) {
            if layer.bias.len() != width
                || layer.weight.len() != width
                || layer
                    .weight
                    .iter()
                    .any(|row| row.len() != inputs || row.iter().any(|v| !v.is_finite()))
                || layer.bias.iter().any(|v| !v.is_finite())
            {
                return Err("invalid layer dimensions or values".into());
            }
            inputs = width;
        }
        Ok(model)
    }
    pub fn bundled() -> Arc<Self> {
        static MODEL: OnceLock<Arc<LearnedPolicy>> = OnceLock::new();
        MODEL
            .get_or_init(|| {
                Arc::new(
                    Self::from_json(include_str!("../assets/policy.json"))
                        .expect("bundled policy must be valid"),
                )
            })
            .clone()
    }
    pub fn logits(&self, features: &[f32]) -> Option<Vec<f32>> {
        if features.len() != self.mean.len() || features.iter().any(|v| !v.is_finite()) {
            return None;
        }
        let mut values: Vec<_> = features
            .iter()
            .zip(&self.mean)
            .zip(&self.std)
            .map(|((x, m), s)| (x - m) / s)
            .collect();
        for (i, layer) in self.layers.iter().enumerate() {
            values = layer
                .weight
                .iter()
                .zip(&layer.bias)
                .map(|(row, bias)| {
                    let value = row
                        .iter()
                        .zip(&values)
                        .fold(*bias, |sum, (w, x)| sum + w * x);
                    if i < 2 { value.max(0.0) } else { value }
                })
                .collect();
        }
        if values.iter().any(|v| !v.is_finite()) {
            None
        } else {
            Some(values)
        }
    }
    pub fn predict(&self, features: &[f32], mask: &[bool; ACTION_COUNT]) -> Option<(usize, f32)> {
        let values = self.logits(features)?;
        let mut best = None;
        for (i, value) in values.iter().enumerate() {
            if mask[i] && best.is_none_or(|old: usize| *value > values[old]) {
                best = Some(i);
            }
        }
        let best = best?;
        let denominator: f32 = values
            .iter()
            .enumerate()
            .filter(|(i, _)| mask[*i])
            .map(|(_, v)| (v - values[best]).exp())
            .sum();
        Some((best, 1.0 / denominator))
    }
}
