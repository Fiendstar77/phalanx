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
/// Cumulative same-state comparisons across all live teams and controller modes.
/// Missing predictions are counted separately and never enter agreement's denominator.
#[derive(Clone, Debug, Default)]
pub struct ComparisonSummary {
    pub compared: u32,
    pub agreements: u32,
    pub unavailable: u32,
    /// Rows are utility actions, columns learned actions, including agreements.
    pub action_pairs: [[u32; ACTION_COUNT]; ACTION_COUNT],
}
#[derive(Debug, Serialize)]
pub struct ActionPair {
    pub utility: &'static str,
    pub learned: &'static str,
    pub count: u32,
}
impl ComparisonSummary {
    pub fn record(&mut self, comparison: &Comparison) {
        if let Some(learned) = comparison.learned.filter(|i| *i < ACTION_COUNT)
            && comparison.utility < ACTION_COUNT
        {
            self.compared += 1;
            self.agreements += u32::from(learned == comparison.utility);
            self.action_pairs[comparison.utility][learned] += 1;
        } else {
            self.unavailable += 1;
        }
    }
    pub fn agreement_rate(&self) -> Option<f64> {
        (self.compared > 0).then(|| self.agreements as f64 / self.compared as f64)
    }
    pub fn top_disagreements(&self, limit: usize) -> Vec<ActionPair> {
        let mut pairs = Vec::new();
        for (utility, row) in self.action_pairs.iter().enumerate() {
            for (learned, count) in row.iter().enumerate() {
                if utility != learned && *count > 0 {
                    pairs.push((utility, learned, *count));
                }
            }
        }
        // Deterministic action-index order resolves tied counts.
        pairs.sort_by_key(|(u, l, count)| (std::cmp::Reverse(*count), *u, *l));
        pairs
            .into_iter()
            .take(limit)
            .map(|(u, l, count)| ActionPair {
                utility: ACTIONS[u],
                learned: ACTIONS[l],
                count,
            })
            .collect()
    }
}
impl Serialize for ComparisonSummary {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("ComparisonSummary", 7)?;
        state.serialize_field("compared", &self.compared)?;
        state.serialize_field("agreements", &self.agreements)?;
        state.serialize_field("unavailable", &self.unavailable)?;
        state.serialize_field("agreement_rate", &self.agreement_rate())?;
        state.serialize_field("actions", &ACTIONS)?;
        state.serialize_field("action_pairs", &self.action_pairs)?;
        state.serialize_field(
            "top_disagreements",
            &self.top_disagreements(ACTION_COUNT * ACTION_COUNT),
        )?;
        state.end()
    }
}
impl LearnedPolicy {
    pub fn from_json(text: &str) -> Result<Self, String> {
        let model: Self = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if model.features != FEATURE_NAMES
            || model.actions != ACTIONS
            || model.mean.len() != FEATURE_NAMES.len()
            || model.std.len() != FEATURE_NAMES.len()
            || model.layers.len() < 2
            || model.layers.len() > 6
        {
            return Err("policy schema mismatch".into());
        }
        if model.schema_version == 1 && (model.layers.len() != 3 || model.layers[0].weight.len() != 64)
        {
            return Err("policy schema mismatch".into());
        }
        if model.schema_version != 1 && model.schema_version != 2 {
            return Err("policy schema mismatch".into());
        }
        if model.mean.iter().any(|v| !v.is_finite())
            || model.std.iter().any(|v| !v.is_finite() || *v <= 0.0)
        {
            return Err("invalid normalization".into());
        }
        let mut inputs = FEATURE_NAMES.len();
        for (i, layer) in model.layers.iter().enumerate() {
            let width = layer.weight.len();
            let last = i + 1 == model.layers.len();
            if width == 0
                || width > 2048
                || (last && width != ACTION_COUNT)
                || (!last && model.schema_version == 1 && width != 64)
                || layer.bias.len() != width
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
                    if i + 1 < self.layers.len() {
                        value.max(0.0)
                    } else {
                        value
                    }
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
