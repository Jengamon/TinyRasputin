use criterion::{criterion_group, criterion_main, Criterion, Throughput, BenchmarkId};
use itertools::Itertools;
use tinyrasputin::{
    engine::{
        showdown::{ShowdownEngine},
        relations::{detect_cycles},
    },
    skeleton::cards::{CardValue, Card, CardSuit},
    into_ordering,
};
use std::collections::HashSet;
// use rand::prelude::*;

pub fn showdown_benchmark(c: &mut Criterion) {
    let values = into_ordering!("2,3,4,5,6,7,8,9,T,J,Q,K,A");
    let suits = vec![CardSuit::Hearts, CardSuit::Diamonds, CardSuit::Clubs, CardSuit::Spades];
    let showdown = ShowdownEngine::new(values.clone());

    for cards in (0..100).into_iter().map(|_| {
        // There's only a point to generating hands up to 7
        let size: usize = rand::random::<usize>() % 6 + 2;
        (0..size).into_iter().map(|_| {
            let value = rand::random::<usize>() % values.len();
            let suit = rand::random::<usize>() % suits.len();
            Card::new(suits[suit], values[value])
        }).collect::<Vec<_>>()
    }) {
        let mut cards = ShowdownEngine::make_hand_unique(cards.into_iter());
        cards.sort_by(|a, b| a.cmp(b));

        let mut group = c.benchmark_group("Showdown Engine");
        let (mut hand, mut hand_all) = (None, None);

        group.throughput(Throughput::Elements(cards.len() as u64));

        group.bench_with_input(BenchmarkId::new("Shortcut", cards.iter().format(", ")), &cards, |b, cards| b.iter(|| {
            hand = Some(showdown.process_hand(&cards));
        }));

        group.bench_with_input(BenchmarkId::new("Complete", cards.iter().format(", ")), &cards, |b, cards| b.iter(|| {
            hand_all = Some(showdown.process_hand_all(&cards));
        }));

        let (hand, hand_all) = (hand.unwrap(), hand_all.unwrap());

        assert_eq!(hand, hand_all, "Shortcut said {}, while Complete said {}", hand, hand_all);

        let (hand, hand_all) = (None, None);

        group.bench_with_input(BenchmarkId::new("Shortcut No Straights", cards.iter().format(", ")), &cards, |b, cards| b.iter(|| {
            hand = Some(showdown.process_hand_no_straight(&cards));
        }));

        group.bench_with_input(BenchmarkId::new("Complete No Straights", cards.iter().format(", ")), &cards, |b, cards| b.iter(|| {
            hand_all = Some(showdown.process_hand_no_straight_all(&cards));
        }));

        let (hand, hand_all) = (hand.unwrap(), hand_all.unwrap());

        assert_eq!(hand, hand_all, "Shortcut No Straights said {}, while Complete No Straights said {}", hand, hand_all);

        group.finish();
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

criterion_group!(benches, showdown_benchmark, cycles_benchmark);
criterion_main!(benches);