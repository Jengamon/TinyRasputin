use crate::skeleton::cards::{CardValue};
use crate::into_ordering;
use itertools::Itertools;
use std::collections::HashSet;

trait CloneableIterator: Iterator + Clone {}

// Tells us the actual index the value was inserted, and checks for ordering errors by panicking (TODO don't panick)
fn relationships(relations: &[(CardValue, CardValue)], valuev: &CardValue) 
    -> (impl Iterator<Item=CardValue> + Clone, 
        impl Iterator<Item=CardValue> + Clone, 
        impl Iterator<Item=CardValue> + Clone) {
    let relations = relations.to_vec();
    let value = *valuev;
    // The value must preceed these values.
    let post = relations.clone().into_iter().filter(move |(a, _)| value == *a).map(|(_, b)| b);
    // The value must follow these values.
    //let value = *valuev;
    let pre = relations.clone().into_iter().filter(move |(_, b)| value == *b).map(|(a, _)| a);
    let mut postv = post.clone();
    let violations = pre.clone().filter(move |x| postv.any(|y| x == &y));
    
    (pre, post, violations)
}

// Returns all cycles found
pub fn detect_cycles(relations: &[(CardValue, CardValue)]) -> Vec<Vec<CardValue>> {
    let values: Vec<_> = relations.iter().flat_map(|(a, b)| vec![a, b])
        .fold(vec![], |mut a, v| {
            if !a.contains(v) {
                a.push(*v);
            }
            a
        })
        .into_iter()
        .map(|v| {
            let (pre, post, _) = relationships(relations, &v);
            (v, (pre, post))
        })
        .collect();
    let mut cycles: Vec<Vec<CardValue>> = vec![];
    for (v, _) in &values {
        let mut potential_cycles = vec![vec![*v]];
        // println!("Progress in chaining [{}] vs [{}]", 
        //     potential_cycles.iter().map(|x| x.iter().format(" -> ")).format(", "),
        //     cycles.iter().map(|x| x.iter().format(" -> ")).format(", "));
        while !potential_cycles.is_empty() {
            let check: Vec<CardValue> = potential_cycles.pop().unwrap();
            assert!(!check.is_empty());
            //println!("Checking {}", check.iter().format(" -> "));
            let first = check.iter().nth(0).unwrap();
            let last = check.iter().last().unwrap();
            let mut links: Vec<_> = values.iter().filter(|(_, (pre, _))| pre.to_owned().any(|x| &x == last)).map(|(v, _)| *v).collect();
            links.dedup();
            for link in &links {
                //println!("Checking link {} ...", link);
                let mut chain = check.clone();
                if link == first {
                    let chainh = chain.iter().copied().collect::<HashSet<_>>();
                    if !cycles.iter().map(|x| x.iter().copied().collect::<HashSet<_>>()).any(|x| x == chainh) {
                        chain.push(*first);
                        cycles.push(chain);
                    }
                } else if !chain.iter().any(|x| x == link) { // No internal cycles allowed
                    chain.push(*link);
                    potential_cycles.push(chain);
                }
            }
        }
    }
    cycles
}

pub trait RelationsExt {
    fn debug_relations(&self) -> String;
    fn remove_redundancies(&self) -> Vec<(CardValue, CardValue)>;
    fn simplify(&self) -> Vec<(CardValue, CardValue)>;
    // Count the number of possibilities this DAG provides
    fn possibilities(&self) -> u64;
}

impl RelationsExt for [(CardValue, CardValue)] {
    fn debug_relations(&self) -> String {
        format!("{}", into_ordering!("2,3,4,5,6,7,8,9,T,J,Q,K,A").iter().map(|v| (v, relationships(self, v)))
        .map(|(v, (pre, post, _))| format!("|{}|[{}][{}]", v, pre.format(""), post.format(""))).format("\n"))
    }

    fn remove_redundancies(&self) -> Vec<(CardValue, CardValue)>  {
        remove_redundancies(self)
    }

    fn simplify(&self) -> Vec<(CardValue, CardValue)>  {
        simplify(self)
    }

    fn possibilities(&self) -> u64 {
        assert!(detect_cycles(self).len() == 0);
        let ordering = into_ordering!("2,3,4,5,6,7,8,9,T,J,Q,K,A");
        (0..ordering.len()).into_iter().fold((1, vec![]), |(count, mut seen), card| {
            // We don't know the relative ordering of these relationships, so just always count them
            let possible: Vec<_> = ordering.iter().filter(|value| {
                let (mut pre, _, _) = relationships(&self, value);
                pre.all(|x| seen.contains(&x))
            }).cloned().collect();
            for value in &possible {
                if !seen.contains(value) {
                    seen.push(*value);
                }
            }
            let value = possible.len().saturating_sub(card);
            assert!(value > 0);
            (count * value as u64, seen)
        }).0
    }
}

// Detects and removes redundant information in a non-cyclical graph
// Link redundancies tell us something, so keep them
#[allow(dead_code)]
fn simplify(relations: &[(CardValue, CardValue)]) -> Vec<(CardValue, CardValue)> {
    let mut new = vec![];
    'loope: for (from, goal) in relations.remove_redundancies().iter() {
        let mut potential_paths = vec![vec![*from]];
        while !potential_paths.is_empty() {
            let path = potential_paths.pop().unwrap();
            let last = path.iter().last().unwrap();
            for link in relations.iter().filter(|rel| *rel != &(*from, *goal)).filter(|(a, _)| a == last).map(|(_, b)| *b) {
                let mut chain = path.clone();
                if &link == goal {
                    continue 'loope
                } else {
                    chain.push(link);
                    potential_paths.push(chain);
                }
            }
        }
        new.push((*from, *goal));
    }
    new
}

// Removes duplicates
fn remove_redundancies(relations: &[(CardValue, CardValue)]) -> Vec<(CardValue, CardValue)> {
    let mut new = vec![];
    for (from, goal) in relations.iter() {
        let item = (*from, *goal);
        if !new.contains(&item) {
            new.push(item);
        }
    }
    new
}

fn linearity_scores(relations: &[(CardValue, CardValue)], (a, b): &(CardValue, CardValue)) -> (usize, usize) {
    let a_relevant = relationships(relations, &a).1;
    let b_relevant = relationships(relations, &b).0;

    let mut a_confirm = vec![];
    let mut b_confirm = vec![];

    let mut pa_rel = a_relevant.collect::<Vec<_>>();
    let mut pb_rel = b_relevant.collect::<Vec<_>>();

    while !pa_rel.is_empty() {
        let value = pa_rel.pop().unwrap();
        for link in relationships(relations, &value).0 {
            if !a_confirm.contains(&link) {
                a_confirm.push(link);
            }
            pa_rel.push(link);
        }
    }

    while !pb_rel.is_empty() {
        let value = pb_rel.pop().unwrap();
        for link in relationships(relations, &value).1 {
            if !b_confirm.contains(&link) {
                b_confirm.push(link);
            }
            pb_rel.push(link);
        }
    }

    (a_confirm.len(), b_confirm.len())
}

fn resolve_cycles_using_probability(relations: &[(CardValue, CardValue)], cycles: &[Vec<CardValue>]) -> Vec<(CardValue, CardValue)> {
    let participants = cycles.iter().flat_map(|x| x.windows(2)).map(|x| (x[0], x[1])).collect::<Vec<_>>().remove_redundancies();

    // All alternatives start equally likely
    let prior = participants.iter().map(|x| (x, 1.0 / participants.len() as f64));

    // Use Bayesian inference to determine how good each rule is
    // P(F | D) = P(D | F) * P(F) / P(D)
    // D = the graph so far is correct
    // F = this item is correct
    let prob_fact = participants.iter().map(|x| {
        let (score_a, score_b) = linearity_scores(relations, x);
        if score_a == 0 || score_b == 0 {
            1.0
        } else {
            let min = std::cmp::min(score_a, score_b) as f64;
            let total = (score_a + score_b) as f64;
            min / total
        }
    });

    let new = prior.zip(prob_fact).map(|((i, p_df), p_f)| (i, p_df * p_f));

    let p_d: f64 = new.clone().map(|(_, p_df_f)| p_df_f).sum();

    let new = new.map(|(i, p_df_f)| (i, p_df_f / p_d));

    let mut sorted_probs = new.clone().map(|(_, p_fd): (_, f64)| p_fd).fold(vec![], |mut a, v| {
        if let Some(position) = a.iter().position(|x| x > &v) {
            a.insert(position, v);
        } else {
            a.push(v);
        }
        a
    });
    sorted_probs.dedup();

    let new = new.inspect(|((a, b), p_fd)| println!("Relation {} -> {}: P({})", a, b, p_fd));

    let answer = sorted_probs.into_iter().filter_map(|x| {
        let iter = new.clone().filter(move |(_, p_fd)| p_fd > &x);
        if iter.clone().count() > 0 {
            Some(iter.map(|(v, _)| v))
        } else {
            None
        }
    }).find(|iter| {
        let mut rels = relations.to_vec();
        rels.extend(iter.clone());
        detect_cycles(&rels).len() == 0
    })
    .map(|iter| iter.cloned().collect())
    .unwrap_or(vec![]);
    println!("P(D) = {}", p_d);
    println!("Answer = {}", answer.iter().map(|(a, b)| format!("{} -> {}", a, b)).format(", "));
    answer
}

// Resolves a cycle by eliminating the least agreeable rule
// A rule is agreeable (a1 -> b1) with (a2 -> b2) if either its a1 == a2 or b1 == b2
// A cycle is a Vec where [n, a, b, c, ..., n]
// The relations are assumed to have all cycles removed
pub fn resolve_cycles(relations: &[(CardValue, CardValue)], cycles: &[Vec<CardValue>]) -> Vec<(CardValue, CardValue)> {
    resolve_cycles_using_probability(relations, cycles)
}

pub fn generate_ordering(relations: &[(CardValue, CardValue)]) -> [CardValue; 13] {
    let original = into_ordering!("2,3,4,5,6,7,8,9,T,J,Q,K,A").to_vec();
    let mut new = [CardValue::Two; 13];
    let gen_index = || -> usize {
        use rand::prelude::*;
        let mut rng = rand::thread_rng();
        for i in 0..13 {
            if rng.gen_bool(0.25) {
                return i
            }
        }
        0 // The most likely result. In case we fail every time, just given the most given result
    };
    let mut original: Vec<_> = original.into_iter().map(|v| (v, relationships(relations, &v)))
        .map(|(v, (pre, post, viol))| {
            assert!(viol.clone().count() == 0, "Something is both pre and post: {}", viol.format(", "));
            (v, (pre, post))
        })
        .collect();
    for index in 0..13 {
        let valid = original
            .iter()
            .filter_map(|(v, (pre, post))| {
                if (pre.clone().all(|x| new[..index].contains(&x)) && !post.clone().any(|x| new[..index].contains(&x))) && !new[..index].contains(v) {
                    Some(*v)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        // println!("Valid: [{}]", valid.iter().format(", "));
        // let selected = rand::random::<usize>() % valid.len();
        let selected = gen_index() % valid.len();
        let value = valid[selected];
        let original_pos = original.iter().position(|(v, _)| v == &value).unwrap();
        let (_, (pre, post)) = original.remove(original_pos);
        drop(pre);
        drop(post);
        new[index] = value;
    }
    new
}

// test for correctness
#[test]
pub fn cycles_test() {
    let relations_to_test = [
        (vec![(CardValue::Three, CardValue::Ace), (CardValue::Ace, CardValue::King), (CardValue::King, CardValue::Three)], vec![vec![CardValue::Three, CardValue::Ace, CardValue::King]]), 
        (vec![(CardValue::Two, CardValue::Ace), (CardValue::Ace, CardValue::Two)], vec![vec![CardValue::Ace, CardValue::Two]]),
        (vec![(CardValue::Two, CardValue::Ace), (CardValue::Ace, CardValue::Two), (CardValue::Three, CardValue::Two)], vec![vec![CardValue::Ace, CardValue::Two]]),
        (vec![(CardValue::Two, CardValue::Ace)], vec![]),
        (vec![(CardValue::Two, CardValue::Ace), (CardValue::Ace, CardValue::Two), (CardValue::Three, CardValue::Two), (CardValue::Two, CardValue::Three)], vec![vec![CardValue::Ace, CardValue::Two], vec![CardValue::Two, CardValue::Three]]),
        (vec![
            (CardValue::Two, CardValue::Three), 
            (CardValue::Three, CardValue::Four), 
            (CardValue::Four, CardValue::Five), 
            (CardValue::Three, CardValue::Five),
            (CardValue::Five, CardValue::Two)],
        vec![
            vec![CardValue::Two, CardValue::Three, CardValue::Four, CardValue::Five],
            vec![CardValue::Two, CardValue::Three, CardValue::Five]
        ]),
    ];

    for (rels, goals) in relations_to_test.into_iter() {
        let goals = goals.iter().map(|x| x.iter().copied().collect::<HashSet<_>>()).collect::<Vec<HashSet<CardValue>>>();
        let detected = detect_cycles(rels).iter().map(|x| x.iter().copied().collect::<HashSet<_>>()).collect::<Vec<_>>();
        let intersection: Vec<_> = goals.iter().filter(|x| detected.contains(x)).cloned().collect();
        let failed: Vec<_> = goals.iter().filter(|x| !detected.contains(x)).cloned().collect();
        let extra: Vec<_> = detected.iter().filter(|x| !goals.contains(x)).cloned().collect();
        assert_eq!(failed, vec![], "Failed goals detected: {}", extra.iter().map(|x| format!("[{}]", x.iter().format(", "))).format(", "));
        assert_eq!(extra, vec![], "Extra cycles detected: {}", extra.iter().map(|x| format!("[{}]", x.iter().format(", "))).format(", "));
        assert_eq!(*intersection, *goals);
    }
}

// test for correctness
#[test]
pub fn cycles_resolution_test() {
    let relations_to_test = [
        (vec![(CardValue::Three, CardValue::Ace), (CardValue::Ace, CardValue::King), (CardValue::King, CardValue::Three)], vec![]), 
        (vec![(CardValue::Two, CardValue::Ace), (CardValue::Ace, CardValue::Two)], vec![]),
        (vec![(CardValue::Two, CardValue::Ace), (CardValue::Ace, CardValue::Two), (CardValue::Three, CardValue::Two)], vec![(CardValue::Three, CardValue::Two)]),
        (vec![(CardValue::Two, CardValue::Ace)], vec![(CardValue::Two, CardValue::Ace)]),
        (vec![(CardValue::Two, CardValue::Ace), (CardValue::Ace, CardValue::Two), (CardValue::Three, CardValue::Two), (CardValue::Two, CardValue::Three)], vec![]),
        (vec![
            (CardValue::Two, CardValue::Three), 
            (CardValue::Three, CardValue::Four), 
            (CardValue::Four, CardValue::Five), 
            (CardValue::Three, CardValue::Five),
            (CardValue::Five, CardValue::Two)],
        vec![]),
    ];

    for (rels, goal) in relations_to_test.into_iter() {
        let mut rels = rels.to_vec();
        let goal = goal.iter().cloned().collect::<HashSet<_>>();
        let detected = detect_cycles(&rels);
        // Remove all cyclical rules
        for cycle in detected.iter() {
            for pair in cycle.windows(2) {
                let position = rels.iter().position(|x| x == &(pair[0], pair[1]));
                if let Some(position) = position {
                    rels.remove(position);
                }
            }
        }
        let add_rules = resolve_cycles(&rels, &detected);
        let detected = rels.iter().cloned().chain(add_rules.into_iter()).collect::<HashSet<_>>();
        assert_eq!(detected, goal);
    }
}

#[test]
fn relations_ext_test() {
    let test = vec![];
    let test2 = vec![(CardValue::Two, CardValue::Ace)];
    assert_eq!(test.possibilities(), 13 * 12 * 11 * 10 * 9 * 8 * 7 * 6 * 5 * 4  * 3 * 2);
    assert_eq!(test2.possibilities(), 12 * 12 * 11 * 10 * 9 * 8 * 7 * 6 * 5 * 4  * 3 * 2);
}