//! Works more with detecting which order is more probable based on how many times we have seen a fact
use crate::skeleton::cards::CardValue;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ProbabilityEngine {
    seen: HashMap<(CardValue, CardValue), (f64, usize)>,
}

impl ProbabilityEngine {
    pub fn new() -> ProbabilityEngine {
        ProbabilityEngine {
            seen: HashMap::new()
        }
    }

    pub fn update(&mut self, (a, b): &(CardValue, CardValue), certainty: f64) {
        assert!(certainty >= -1.0 && certainty <= 1.0);

        let (certainty, a, b) = if a < b { (certainty, a, b) } else { (-certainty, b, a) };
        let (probability, reps) = self.seen.entry((*a, *b)).or_insert((0.0, 0));
        *reps += 1;
        *probability += (certainty.abs() / *reps as f64) * (certainty.signum() - *probability);
    }

    pub fn likely_ordering(&mut self, a: &CardValue, b: &CardValue) -> Option<(CardValue, CardValue)> {
        let (scale, a, b) = if a < b { (1.0, a, b) } else { (-1.0, b, a) };

        let probability = self.seen.entry((*a, *b)).or_insert((0.0, 0)).0 * scale;
        if probability > 0.0 {
            Some((*a, *b))
        } else if probability < 0.0 {
            Some((*b, *a))
        } else {
            None
        }
    }
}