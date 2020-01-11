use crate::skeleton::cards::{CardValue};
use crate::into_ordering;
use itertools::Itertools;
use std::collections::HashSet;


// Tells us the actual index the value was inserted, and checks for ordering errors by panicking (TODO don't panick)
fn relationships(relations: &[(CardValue, CardValue)], value: &CardValue) -> (Vec<CardValue>, Vec<CardValue>, Vec<CardValue>) {
    // The value must preceed these values.
    let post = relations.iter().filter(|(a, _)| value == a).map(|(_, b)| *b).collect::<Vec<_>>();
    // The value must follow these values.
    let pre = relations.iter().filter(|(_, b)| value == b).map(|(a, _)| *a).collect::<Vec<_>>();
    let violations = pre.iter().filter(|x| post.contains(x)).cloned().collect::<Vec<_>>();
    
    (pre, post, violations)
}

// Returns all cycles found
pub fn detect_cycles(relations: &[(CardValue, CardValue)]) -> Vec<Vec<CardValue>> {
    let mut values: Vec<_> = relations.iter().flat_map(|(a, b)| vec![a, b]).map(|v| (v, relationships(relations, v)))
        .map(|(v, (pre, post, _))| (v, (pre, post)))
        .collect();
    values.dedup();
    let mut cycles: Vec<Vec<CardValue>> = vec![];
    for (v, _) in &values {
        let mut potential_cycles = vec![vec![**v]];
        // println!("Progress in chaining [{}] vs [{}]", 
        //     potential_cycles.iter().map(|x| x.iter().format(" -> ")).format(", "),
        //     cycles.iter().map(|x| x.iter().format(" -> ")).format(", "));
        while !potential_cycles.is_empty() {
            let check: Vec<CardValue> = potential_cycles.pop().unwrap();
            assert!(!check.is_empty());
            //println!("Checking {}", check.iter().format(" -> "));
            let first = check.iter().nth(0).unwrap();
            let last = check.iter().last().unwrap();
            let mut links: Vec<_> = values.iter().filter(|(_, (pre, _))| pre.contains(last)).map(|(v, _)| *v).collect();
            links.dedup();
            for link in &links {
                //println!("Checking link {} ...", link);
                let mut chain = check.clone();
                if *link == first {
                    let chainh = chain.iter().copied().collect::<HashSet<_>>();
                    if !cycles.iter().map(|x| x.iter().copied().collect::<HashSet<_>>()).any(|x| x == chainh) {
                        chain.push(*first);
                        cycles.push(chain);
                    }
                } else if !chain.iter().any(|x| x == *link) { // No internal cycles allowed
                    chain.push(**link);
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
}

impl RelationsExt for [(CardValue, CardValue)] {
    fn debug_relations(&self) -> String {
        format!("{}", into_ordering!("2,3,4,5,6,7,8,9,T,J,Q,K,A").into_iter().map(|v| (v, relationships(self, v)))
        .map(|(v, (pre, post, _))| format!("|{}|[{}],[{}]", v, pre.iter().format(","), post.iter().format(","))).format("\n"))
    }

    fn remove_redundancies(&self) -> Vec<(CardValue, CardValue)>  {
        remove_redundancies(self)
    }
}

// Detects and removes redundancies in a non-cyclical graph
fn remove_redundancies(relations: &[(CardValue, CardValue)]) -> Vec<(CardValue, CardValue)> {
    let mut new = vec![];
    'loope: for (from, goal) in relations.iter() {
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
        let item = (*from, *goal);
        if !new.contains(&item) {
            new.push((*from, *goal));
        }
    }
    new
}

// Resolves a cycle by eliminating the least agreeable rule
// A rule is agreeable (a1 -> b1) with (a2 -> b2) if either its a1 == a2 or b1 == b2
// A cycle is a Vec where [n, a, b, c, ..., n]
// The relations are assumed to have all cycles removed
pub fn resolve_cycles(relations: &[(CardValue, CardValue)], cycles: &[Vec<CardValue>]) -> Vec<(CardValue, CardValue)> {
    // println!("{}", relations.debug_relations());
    let participants: Vec<_> = cycles.iter().flat_map(|x| x.windows(2)).map(|x| (x[0], x[1])).map(|(a, b)| { 
        let mut a_relevant: Vec<CardValue> = relationships(relations, &a).1;
        let mut b_relevant: Vec<CardValue> = relationships(relations, &b).0;

        let mut pa_rel = a_relevant.clone();
        let mut pb_rel = b_relevant.clone();

        while !pa_rel.is_empty() {
            let value = pa_rel.pop().unwrap();
            for link in relationships(relations, &value).0 {
                if !a_relevant.contains(&link) {
                    a_relevant.push(link);
                }
                pa_rel.push(link);
            }
        }

        while !pb_rel.is_empty() {
            let value = pb_rel.pop().unwrap();
            for link in relationships(relations, &value).1 {
                if !b_relevant.contains(&link) {
                    b_relevant.push(link);
                }
                pb_rel.push(link);
            }
        }
        
        ((a, b), (a_relevant.len(), b_relevant.len()))
    })
    //.inspect(|(_, (a, b))| println!("[{}][{}]", a, b))
    .collect();

    let sorted_counts: Vec<_> = participants.iter().flat_map(|(_, (a, b))| vec![a, b]).fold(vec![], |mut a, v| {
        if let Some(position) = a.iter().position(|x| x > &v) {
            a.insert(position, v);
        } else {
            a.push(v);
        }
        a
    });

    sorted_counts.into_iter().filter_map(|x| if participants.iter().filter(|(_, (a, b))| a == x || b == x).count() == participants.len() {
        None
    } else {
        Some(participants.iter().filter(|(_, (a, b))| a != x && b != x).map(|(val, _)| val).cloned().collect::<Vec<_>>())
    }).find(|val| {
        let mut rels = relations.to_vec();
        rels.extend(val.iter());
        detect_cycles(&rels).len() == 0
    }).unwrap_or(vec![])
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
        .inspect(|(_, (_, _, viol))| assert!(viol.len() == 0, "Something is both pre and post: {}", viol.iter().format(", ")))
        .map(|(v, (pre, post, _))| (v, (pre, post)))
        .collect();
    for index in 0..13 {
        let valid = original
            .iter()
            .filter(|(v, (pre, post))| (pre.iter().all(|x| new[..index].contains(x)) && !post.iter().any(|x| new[..index].contains(x))) && !new[..index].contains(v))
            .map(|(v, _)| *v).collect::<Vec<_>>();
        // println!("Valid: [{}]", valid.iter().format(", "));
        // let selected = rand::random::<usize>() % valid.len();
        let selected = gen_index() % valid.len();
        let value = valid[selected];
        let original_pos = original.iter().position(|(v, _)| *v == value).unwrap();
        original.remove(original_pos);
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
        (vec![(CardValue::Two, CardValue::Ace), (CardValue::Ace, CardValue::Two), (CardValue::Three, CardValue::Two)], vec![(CardValue::Two, CardValue::Ace), (CardValue::Three, CardValue::Two)]),
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