#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    use tinyrasputin::{
        into_ordering,
        engine::showdown::ShowdownEngine,
        skeleton::cards::{Card, CardValue},
    };
    use itertools::Itertools;
    use std::cmp::Ordering;

    // fuzzed code goes here
    let ordering = into_ordering!(chars "23456789TJQKA");
    let showdown = ShowdownEngine::new(ordering);
    let possible_cards = String::from_utf8_lossy(data).split(",").try_fold(vec![], |acc, card| {
        if let Ok(card) = card.parse::<Card>() {
            Some(acc.into_iter().chain(vec![card].into_iter()).collect())
        } else {
            None
        }
    });
    if let Some(cards) = possible_cards {
        // Only test strings where the cards are unique and bias towards real data
        let unique_cards = ShowdownEngine::make_hand_unique(cards.iter());
        if unique_cards.len() == cards.len() && cards.len() < 8 && cards.len() > 1 {
            let possible_hands = showdown.all_possible_hands(&cards, true);
            let hands = showdown.potential_hands(&cards, true);
            println!("Potential Hands:\n{}", hands.iter().format("\n"));
            println!("Possible Hands:\n{}", possible_hands.iter().format("\n"));
            for hand in hands.iter() {
                assert!(possible_hands.contains(&hand), "Hand should not be potential: {}", hand);
            }
            let best_hand_detected = showdown.process_hand(&cards);
            let best_hand_possible = showdown.process_hand_all(&cards);
            let engine_comparison = showdown.compare_potential_hands(&best_hand_detected, &best_hand_possible);
            assert_eq!(engine_comparison, Ordering::Equal, "Detection contradiction: engine says {} is best, when {} is best", best_hand_detected, best_hand_possible);

            let possible_hands = showdown.all_possible_hands(&cards, false);
            let hands = showdown.potential_hands(&cards, false);
            println!("Potential Hands (no straights):\n{}", hands.iter().format("\n"));
            println!("Possible Hands (no straights):\n{}", possible_hands.iter().format("\n"));
            for hand in hands.iter() {
                assert!(possible_hands.contains(&hand), "Hand should not be potential: {}", hand);
            }
            let best_hand_detected = showdown.process_hand_no_straight(&cards);
            let best_hand_possible = showdown.process_hand_no_straight_all(&cards);
            let engine_comparison = showdown.compare_potential_hands(&best_hand_detected, &best_hand_possible);
            assert_eq!(engine_comparison, Ordering::Equal, "Detection contradiction: engine says {} is best, when {} is best", best_hand_detected, best_hand_possible);
        }
    }
});
