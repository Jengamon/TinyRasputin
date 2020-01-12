use criterion::{criterion_group, criterion_main, Criterion};
use itertools::Itertools;
use tinyrasputin::{
    engine::{
        showdown::{ShowdownEngine, ShowdownHand, ShowdownSet},
        relations::{detect_cycles, resolve_cycles},
    },
    skeleton::cards::{CardValue, Card},
    into_cards, into_ordering,
};
use std::collections::HashSet;

pub fn showdown_benchmark(c: &mut Criterion) {
    let showdown = ShowdownEngine::new(into_ordering!("2,3,4,5,6,7,8,9,T,J,Q,K,A"));
    let situations = [
        // Real situation benching
        ("Ac, As, Ad, Qs, Ks, Js, Ts", ShowdownHand::RoyalFlush(into_cards!("As, Ks, Qs, Js, Ts"))),
        // Coherency benching
        ("2c,2h,Ac,Ah", ShowdownHand::TwoPair(into_cards!("2c,2h,Ac,Ah"))), 
        ("2c,2h,Ac,Ah,As", ShowdownHand::FullHouse(into_cards!("2c,2h,Ac,Ah,As"))),
        ("2c,Ac,2h,Ah,As", ShowdownHand::FullHouse(into_cards!("As,Ah,Ac,2c,2h"))),
        ("2c,3c,4c,5c,7c", ShowdownHand::Flush(into_cards!("2c,3c,4c,5c,7c"))),
        ("Tc,Jc,Kc,Qc,Ac", ShowdownHand::RoyalFlush(into_cards!("Tc,Jc,Qc,Kc,Ac"))),
        ("Ac,2c,3c,4c,5c", ShowdownHand::StraightFlush(into_cards!("Ac,3c,5c,2c,4c"))),
        ("6c,2c", ShowdownHand::HighCard(into_cards!("6c")[0])),
    ];

    for (cards, goal) in situations.into_iter() {
        let cards = into_cards!(cards);
        c.bench_function(&format!("Testing {} for {}", cards.iter().format(", "), goal), |b| {
            b.iter(|| {
                let hand = showdown.process_hand(&cards);
                assert_eq!(ShowdownSet(showdown, hand), ShowdownSet(showdown, goal.clone()));
            });
        });
    }
}

pub fn cycles_benchmark(c: &mut Criterion) {
    let relations_to_test = [
        // Coherency benching
        (vec![(CardValue::Three, CardValue::Ace), (CardValue::Ace, CardValue::King), (CardValue::King, CardValue::Three)], vec![vec![CardValue::Three, CardValue::Ace, CardValue::King]]), 
        (vec![(CardValue::Two, CardValue::Ace), (CardValue::Ace, CardValue::Two)], vec![vec![CardValue::Ace, CardValue::Two]]),
        (vec![(CardValue::Two, CardValue::Ace), (CardValue::Ace, CardValue::Two), (CardValue::Three, CardValue::Two)], vec![vec![CardValue::Ace, CardValue::Two]]),
        (vec![(CardValue::Two, CardValue::Ace)], vec![]),
        (vec![(CardValue::Two, CardValue::Ace), (CardValue::Ace, CardValue::Two), (CardValue::Three, CardValue::Two), (CardValue::Two, CardValue::Three)], vec![vec![CardValue::Ace, CardValue::Two], vec![CardValue::Two, CardValue::Three]]),

    ];

    for (rels, goals) in relations_to_test.into_iter() {
        let goals = goals.iter().map(|x| x.iter().cloned().collect::<HashSet<_>>()).collect::<Vec<HashSet<CardValue>>>();
        c.bench_function(&format!("Testing ({}) for cycles [{}]", 
            rels.iter().map(|(a, b)| format!("{} -> {}", a, b)).format(", "), 
            goals.iter().map(|x| x.iter().format(", ")).format("; ")), |b| {
                b.iter(|| {
                    let detected = detect_cycles(rels).iter().map(|x| x.iter().copied().collect::<HashSet<_>>()).collect::<Vec<_>>();
                    let intersection: Vec<_> = goals.iter().filter(|x| detected.contains(x)).cloned().collect();
                    let failed: Vec<_> = goals.iter().filter(|x| !detected.contains(x)).cloned().collect();
                    let extra: Vec<_> = detected.iter().filter(|x| !goals.contains(x)).cloned().collect();
                    assert_eq!(failed, vec![], "Failed goals detected: {}", extra.iter().map(|x| format!("[{}]", x.iter().format(", "))).format(", "));
                    assert_eq!(extra, vec![], "Extra cycles detected: {}", extra.iter().map(|x| format!("[{}]", x.iter().format(", "))).format(", "));
                    assert_eq!(*intersection, *goals);
                });
        });
    }
}

pub fn cycles_resolution_benchmark(c: &mut Criterion) {
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

    for (relsa, goal) in relations_to_test.into_iter() {
        let mut rels = relsa.to_vec();
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
        c.bench_function(&format!("Testing resolution of [{}] to [{}]", 
            relsa.iter().map(|(a, b)| format!("{} -> {}", a, b)).format(", "), 
            goal.iter().map(|(a, b)| format!("{} -> {}", a, b)).format(", ")), |b| {
                b.iter(|| {
                    let add_rules = resolve_cycles(&rels, &detected);
                    let detected = rels.iter().cloned().chain(add_rules.into_iter()).collect::<HashSet<_>>();
                    assert_eq!(detected, goal);
                });
        });
    }
}

criterion_group!(benches, showdown_benchmark, cycles_benchmark, cycles_resolution_benchmark);
criterion_main!(benches);