//! Works more with detecting which order is more probable based on how many times we have seen a fact
use crate::{
    skeleton::cards::CardValue,
    engine::relations::{relationships, detect_cycles},
};
use std::collections::HashMap;
use std::cell::RefCell;
use itertools::Itertools;
use approx::{relative_eq, relative_ne};
use std::collections::HashSet;
use std::convert::AsRef;
use std::cmp::Ordering;

const CONFIRMATION_THRESHOLD: f64 = 0.5;
const EPSILON: f64 = std::f64::EPSILON;

#[derive(Debug, Clone)]
pub struct ProbabilityEngine {
    seen: RefCell<HashMap<(CardValue, CardValue), (f64, usize, Vec<(String, f64)>)>>,
    inconsistent_rules: RefCell<HashSet<String>>,
}

impl ProbabilityEngine {
    pub fn new() -> ProbabilityEngine {
        ProbabilityEngine {
            seen: RefCell::new(HashMap::new()),
            inconsistent_rules: RefCell::new(HashSet::new()),
        }
    }

    // Avoid using certainty 1.0 or -1.0
    // Also, the more relations we see, the weaker the certainty becomes
    pub fn update<B: AsRef<str>>(&mut self, name: B, a: &CardValue, b: &CardValue, certainty: f64) -> bool {
        println!("[ProbEngine] Called with {} -> {} (certainty {})", a, b, certainty);
        assert!(name.as_ref() != "", "Cannot name a rule an empty string");
        self._update(name, a, b, certainty)
    }

    // Returns whether or not we actually added the rule
    fn _update<B: AsRef<str>>(&self, name: B, a: &CardValue, b: &CardValue, certainty: f64) -> bool {
        use rand::Rng;
        assert!(certainty > -1.0 && certainty < 1.0);

        let mut rng = rand::thread_rng();

        if a != b && certainty != 0.0 && !self.inconsistent_rules.borrow().contains(name.as_ref()) {
            let (certainty, a, b) = if a < b { (certainty, a, b) } else { (-certainty, b, a) };

            // let relations: Vec<_> = self.probabilities().into_iter().filter_map(|((a, b), p)| if p > 0.0 { Some((a, b)) } else if p < 0.0 { Some((b, a)) } else { None }).collect();
            // let transitive = relations.iter().filter(|(a2, b2)| {
            //     let (mut pre, _, _) = relationships(&relations, &a2);
            //     let (_, mut post, _) = relationships(&relations, &b2);
            //     post.any(|x| x == *b) && pre.any(|x| x == *a) && !(a == a2 && b == b2)
            // }).inspect(|(a, b)| println!("Transitively updating {} -> {} with probability {}", a, b, certainty));

            // for (a, b) in transitive {
            //     self._update(a, b, certainty);
            // }

            let mut seen = self.seen.borrow_mut();
            let (probability, reps, names) = seen.entry((*a, *b)).or_insert((0.0, 0, vec![]));
            *reps += 1;
            *probability += certainty.abs() * (if *reps > 1 {rng.gen_range(1, *reps)} else {1} as f64  / *reps as f64) * (certainty.signum() - *probability);
            let pair = (name.as_ref().to_string(), certainty);
            if !names.contains(&pair) {
                names.push(pair);
            }

            assert!(!probability.is_nan());
            true
        } else {
            false
        }
    }

    pub fn likely_ordering(&self, a: &CardValue, b: &CardValue) -> Option<(CardValue, CardValue)> {
        let (scale, a, b) = if a < b { (1.0, a, b) } else { (-1.0, b, a) };

        let mut seen = self.seen.borrow_mut();
        let probability = seen.entry((*a, *b)).or_insert((0.0, 0, vec![])).0 * scale;
        if probability > 0.0 {
            Some((*a, *b))
        } else if probability < 0.0 {
            Some((*b, *a))
        } else {
            None
        }
    }

    pub fn probability(&self, a: &CardValue, b: &CardValue) -> f64 {
        let (scale, a, b) = if a < b { (1.0, a, b) } else { (-1.0, b, a) };

        self.seen.borrow().get(&(*a, *b)).map(|(p, _, _)| p * scale).unwrap_or(0.0)
    }

    pub fn probabilities(&self) -> Vec<((CardValue, CardValue), f64)> {
        self.seen.borrow().iter().map(|(k, (v, _, _))| (*k, *v)).collect()
    }

    pub fn get_rules(&self, a: &CardValue, b: &CardValue) -> Vec<(String, f64)> {
        let (a, b) = if a < b { (a, b) } else { (b, a) };
        self.seen.borrow().get(&(*a, *b)).map(|(_, _, n)| n.clone()).unwrap_or(vec![])
    }

    pub fn relations(&self) -> Vec<(CardValue, CardValue)> {
        let mut probs: Vec<_> = self.probabilities().into_iter().map(|((a, b), p)| if p < 0.0 { ((b, a), -p) } else { ((a, b), p) }).collect();
        // Invariant is the fact that no probability should be NaN
        probs.sort_by(|(_, p1), (_, p2)| {
            p1.partial_cmp(&p2).unwrap()
        });
        let guess = |a, b, p: f64| {
            assert!(p >= 0.0);

            if !relative_eq!(p, 0.0, epsilon = EPSILON) {
                use rand::Rng;
                let mut rng = rand::thread_rng();

                let (a, b, p) = if rng.gen_bool(1.0 - p) {
                    (b, a, 1.0 - p)
                } else {
                    (a, b, p)
                };

                if relative_eq!(p, 0.0, epsilon = EPSILON) {
                    // println!("Using relationship {} -> {} with no confidence", a, b);
                } else {
                    // println!("Using relationship {} -> {} with confidence {}", a, b, p);
                }
                ((a, b), p)
            } else {
                // We have no information on their relative ordering, so flip a coin
                if rand::random() {
                    // println!("Using relationship {} -> {} with no confidence", a, b);
                    ((a, b), 0.0)
                } else {
                    // println!("Using relationship {} -> {} with no confidence", b, a);
                    ((b, a), 0.0)
                }
            }
        };

        // Make the most likely rules go first
        let proposal: Vec<((CardValue, CardValue), f64)> = probs.iter().copied()
            .rev()
            .fold(vec![], |mut proposal: Vec<((CardValue, CardValue), f64)>, ((a, b), p)| {
                assert!(p >= 0.0);
                // If we already selected a certain direction, make sure to confirm that direction
                let (pre_a, post_a, viol_a) = relationships(&probs.iter().copied().map(|(rel, _)| rel).collect::<Vec<_>>(), &a);
                let (pre_b, post_b, viol_b) = relationships(&probs.iter().copied().map(|(rel, _)| rel).collect::<Vec<_>>(), &b);
                assert!(viol_a.clone().count() == 0, "Something is both pre and post: {}", viol_a.format(""));
                assert!(viol_b.clone().count() == 0, "Something is both pre and post: {}", viol_b.format(""));
                let pre_ab = pre_a.clone().filter(|x| pre_b.clone().any(|y| x == &y));
                let post_ab = post_a.clone().filter(|x| post_b.clone().any(|y| x == &y));
                // If b.pre contains a or a.post contains b, follow that
                // Otherwise, try to maximize pre_ab and post_ab
                if pre_b.clone().any(|x| x == a) || post_a.clone().any(|x| x == b) {
                    // Find the highest confidence, add epsilon to it, and use that (confirmation confidence)
                    let mx = proposal.iter().copied()
                        .filter(|_| pre_b.clone().any(|x| x == a) || post_a.clone().any(|x| x == b)) // Pertinent relations
                        .filter(|((a2, b2), _)| post_a.clone().any(|x| &x == b2) && pre_b.clone().any(|x| &x == a2)) // Do they really confirm our relation? ( are they in between? )
                        .map(|(_, v)| v)
                        .max_by(|a, b| a.partial_cmp(b).unwrap());
                    if let Some(mx) = mx {
                        if mx > CONFIRMATION_THRESHOLD {
                            // println!("Using relationship {} -> {} with confidence {}", a, b, mx + EPSILON);
                            proposal.push(((a, b), mx + EPSILON));
                        } else {
                            // if mx != 0.0 { // We don't generate relations that we aren't confident in at all
                            //     // println!("Could generate relation {} -> {}, but fails confidence check...({})", a, b, mx);
                            // }
                            proposal.push(guess(a, b, p));
                        }
                    } else {
                        proposal.push(guess(a, b, p));
                    }
                } else {
                    // NOTE: We assume that the events -> A & -> B and B -> & A -> are independant
                    // Probabilistically push either a -> b || b -> a depending on which is more likely based on the facts we have proposed
                    let pre_a_or_b = pre_a.chain(pre_b.clone());
                    let post_a_or_b = post_a.chain(post_b.clone());
                    let p_post_a_pre_b = p;
                    let pre_b = pre_a_or_b.clone().chain(pre_ab.clone()).fold(vec![], |mut acc, v| {
                        if !acc.contains(&v) {
                            acc.push(v);
                        }
                        acc
                    });
                    let pre_b_total = pre_b.len() as i64;
                    let pre_b_count = pre_ab.clone().fold(0, |count, item| if pre_a_or_b.clone().all(|x| x != item) {
                        count - 1
                    } else if pre_ab.clone().all(|x| x != item) { // Don't double count stuff
                        count + 1
                    } else {
                        count
                    }) + pre_b_total;
                    let post_a = post_a_or_b.clone().chain(post_ab.clone()).fold(vec![], |mut acc, v| {
                        if !acc.contains(&v) {
                            acc.push(v);
                        }
                        acc
                    });
                    let post_a_total = post_a.len() as i64;
                    let post_a_count = post_ab.clone().fold(0, |count, item| if post_a_or_b.clone().all(|x| x != item) {
                        count - 1
                    } else if post_ab.clone().all(|x| x != item) { // Don't double count stuff
                        count + 1
                    } else {
                        count
                    }) + post_a_total;

                    // println!("State: P(-> {1}) = {2} / {3} P({0} ->) = {4} / {5} P({0} -> | -> {1}) = {6}", a, b, pre_b_count, pre_b_total, post_a_count, post_a_total, p_post_a_pre_b);
                    // println!("Relations pre {0}&{1} [{2}] pre{0}|{1} [{3}] post {0}|{1} [{4}] post {0}&{1} [{5}]", a, b,
                        // pre_ab.clone().format(""), pre_a_or_b.clone().format(""), post_a_or_b.clone().format(""), post_ab.clone().format(""));
                    let p = if pre_b_total == 0 {
                        // B is an orphan or initial (we have no information on what is in front of B)
                        p_post_a_pre_b
                    } else if pre_b_total != 0 && post_a_total == 0 {
                        // A says nothing is after it, but B has things that it goes before
                        // Check is B says A is before it, if it is, this would create a loop else, A is just an orphan
                        if pre_b.iter().any(|x| x == &a) {
                            0.0 // We have no confidence of the relative ordering of a -> b, so just pick one with no confidence
                        } else {
                            p_post_a_pre_b
                        }
                    } else if post_a_total != 0 && pre_b_total != 0 {
                        // We have relatively "complete" knowledge of A -> and -> B
                        assert!(post_a_count.abs() <= post_a_total);
                        assert!(pre_b_count.abs() <= pre_b_total);
                        let p_pre_b = pre_b_count as f64 / pre_b_total as f64;
                        let p_post_a = post_a_count as f64 / post_a_total as f64;
                        let sign = if p_pre_b.signum() == -1.0 {
                            -1.0
                        } else {
                            p_post_a.signum()
                        };
                        p_post_a_pre_b * p_pre_b.abs() / p_post_a.abs() * sign
                    } else {
                        unreachable!("LOGIC ERROR")
                    };

                    // println!("P(-> {1} | {0} ->) = {2}", a, b, p);

                    let (p, a, b) = if p < 0.0 {
                        (-p, b, a)
                    } else {
                        (p, a, b)
                    };

                    assert!(p >= 0.0);

                    proposal.push(guess(a, b, p));
                }
                proposal
            }).into_iter().inspect(|(_, p)| assert!(p >= &0.0)).collect();

        let mut final_ = proposal;
        // Eliminate loops, starting from our least confident guesses
        let mut run_count = 0;
        while detect_cycles(&final_.iter().map(|v| v.0).collect::<Vec<_>>()).len() > 0 {
            let cycles = detect_cycles(&final_.iter().map(|v| v.0).collect::<Vec<_>>());
            println!("===========RUN COUNT {}===========", run_count);
            for cycle in &cycles {
                println!("Detected cycle {}", cycle.iter().format(" -> "));
            }
            run_count += 1;
            println!("=================================");
            let cyclical = cycles.into_iter().flat_map(|cycle| cycle.windows(2).map(|pair| (pair[0], pair[1])).collect::<Vec<_>>()).collect::<Vec<_>>();
            let highest_cyclical_confidence = cyclical.iter().flat_map(|rel| final_.iter().find(|(frel, _)| frel == rel).map(|(_, conf)| conf)).max_by(|a, b| a.partial_cmp(b).unwrap());
            let lowest_confidence = final_.iter().map(|(_, conf)| conf).min_by(|a, b| a.partial_cmp(&b).unwrap());
            if let (Some(highest_cyclical_confidence), Some(lowest_confidence)) = (highest_cyclical_confidence, lowest_confidence) {
                let (problematic, okay): (Vec<_>, Vec<_>) = final_.iter()
                    .partition(|((a, b), confidence)| (cyclical.contains(&(*a, *b)) || cyclical.contains(&(*b, *a))) && relative_eq!(confidence, highest_cyclical_confidence, epsilon = EPSILON));
                println!("Okay rules {}", okay.iter().map(|((a, b), p)| format!("{0} -({1:.2})> {2}",a,p, b)).format(", "));
                println!("Problematic rules {}", problematic.iter().map(|((a, b), p)| format!("{0} -({1:.2})> {2}",a,p, b)).format(", "));
                println!("Highest cyclical rule {0:.2} Lowest rule {1:.2}", highest_cyclical_confidence, lowest_confidence);
                for ((a, b), p) in problematic {
                    let prob = self.probabilities().into_iter().find(|(rel, _)| if a < b { rel == &(a, b) } else { rel == &(b, a) }).map(|x| if a < b { x.1 } else { -x.1 });
                    let confidence = p * (1.0 - lowest_confidence / highest_cyclical_confidence);
                    assert!(confidence >= 0.0, "Erroneous confidence {} detected", confidence);
                    if let Some(prob) = prob {
                        // An invariant should be that confidence <= prob, if it isn't. We should panic~
                        if relative_ne!(confidence / prob.abs(), 1.0, epsilon = 0.1) {
                            // We are generating inconsistent information, and we were so sure. Wipe the relation
                            println!("Problematic cyclical relation {0} -> {1} formed with confidence {2:.2} and actual prob {3:.2} (calculated prob {4:.2}). Rule conflict. Wiping rule...", a, b, prob, confidence, p);
                            let key = if a < b { (a, b) } else { (b, a) };
                            // If we were more confident than the calculated probabilty, a rule is going haywire
                            // We now ignore it
                            if p > confidence {
                                // We were more confident than the actual probability
                                // We will now ignore the strongest rule if it is biased more than a random toss in a -> b direction
                                let rules = self.get_rules(&a, &b).into_iter().filter(|(name, _)| !name.is_empty()).collect::<Vec<_>>();
                                println!("Production rules: {}", rules.iter().map(|(name, strength)| format!("[{} P({})]", name, strength)).format(", "));
                                if rules.len() >= 2 {
                                    // Ignore the strongest rule
                                    if let Some((rule, _)) = rules.into_iter().max_by(|(_, a), (_, b)| match a.abs().partial_cmp(&b.abs()) {
                                        Some(Ordering::Equal) | None => a.signum().partial_cmp(&b.signum()).unwrap(),
                                        Some(other) => other,
                                    }) {
                                        let mut inc_rules = self.inconsistent_rules.borrow_mut();
                                        let pre_len = inc_rules.len();
                                        let intersection = &vec![rule.clone()].into_iter().collect::<HashSet<_>>() | &inc_rules;
                                        *inc_rules = intersection;
                                        let post_len = inc_rules.len();
                                        if post_len > pre_len {
                                            println!("Marking rule {} as inconsistant.", rule);
                                        }
                                        
                                    } else {
                                        unreachable!("Problematic rule {} -> {} appeared out of the ether", a, b);
                                    }
                                } else if rules.len() == 1 {
                                    // The rule simply got it incorrect, not inconsistant with other results
                                    println!("Problematic pair {} -> {} was randomly generated by rule [{}]", a, b, rules[0].0);
                                }
                            }
                            self.seen.borrow_mut().remove(&key);
                        } else { // Our confidence is either equal to or less than our probability. Let's not panic yet, just fix if possible.
                            println!("Problematic cyclical relation {0} -> {1} formed with prob {2:.2}. Weak rule conflict or wrong information generation is possible. Weakening rule...", a, b, prob.abs());
                            self._update("", &a, &b, -prob.abs());
                        }
                    } else {
                        println!("Problematic cyclical relation {0} -> {1} was randomly formed. Just ignore.", a ,b);
                    }
                }
                final_ = okay;
            } else {
                unreachable!("Cycle was detected, even though rules should be empty rules = {}, cyclical = {}", final_.iter().map(|((a, b), p)| format!("{0} -({1:.2})> {2}",a,p, b)).format(", "), cyclical.iter().map(|(a, b)| format!("{} -> {}",a, b)).format(", "))
            }
        }
        final_.into_iter().map(|v| v.0).collect()
    }
}
