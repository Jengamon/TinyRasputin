use crate::skeleton::cards::{CardValue};
use crate::into_ordering;
use itertools::Itertools;
use std::collections::HashSet;
// use crate::debug_println;

trait CloneableIterator: Iterator + Clone {}

// Tells us the actual index the value was inserted, and checks for ordering errors by panicking (TODO don't panick)
pub fn relationships(relations: &[(CardValue, CardValue)], valuev: &CardValue) 
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
        // debug_println!("Progress in chaining [{}] vs [{}]", 
        //     potential_cycles.iter().map(|x| x.iter().format(" -> ")).format(", "),
        //     cycles.iter().map(|x| x.iter().format(" -> ")).format(", "));
        while !potential_cycles.is_empty() {
            let check: Vec<CardValue> = potential_cycles.pop().unwrap();
            assert!(!check.is_empty());
            //debug_println!("Checking {}", check.iter().format(" -> "));
            let first = check.iter().nth(0).unwrap();
            let last = check.iter().last().unwrap();
            let mut links: Vec<_> = values.iter().filter(|(_, (pre, _))| pre.to_owned().any(|x| &x == last)).map(|(v, _)| *v).collect();
            links.dedup();
            for link in &links {
                //debug_println!("Checking link {} ...", link);
                let mut chain = check.clone();
                if link == first {
                    let chainh = chain.iter().copied().collect::<HashSet<_>>();
                    if !cycles.iter().map(|x| x.iter().copied().collect::<HashSet<_>>()).any(|x| x == chainh) {
                        chain.push(*first);
                        cycles.push(chain);
                        // Keep only the longest cycles
                        cycles = cycles.into_iter().fold(vec![], |candidates, cycle| {
                            // We know a longer cycle
                            if candidates.iter().any(|x| x.iter().all(|x| cycle.contains(&x)) && x.len() > cycle.len()) {
                                candidates
                            } else {
                                // We are the longest chain of our loop
                                let mut longest_unique: Vec<_> = candidates.into_iter().filter(|x| !x.iter().all(|x| cycle.contains(&x))).collect();
                                longest_unique.push(cycle);
                                longest_unique
                            }
                        });
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

    // TODO Change return types to handle possibility of cycles
    
    fn remove_redundancies(&self) -> Vec<(CardValue, CardValue)>  {
        remove_redundancies(self)
    }

    fn simplify(&self) -> Vec<(CardValue, CardValue)>  {
        simplify(self)
    }

    fn possibilities(&self) -> u64 {
        assert!(detect_cycles(self).len() == 0);
        let ordering = into_ordering!("2,3,4,5,6,7,8,9,T,J,Q,K,A");
        ordering.iter().fold((1, vec![]), |(count, mut seen), cv| {
            // We don't know the relative ordering of these relationships, so just always count them
            let possible: Vec<_> = ordering.iter().filter(|value| {
                let (mut pre, mut post, _) = relationships(&self, value);
                pre.all(|x| seen.contains(&x)) && !post.any(|x| seen.contains(&x))
            }).cloned().collect();
            let value = possible.len().saturating_sub(seen.len());
            let value = if value == 0 { 1 } else { value };
            // debug_println!("Possible Len: {} ({}) [{}]", possible.len(), seen.len(), value);
            seen.push(*cv);
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

pub fn generate_ordering(relations: &[(CardValue, CardValue)]) -> [CardValue; 13] {
    let cycles = detect_cycles(relations);
    assert!(cycles.len() == 0, "Detected cycles\n{}", cycles.into_iter().map(|x| x.into_iter().format(" -> ")).format("\n"));
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
    // debug_println!("Generation Rules:\n{}", relations.debug_relations());
    for index in 0..13 {
        let valid = original
            .iter()
            .filter_map(|(v, (pre, post))| {
                if (pre.clone().all(|x| new[..index].contains(&x)) && post.clone().all(|x| !new[..index].contains(&x))) && !new[..index].contains(v) {
                    Some(*v)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        // debug_println!("Valid: [{}]", valid.iter().format(", "));
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
        assert_eq!(failed, vec![], "Failed goals detected: {}", failed.iter().map(|x| format!("[{}]", x.iter().format(", "))).format(", "));
        assert_eq!(extra, vec![], "Extra cycles detected: {}", extra.iter().map(|x| format!("[{}]", x.iter().format(", "))).format(", "));
        assert_eq!(*intersection, *goals);
    }
}

#[test]
fn relations_ext_test() {
    let test = vec![];
    let test2 = vec![(CardValue::Two, CardValue::Ace)];
    let test3 = vec![(CardValue::Two, CardValue::Ace), (CardValue::Three, CardValue::Ace)];
    assert_eq!(test.possibilities(), 13 * 12 * 11 * 10 * 9 * 8 * 7 * 6 * 5 * 4  * 3 * 2);
    assert_eq!(test2.possibilities(), 12 * 12 * 11 * 10 * 9 * 8 * 7 * 6 * 5 * 4  * 3 * 2);
    assert_eq!(test3.possibilities(), 12 * 11 * 11 * 10 * 9 * 8 * 7 * 6 * 5 * 4  * 3 * 2);
}