use super::super::skeleton::{
    cards::{CardValue, Card, CardSuit},
};

use std::cmp::{PartialEq, Eq, PartialOrd, Ord, Ordering};
use std::fmt;
#[allow(unused_imports)]
use itertools::Itertools;
use std::collections::HashSet;
use std::borrow::Borrow;

#[macro_export]
macro_rules! into_ordering {
    ($t:expr) => {{
        let order: Vec<_> = $t.split(",").map(|x| x.parse::<CardValue>()).fold(vec![], |mut a, c| {
            match c {
                Ok(c) => {
                    let contains = a.iter().any(|x| match x {
                        Ok(x) => *x == c,
                        Err(_) => false
                    });
                    if !contains {
                        a.push(Ok(c));
                    }
                },
                Err(e) => a.push(Err(e))
            };
            a
        });
        into_ordering!(vec order)
    }};
    (chars $t:expr) => {{
        let order: Vec<_> = $t.chars().map(|x| x.to_string().parse::<CardValue>()).fold(vec![], |mut a, c| {
            match c {
                Ok(c) => {
                    let contains = a.iter().any(|x| match x {
                        Ok(x) => *x == c,
                        Err(_) => false
                    });
                    if !contains {
                        a.push(Ok(c));
                    }
                },
                Err(e) => a.push(Err(e))
            };
            a
        });
        into_ordering!(vec order)
    }};
    (vec $t:expr) => {{
        let order = $t;
        assert!(order.len() == 13);
        order.into_iter().enumerate().try_fold([CardValue::Two; 13], |mut ordering, (i, val)| match val {
            Ok(val) => { ordering[i] = val; Ok(ordering) },
            Err(e) => Err(e)
        }).unwrap()
    }};
}

/// Valid hands that will win a game
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Hand {
    RoyalFlush(HashSet<Card>),
    StraightFlush(HashSet<Card>),
    FourOfAKind(HashSet<Card>),
    FullHouse(HashSet<Card>),
    Flush(HashSet<Card>),
    Straight(HashSet<Card>),
    ThreeOfAKind(HashSet<Card>),
    TwoPair(HashSet<Card>),
    Pair(HashSet<Card>)
}

impl Hand {
    pub fn cards(&self) -> HashSet<Card> {
        match self {
            Hand::RoyalFlush(a) => a.clone(),
            Hand::StraightFlush(a) => a.clone(),
            Hand::FourOfAKind(a) => a.clone(),
            Hand::FullHouse(a) => a.clone(),
            Hand::Flush(a) => a.clone(),
            Hand::Straight(a) => a.clone(),
            Hand::ThreeOfAKind(a) => a.clone(),
            Hand::TwoPair(a) => a.clone(),
            Hand::Pair(a) => a.clone()
        }
    }
}

impl fmt::Display for Hand {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Hand::RoyalFlush(a) => write!(fmt, "[RoyalFlush {}]", a.iter().format(" ")),
            Hand::StraightFlush(a) => write!(fmt, "[StraightFlush {}]", a.iter().format(" ")),
            Hand::FourOfAKind(a) => write!(fmt, "[FourKind {}]", a.iter().format(" ")),
            Hand::FullHouse(a) => write!(fmt, "[FullHouse {}]", a.iter().format(" ")),
            Hand::Flush(a) => write!(fmt, "[Flush {}]", a.iter().format(" ")),
            Hand::Straight(a) => write!(fmt, "[Straight {}]", a.iter().format(" ")),
            Hand::ThreeOfAKind(a) => write!(fmt, "[ThreeKind {}]", a.iter().format(" ")),
            Hand::TwoPair(a) => write!(fmt, "[TwoPair {}]", a.iter().format(" ")),
            Hand::Pair(a) => write!(fmt, "[Pair {}]", a.iter().format(" "))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StraightDrawType {
    Complete,
    OpenEnded,
    Inside
}

// Either a potential hand, or an actual formed hand
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PotentialHand {
    Hand(Hand), // An actual hand that can win
    StraightDraw(HashSet<Card>, StraightDrawType), // A potential straight with a hole inside
    // Don't mess with backhand straight draws, so don't even detect them
    FlushDraw(HashSet<Card>), // A potential flush with 1 missing card.
    StraightFlushDraw(HashSet<Card>, StraightDrawType), // A straight
    RoyalFlushDraw(HashSet<Card>, StraightDrawType),
    HighCard(Card)
}

impl fmt::Display for PotentialHand {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PotentialHand::Hand(a) => write!(fmt, "[Winning Hand {}]", a),
            PotentialHand::StraightDraw(a, typ) => write!(fmt, "[StraightDraw {} ({:?})]", a.iter().format(" "), typ),
            PotentialHand::StraightFlushDraw(a, typ) => write!(fmt, "[StraightFlushDraw {} ({:?})]", a.iter().format(" "), typ),
            PotentialHand::RoyalFlushDraw(a, typ) => write!(fmt, "[RoyalFlushDraw {} ({:?})]", a.iter().format(" "), typ),
            PotentialHand::FlushDraw(a) => write!(fmt, "[FlushDraw {}]", a.iter().format(" ")),
            PotentialHand::HighCard(a) => write!(fmt, "[HighCard {}]", a),
        }
    }
}

impl PotentialHand {
    pub fn cards(&self) -> HashSet<Card> {
        match self {
            PotentialHand::Hand(hand) => hand.cards(),
            PotentialHand::StraightDraw(draw, _) => draw.clone(),
            PotentialHand::StraightFlushDraw(draw, _) => draw.clone(),
            PotentialHand::RoyalFlushDraw(draw, _) => draw.clone(),
            PotentialHand::FlushDraw(draw) => draw.clone(),
            PotentialHand::HighCard(card) => vec![*card].into_iter().collect(),
        }
    }

    pub fn showdown(&self) -> Option<Hand> {
        match self {
            PotentialHand::Hand(hand) => Some(hand.clone()),
            PotentialHand::StraightDraw(_, _) => None,
            PotentialHand::StraightFlushDraw(_, _) => None,
            PotentialHand::RoyalFlushDraw(_, _) => None,
            PotentialHand::FlushDraw(_) => None,
            PotentialHand::HighCard(_) => None
        }
    }
}

/// Detects possible and best hands out of a given set of cards
/// NOTE: Behavior for `potential_hands` or `all_possible_hands` is undefined if passed hand contains duplicate cards, so be sure to call
/// ShowdownEngine::make_hand_unique on any potential hands you try to pass in if you can't guarantee that
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShowdownEngine {
    ordering: [CardValue; 13]
}

/* Poker hands are (high to low):
Royal flush - Top 5 of ordering, same suit
Straight flush - 5 in order, same suit
Four-of-a-kind - 4 cards same value
Full house - Three-of-a-kind and a pair
Flush - 5 cards of any suit
Straight - 5 in order, not same suit
Three-of-a-kind - 3 cards same value
Two-pair - 2 different pairs
Pair - 2 cards same value
High Card - None of the above. Card of highest value
*/

macro_rules! detect_hands {
    ($slf:expr, $hand:expr) => {{
        let detected_straights = $slf.detect_straights($hand);
        let detected_flushes = $slf.detect_flushes($hand);

        let detected_straight_flushes = detected_straights.clone().into_iter().filter_map(|(straight, typ)| if detected_flushes.contains(&straight) {
            Some((straight, typ))
        } else {
            None
        }).collect::<Vec<_>>();

        let detected_four_of_a_kind = $slf.detect_of_a_kind($hand, 4);
        let detected_three_of_a_kind = $slf.detect_of_a_kind($hand, 3);
        let detected_pairs = $slf.detect_of_a_kind($hand, 2);

        // Arrange all possible combinations of detected hands
        let detected_hands: Vec<HashSet<Card>> = detected_straights.iter().cloned().map(|(hand, _)| hand)
            .chain(detected_flushes.iter().cloned())
            .chain(detected_four_of_a_kind.iter().cloned())
            // Full Houses and 3K
            .chain(detected_three_of_a_kind.iter().flat_map(|toak| {
                detected_pairs.clone().into_iter().map(move |pair| toak.clone().into_iter().chain(pair.clone().into_iter()).collect::<HashSet<_>>())
            }))
            .chain(detected_three_of_a_kind.iter().cloned())
            // Two pair and pairs
            .chain(detected_pairs.iter().flat_map(|pair| {
                detected_pairs.clone().into_iter().filter_map(move |opair| if &opair != pair {
                    Some(opair.into_iter().chain(pair.clone().into_iter()).collect::<HashSet<_>>())
                } else {
                    None
                })
            }))
            .chain(detected_pairs.iter().cloned())
            .collect();
        (detected_hands, detected_pairs, detected_three_of_a_kind, detected_four_of_a_kind, detected_straights, detected_flushes, detected_straight_flushes)
    }};
    (no straights $slf:expr, $hand:expr) => {{
        let detected_flushes = $slf.detect_flushes($hand);

        let detected_four_of_a_kind = $slf.detect_of_a_kind($hand, 4);
        let detected_three_of_a_kind = $slf.detect_of_a_kind($hand, 3);
        let detected_pairs = $slf.detect_of_a_kind($hand, 2);

        // Arrange all possible combinations of detected hands
        let detected_hands: Vec<HashSet<Card>> = detected_flushes.iter().cloned()
            .chain(detected_four_of_a_kind.iter().cloned())
            // Full Houses and 3K
            .chain(detected_three_of_a_kind.iter().flat_map(|toak| {
                detected_pairs.clone().into_iter().map(move |pair| toak.clone().into_iter().chain(pair.clone().into_iter()).collect::<HashSet<_>>())
            }))
            .chain(detected_three_of_a_kind.iter().cloned())
            // Two pair and pairs
            .chain(detected_pairs.iter().flat_map(|pair| {
                detected_pairs.clone().into_iter().filter_map(move |opair| if &opair != pair {
                    Some(opair.into_iter().chain(pair.clone().into_iter()).collect::<HashSet<_>>())
                } else {
                    None
                })
            }))
            .chain(detected_pairs.iter().cloned())
            .collect();
        (detected_hands, detected_pairs, detected_three_of_a_kind, detected_four_of_a_kind, vec![], detected_flushes, vec![])
    }};
}

macro_rules! process_hands {
    ($slf:expr, $hands:expr) => {
        $hands.into_iter().fold(None, |best_hand: Option<PotentialHand>, hand| match best_hand {
            Some(best_hand) => vec![best_hand, hand].into_iter().max_by(|a, b| $slf.compare_potential_hands(&a, &b)),
            None => Some(hand)
        })
    }
}

impl ShowdownEngine {
    pub fn new(ordering: [CardValue; 13]) -> ShowdownEngine {
        ShowdownEngine {
            ordering
        }
    }

    pub fn make_hand_unique<'a, H, C: Borrow<Card>>(hand: H) -> Vec<Card> where H: 'a + Iterator<Item=C> {
        hand.fold(vec![], |mut acc, card| {
            if !acc.contains(card.borrow()) {
                acc.push(card.borrow().clone());
            }
            acc
        })
    }

    // Only for consistency checking
    pub fn all_possible_hands(&self, hand: &[Card], straights: bool) -> Vec<PotentialHand> {
        // Brutely detect all hands, so every 4K will have 3 pairs, every 3K will have 2 pair and so on
        let (hands, pairs, three_of_a_kind, four_of_a_kind, straights, flushes, straight_flushes) = if straights {
            detect_hands!(self, hand)
        } else {
            detect_hands!(no straights self, hand)
        };
        hands.into_iter().flat_map(|hand| {
            // Four of a Kinds
            four_of_a_kind.iter().filter_map(|x| if x.is_subset(&hand.iter().copied().collect()) {
                Some(PotentialHand::Hand(Hand::FourOfAKind(x.clone())))
            } else {
                None
            })
            // Full Houses and Three of a Kinds
            .chain(three_of_a_kind.iter().filter_map(|x| if x.is_subset(&hand.iter().copied().collect()) {
                if let Some(y) = pairs.iter().find(|y| !y.is_subset(&hand)) {
                    Some(PotentialHand::Hand(Hand::FullHouse(x | y)))
                } else {
                    Some(PotentialHand::Hand(Hand::ThreeOfAKind(x.clone())))
                }
            } else {
                None
            }))
            // Pairs and Two Pairs
            .chain(pairs.iter().filter_map(|x| if x.is_subset(&hand.iter().copied().collect()) {
                if let Some(y) = pairs.iter().find(|y| y.is_subset(&hand) && &x != y) {
                    Some(PotentialHand::Hand(Hand::TwoPair(x | y)))
                } else {
                    Some(PotentialHand::Hand(Hand::Pair(x.clone())))
                }
            } else {
                None
            }))
            // Straights and Straight Draws
            .chain(straights.iter().filter_map(|(x, typ)| if x.is_subset(&hand.iter().copied().collect()) {
                if typ == &StraightDrawType::Complete {
                    Some(PotentialHand::Hand(Hand::Straight(x.clone())))
                } else {
                    Some(PotentialHand::StraightDraw(x.clone(), *typ))
                }
            } else {
                None
            }))
            // Flush and Flush Draws
            .chain(flushes.iter().filter_map(|x| if x.is_subset(&hand.iter().copied().collect()) {
                if x.len() == 5 {
                    Some(PotentialHand::Hand(Hand::Flush(x.clone())))
                } else {
                    Some(PotentialHand::FlushDraw(x.clone()))
                }
            } else {
                None
            }))
            .chain(straight_flushes.iter().filter_map(|(x, typ)| if x.is_subset(&hand.iter().copied().collect()) {
                if typ == &StraightDrawType::Complete {
                    if self.highest_card_value(x.iter()) == self.ordering[12] {
                        Some(PotentialHand::Hand(Hand::RoyalFlush(x.clone())))
                    } else {
                        Some(PotentialHand::Hand(Hand::StraightFlush(x.clone())))
                    }
                } else {
                    if self.highest_card_value(x.iter()) == self.ordering[12] || self.highest_card_value(x.iter()) == self.ordering[11] {
                        Some(PotentialHand::RoyalFlushDraw(x.clone(), *typ))
                    } else {
                        Some(PotentialHand::StraightFlushDraw(x.clone(), *typ))
                    }
                }
            } else {
                None
            }))
            .collect::<Vec<_>>()
        }).fold(vec![], |mut acc, hand| {
            if !acc.contains(&hand) {
                acc.push(hand);
            }
            acc
        })
    }

    // We don't detect high cards, because those are technically not a "potential" hand, but rather when we have no other hands
    // and we might want to react differently if we have potential straights or flushes
    // Tries to detect the best possible hand for a given set of cards
    pub fn potential_hands(&self, hand: &[Card], straights: bool) -> Vec<PotentialHand> {
        let (hands, pairs, three_of_a_kind, four_of_a_kind, straights, flushes, straight_flushes) = if straights {
            detect_hands!(self, hand)
        } else {
            detect_hands!(no straights self, hand)
        };

        macro_rules! best_hand {
            ($hands_iter:expr) => {
                $hands_iter.iter().fold(HashSet::new(), |seen, hand| {
                    &seen | &hand
                })
            };
        };

        macro_rules! hands {
            ($hand:expr, $subset_array:expr) => {
                $subset_array.iter().filter(|x| x.is_subset(&$hand.iter().copied().collect()))
            };

            (straight $hand:expr, $subset_array:expr) => {
                $subset_array.iter().cloned().filter(|(x, _)| x.is_subset(&$hand.iter().copied().collect()))
            }
        }

        // println!("Hand: [{}]", best_hand!(hands).into_iter().format(", "));

        let hand = best_hand!(hands);
        if hand.len() > 0 {
            // Start from the bottom and go up!
            let pairs: Vec<_> = hands!(hand, pairs).collect();
            let straight_flushes: Vec<_> = hands!(straight hand, straight_flushes).collect();
            let mut straights: Vec<(HashSet<_>, _)> = hands!(straight hand, straights).collect();
            straights = straights.into_iter().filter(|(straight, _)| !straight_flushes.iter().any(|(sf, _)| sf.is_superset(straight))).collect();
            straights.sort_unstable_by(|(a, _), (b, _)| b.len().cmp(&a.len()));
            let flushes: Vec<_> = hands!(hand, flushes).cloned().filter(|flush| !straight_flushes.iter().any(|(sf, _)| sf.is_superset(flush))).collect();
            let not_straight_flush_winning_hand: Vec<Hand> = if pairs.len() == 0 {
                // There are no pairs, and thus no 3K, 2P, or 4K
                // The only possibilities are HC STRAIGHT FLUSH STRAIGHT_FLUSH ROYAL_FLUSH and draws
                vec![].into_iter().collect()
            } else {
                // We have at least one pair, which beats out any draws
                // Do we have at least 2 pairs?
                if pairs.len() > 1 {
                    pairs.windows(2).flat_map(|pairs| {
                        let toaks: Vec<_> = hands!(hand, three_of_a_kind).collect();
                        let foaks: Vec<_> = hands!(hand, four_of_a_kind).collect();
                        let pairs = pairs.to_vec();
                        foaks.into_iter().cloned().map(Hand::FourOfAKind)
                            .chain(toaks.into_iter().flat_map(|toak| {
                                let toak_value = toak.iter().map(|x| x.value()).collect::<Vec<_>>()[0];
                                let possible_full_house_pairs: Vec<_> = pairs.iter().filter(|x| !x.iter().any(|card| card.value() == toak_value)).collect();
                                if possible_full_house_pairs.len() > 0 {
                                    // We have a Full House
                                    possible_full_house_pairs.into_iter().map(|pair| Hand::FullHouse(toak | pair)).collect()
                                } else {
                                    // We have a Three of a Kind
                                    vec![Hand::ThreeOfAKind(toak.clone())]
                                }
                            }))
                            .chain(vec![Hand::TwoPair(pairs[0] | pairs[1]), Hand::Pair(pairs[0].clone()), Hand::Pair(pairs[1].clone())].into_iter()).collect::<Vec<_>>()
                    }).collect()
                } else {
                    // We only have 1 pair
                    vec![Hand::Pair(pairs[0].clone())]
                }
            };
            straight_flushes.iter().filter_map(|(sf, _)| if sf.len() == 5 {
                if self.highest_card_value(sf) == self.ordering[12] {
                    Some(PotentialHand::Hand(Hand::RoyalFlush(sf.clone())))
                } else {
                    Some(PotentialHand::Hand(Hand::StraightFlush(sf.clone())))
                }
            } else {
                None
            }).chain(not_straight_flush_winning_hand.iter().cloned().filter_map(|wh| match wh.clone() {
                Hand::FourOfAKind(..) | Hand::FullHouse(..) => Some(PotentialHand::Hand(wh.clone())),
                _ => None
            })).chain(flushes.iter().cloned().filter_map(|flush| if flush.len() == 5 {
                Some(PotentialHand::Hand(Hand::Flush(flush.clone())))
            } else {
                None
            })).chain(straights.iter().cloned().filter_map(|(straight, typ)| if typ == StraightDrawType::Complete {
                Some(PotentialHand::Hand(Hand::Straight(straight.clone())))
            } else {
                None
            })).chain(not_straight_flush_winning_hand.iter().cloned().map(PotentialHand::Hand))
            .chain(straight_flushes.iter().cloned().filter_map(|(sf, typ)| if typ != StraightDrawType::Complete {
                let hv = self.highest_card_value(&sf);
                if hv == self.ordering[12] || hv == self.ordering[11] {
                    Some(PotentialHand::RoyalFlushDraw(sf.clone(), typ))
                } else {
                    Some(PotentialHand::StraightFlushDraw(sf.clone(), typ))
                }
            } else {
                None
            }))
            .chain(flushes.iter().cloned().filter_map(|flush| if flush.len() > 5 {
                Some(PotentialHand::FlushDraw(flush))
            } else {
                None
            }))
            .chain(straights.iter().cloned().filter_map(|(straight, typ)| if typ != StraightDrawType::Complete {
                Some(PotentialHand::StraightDraw(straight.clone(), typ))
            } else {
                None
            }))
            // .inspect(|hand| println!("Found hand {}", hand))
            .fold((vec![], HashSet::new()), |(mut hands, seen), hand| {
                let cards = hand.cards();
                if !seen.is_superset(&cards) {
                    hands.push(hand);
                }
                (hands, &seen | &cards)
            }).0
        } else {
            // We have nothing, so say that
            vec![]
        }
    }

    // Don't use this in practice. Only used for consistency checking of the engine
    pub fn process_hand_no_straight_all(&self, hand: &[Card]) -> PotentialHand {
        let hand = ShowdownEngine::make_hand_unique(hand.iter());
        let hands = self.all_possible_hands(&hand, false);
        match process_hands!(self, hands) {
            Some(hand) => hand,
            None => PotentialHand::HighCard(self.highest_card(hand))
        }
    }

    pub fn process_hand_no_straight(&self, hand: &[Card]) -> PotentialHand {
        let hand = ShowdownEngine::make_hand_unique(hand.iter());
        let hands = self.potential_hands(&hand, false);
        // match hands.max_by(|a, b| process_hands!())
        match process_hands!(self, hands) {
            Some(hand) => hand,
            None => PotentialHand::HighCard(self.highest_card(hand))
        }
    }

    // Don't use this in practice. Only used for consistency checking of the engine
    pub fn process_hand_all(&self, hand: &[Card]) -> PotentialHand {
        let hand = ShowdownEngine::make_hand_unique(hand.iter());
        let hands = self.all_possible_hands(&hand, true);
        match process_hands!(self, hands) {
            Some(hand) => hand,
            None => PotentialHand::HighCard(self.highest_card(hand))
        }
    }

    pub fn process_hand(&self, hand: &[Card]) -> PotentialHand {
        let hand = ShowdownEngine::make_hand_unique(hand.iter());
        let hands = self.potential_hands(&hand, true);
        // match hands.max_by(|a, b| process_hands!())
        match process_hands!(self, hands) {
            Some(hand) => hand,
            None => PotentialHand::HighCard(self.highest_card(hand))
        }
    }

    fn detect_straights(&self, hand: &[Card]) -> Vec<(HashSet<Card>, StraightDrawType)> {
        let mut sorted_bins = [vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![]];
        for i in 1..14 {
            sorted_bins[i] = hand.iter().filter(|x| (i - 1) == self.ordering.iter().position(|y| *y == x.value()).unwrap()).collect();
        }
        sorted_bins[0] = sorted_bins[13].clone();
        // println!("Bins: {}", sorted_bins.iter().map(|x| format!("[{}]", x.iter().format(", "))).format(", "));
        sorted_bins.windows(5).flat_map(|x| {
            let holes = x.iter().filter(|x| x.is_empty()).count();
            // All 5 bins in a row are full, we have at least one straight
            if x.len() == 5 && holes == 0 {
                // Start with the last bin and go up from there
                let straights = x[4].iter().copied()
                    .map(|ele| vec![*ele].into_iter().collect::<HashSet<Card>>()).collect::<Vec<_>>().into_iter()
                    .flat_map(|set| x[3].iter().copied().map(move |ele| set.iter().copied().chain(vec![*ele].into_iter()).collect::<HashSet<_>>() ))
                    .flat_map(|set| x[2].iter().copied().map(move |ele| set.iter().copied().chain(vec![*ele].into_iter()).collect::<HashSet<_>>() ))
                    .flat_map(|set| x[1].iter().copied().map(move |ele| set.iter().copied().chain(vec![*ele].into_iter()).collect::<HashSet<_>>() ))
                    .flat_map(|set| x[0].iter().copied().map(move |ele| set.iter().copied().chain(vec![*ele].into_iter()).collect::<HashSet<_>>() ))
                    .fold(vec![], |mut acc, set| {
                        if !acc.contains(&set) {
                            acc.push(set);
                        }
                        acc
                    }).into_iter().map(|x| (x, StraightDrawType::Complete)).collect();
                straights
            } else if holes == 1 {
                // We have exactly 1 hole
                let open_ended = x[0].is_empty() || x[4].is_empty();
                let adjacent_x = x.into_iter().filter(|x| !x.is_empty()).collect::<Vec<_>>();
                // We know the length of adjacent_x is 3
                let straight_draws = adjacent_x[3].iter().copied()
                    .map(|ele| vec![*ele].into_iter().collect::<HashSet<Card>>()).collect::<Vec<_>>().into_iter()
                    .flat_map(|set| adjacent_x[2].iter().copied().map(move |ele| set.iter().copied().chain(vec![*ele].into_iter()).collect::<HashSet<_>>() ))
                    .flat_map(|set| adjacent_x[1].iter().copied().map(move |ele| set.iter().copied().chain(vec![*ele].into_iter()).collect::<HashSet<_>>() ))
                    .flat_map(|set| adjacent_x[0].iter().copied().map(move |ele| set.iter().copied().chain(vec![*ele].into_iter()).collect::<HashSet<_>>() ))
                    .fold(vec![], |mut acc, set| {
                        if !acc.contains(&set) {
                            acc.push(set);
                        }
                        acc
                    }).into_iter().map(|x| (x, if open_ended { StraightDrawType::OpenEnded } else { StraightDrawType::Inside })).collect();
                straight_draws
            } else if holes == 2 {
                let mut x = x.to_vec();
                x.dedup();
                // We check if we can count the 3 cards as remaining side by side
                if x.iter().fold(0, |adj_count, bin| if !bin.is_empty() { adj_count + 1 } else { 0 }) == 3 {
                    // This is a potential open ended straight draw. remove holes and create the sets
                    let adjacent_x = x.into_iter().filter(|x| !x.is_empty()).collect::<Vec<_>>();
                    // We know the length of adjacent_x is 3
                    let straight_draws = adjacent_x[2].iter().copied()
                        .map(|ele| vec![*ele].into_iter().collect::<HashSet<Card>>()).collect::<Vec<_>>().into_iter()
                        .flat_map(|set| adjacent_x[1].iter().copied().map(move |ele| set.iter().copied().chain(vec![*ele].into_iter()).collect::<HashSet<_>>() ))
                        .flat_map(|set| adjacent_x[0].iter().copied().map(move |ele| set.iter().copied().chain(vec![*ele].into_iter()).collect::<HashSet<_>>() ))
                        .fold(vec![], |mut acc, set| {
                            if !acc.contains(&set) {
                                acc.push(set);
                            }
                            acc
                        }).into_iter().map(|x| (x, StraightDrawType::OpenEnded)).collect();
                    straight_draws
                } else {
                    // This is defintely a backdoor hand, so we ignore it
                    vec![]
                }
            } else {
                // We don't detect any potential hands
                vec![]
            }
        }).collect()
    }

    fn detect_flushes(&self, hand: &[Card]) -> Vec<HashSet<Card>> {
        let mut sorted_bins = [vec![], vec![], vec![], vec![]];
        for i in 0..4 {
            sorted_bins[i] = hand.iter().filter(|x| i == match x.suit() {
                CardSuit::Spades => 0,
                CardSuit::Hearts => 1,
                CardSuit::Clubs => 2,
                CardSuit::Diamonds => 3,
            }).copied().collect();
        }

        sorted_bins.iter().cloned().flat_map(|x| x.windows(5).filter_map(|x| if x.len() >= 3 {
            Some(x.into_iter().copied().collect::<HashSet<_>>())
        } else {
            None
        }).collect::<Vec<_>>()).fold(vec![], |mut acc, set| {
            if !acc.contains(&set) {
                acc.push(set);
            }
            acc
        })
    }

    /// Detect all sets of cards with <number> or more cards in the hand
    fn detect_of_a_kind(&self, hand: &[Card], number: usize) -> Vec<HashSet<Card>> {
        let mut sorted_bins = [vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![]];
        for i in 0..13 {
            sorted_bins[i] = hand.iter().filter(|x| i == self.ordering.iter().position(|y| *y == x.value()).unwrap()).collect();
        }
        // println!("Bins: {}", sorted_bins.iter().map(|x| format!("[{}]", x.iter().format(", "))).format(", "));
        let potential_oak: Vec<HashSet<_>> = sorted_bins.iter().cloned().filter_map(|x| if x.len() >= number {
            Some(x.into_iter().cloned().collect())
        } else {
            None
        }).collect();
        let mut sets: Vec<HashSet<Card>> = vec![];
        for set in potential_oak.into_iter() {
            let set: Vec<Card> = set.into_iter().collect();
            for window in set.windows(number) {
                sets.push(window.into_iter().copied().collect());
            }
        }
        sets
    }

    pub fn value_order(&self, a: &CardValue, b: &CardValue) -> Ordering {
        let oa = self.ordering.iter().position(|x| x == a).unwrap();
        let ob = self.ordering.iter().position(|x| x == b).unwrap();
        oa.cmp(&ob)
    }

    fn highest_card<H, C: Borrow<Card> + Copy, I>(&self, hand: H) -> Card where H: IntoIterator<Item=C, IntoIter=I>, I: Iterator<Item=C> {
        *hand.into_iter()
            .map(|x| (x, self.ordering.iter().position(|y| *y == x.borrow().value())))
            .max_by(|x, y| x.1.cmp(&y.1))
            .expect("Expected non-empty hand").0.borrow()
    }

    pub fn highest_card_value<H, C: Borrow<Card> + Copy, I>(&self, hand: H) -> CardValue where H: IntoIterator<Item=C, IntoIter=I>, I: Iterator<Item=C> {
        self.highest_card(hand).value()
    }

    pub fn compare_potential_hands(&self, a: &PotentialHand, b: &PotentialHand) -> Ordering {
        match a {
            PotentialHand::Hand(hand) => match b {
                PotentialHand::Hand(best_hand) => self.compare_hands(&hand, &best_hand),
                _ => Ordering::Greater,
            },
            PotentialHand::RoyalFlushDraw(draw, typ) => match b {
                PotentialHand::Hand(..) => Ordering::Less,
                PotentialHand::RoyalFlushDraw(best_draw, best_typ) => match typ.cmp(best_typ) {
                    Ordering::Equal => self.value_order(&self.highest_card_value(draw), &self.highest_card_value(best_draw)),
                    other => other,
                },
                _ => Ordering::Greater
            },
            PotentialHand::StraightFlushDraw(draw, typ) => match b {
                PotentialHand::Hand(..) | PotentialHand::RoyalFlushDraw(..) => Ordering::Less,
                PotentialHand::StraightFlushDraw(best_draw, best_typ) => match typ.cmp(best_typ) {
                    Ordering::Equal => self.value_order(&self.highest_card_value(draw), &self.highest_card_value(best_draw)),
                    other => other,
                },
                _ => Ordering::Greater
            },
            PotentialHand::FlushDraw(draw) => match b {
                PotentialHand::Hand(..) | PotentialHand::RoyalFlushDraw(..) | PotentialHand::StraightFlushDraw(..) => Ordering::Less,
                PotentialHand::FlushDraw(best_draw) => match draw.len().cmp(&best_draw.len()) {
                    Ordering::Equal => self.value_order(&self.highest_card_value(draw), &self.highest_card_value(best_draw)),
                    other => other
                },
                _ => Ordering::Greater
            },
            PotentialHand::StraightDraw(draw, typ) => match b {
                PotentialHand::Hand(..) | PotentialHand::RoyalFlushDraw(..) | PotentialHand::StraightFlushDraw(..) | PotentialHand::FlushDraw(..) => Ordering::Less,
                PotentialHand::StraightDraw(best_draw, best_typ) => match draw.len().cmp(&best_draw.len()) {
                    Ordering::Equal => match typ.cmp(best_typ) {
                        Ordering::Equal => self.value_order(&self.highest_card_value(draw), &self.highest_card_value(best_draw)),
                        other => other,
                    },
                    other => other
                },
                _ => Ordering::Greater,
            },
            PotentialHand::HighCard(card) => match b {
                PotentialHand::HighCard(best_card) => self.value_order(&card.value(), &best_card.value()),
                _ => Ordering::Less,
            }
        }
    }

    pub fn compare_hands(&self, a: &Hand, b: &Hand) -> Ordering {
        let resolve_conflict = |a: &HashSet<Card>, b: &HashSet<Card>| {
            let ahc = self.highest_card_value(a.iter());
            let bhc = self.highest_card_value(b.iter());
            self.value_order(&ahc, &bhc)
        };
        match a {
            Hand::RoyalFlush(ref a) => match b {
                Hand::RoyalFlush(ref b) => resolve_conflict(a, b),
                _ => Ordering::Greater
            },
            Hand::StraightFlush(ref a) => match b {
                Hand::RoyalFlush(..) => Ordering::Less,
                Hand::StraightFlush(ref b) => resolve_conflict(a, b),
                _ => Ordering::Greater,
            },
            Hand::FourOfAKind(ref a) => match b {
                Hand::RoyalFlush(..) | Hand::StraightFlush(..) => Ordering::Less,
                Hand::FourOfAKind(ref b) => resolve_conflict(a, b),
                _ => Ordering::Greater,
            },
            Hand::FullHouse(ref a) => match b {
                Hand::RoyalFlush(..) | Hand::StraightFlush(..) | Hand::FourOfAKind(..) => Ordering::Less,
                Hand::FullHouse(ref b) => resolve_conflict(a, b),
                _ => Ordering::Greater
            },
            Hand::Flush(ref a) => match b {
                Hand::RoyalFlush(..) | Hand::StraightFlush(..) | Hand::FourOfAKind(..) | Hand::FullHouse(..) => Ordering::Less,
                Hand::Flush(ref b) => resolve_conflict(a, b),
                _ => Ordering::Greater
            },
            Hand::Straight(ref a) => match b {
                Hand::RoyalFlush(..) | Hand::StraightFlush(..) | Hand::FourOfAKind(..) | Hand::FullHouse(..) | Hand::Flush(..) 
                    => Ordering::Less,
                Hand::Straight(ref b) => resolve_conflict(a, b),
                _ => Ordering::Greater
            },
            Hand::ThreeOfAKind(ref a) =>  match b {
                Hand::RoyalFlush(..) | Hand::StraightFlush(..) | Hand::FourOfAKind(..) | Hand::FullHouse(..) | Hand::Flush(..) |
                Hand::Straight(..) => Ordering::Less,
                Hand::ThreeOfAKind(ref b) => resolve_conflict(a, b),
                _ => Ordering::Greater
            },
            Hand::TwoPair(ref a) => match b {
                Hand::RoyalFlush(..) | Hand::StraightFlush(..) | Hand::FourOfAKind(..) | Hand::FullHouse(..) | Hand::Flush(..) |
                Hand::Straight(..) | Hand::ThreeOfAKind(..) => Ordering::Less,
                Hand::TwoPair(ref b) => resolve_conflict(a, b),
                _ => Ordering::Greater
            },
            Hand::Pair(ref a) => match b {
                Hand::RoyalFlush(..) | Hand::StraightFlush(..) | Hand::FourOfAKind(..) | Hand::FullHouse(..) | Hand::Flush(..) |
                Hand::Straight(..) | Hand::ThreeOfAKind(..) | Hand::TwoPair(..) => Ordering::Less,
                Hand::Pair(ref b) => resolve_conflict(a, b),
            }
        }
    }
}