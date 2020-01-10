const MAX_CARDS: usize = 7;
const MIN_CARDS: usize = 2;

use super::super::skeleton::{
    cards::{CardValue, Card}
};
use crate::into_cards;

use std::cmp::Ordering;
use std::fmt;

/// Valid hands
#[derive(Debug, Clone)]
pub enum ShowdownHand {
    RoyalFlush(Vec<Card>),
    StraightFlush(Vec<Card>),
    FourOfAKind(Vec<Card>),
    FullHouse(Vec<Card>),
    Flush(Vec<Card>),
    Straight(Vec<Card>),
    ThreeOfAKind(Vec<Card>),
    TwoPair(Vec<Card>),
    Pair(Vec<Card>),
    HighCard(Card)
}

impl ShowdownHand {
    pub fn cards(&self) -> Vec<Card> {
        match self {
            ShowdownHand::RoyalFlush(a) => a.clone(),
            ShowdownHand::StraightFlush(a) => a.clone(),
            ShowdownHand::FourOfAKind(a) => a.clone(),
            ShowdownHand::FullHouse(a) => a.clone(),
            ShowdownHand::Flush(a) => a.clone(),
            ShowdownHand::Straight(a) => a.clone(),
            ShowdownHand::ThreeOfAKind(a) => a.clone(),
            ShowdownHand::TwoPair(a) => a.clone(),
            ShowdownHand::Pair(a) => a.clone(),
            ShowdownHand::HighCard(c) => vec![*c]
        }
    }

    pub fn is_same_type(&self, o: &ShowdownHand) -> bool {
        match self {
            ShowdownHand::RoyalFlush(..) => match o {
                ShowdownHand::RoyalFlush(..) => true,
                _ => false
            },
            ShowdownHand::StraightFlush(..) => match o {
                ShowdownHand::StraightFlush(..) => true,
                _ => false
            },
            ShowdownHand::FourOfAKind(..) => match o {
                ShowdownHand::FourOfAKind(..) => true,
                _ => false
            },
            ShowdownHand::FullHouse(..) => match o {
                ShowdownHand::FullHouse(..) => true,
                _ => false
            },
            ShowdownHand::Flush(..) => match o {
                ShowdownHand::Flush(..) => true,
                _ => false
            },
            ShowdownHand::Straight(..) => match o {
                ShowdownHand::Straight(..) => true,
                _ => false
            },
            ShowdownHand::ThreeOfAKind(..) => match o {
                ShowdownHand::ThreeOfAKind(..) => true,
                _ => false
            },
            ShowdownHand::TwoPair(..) => match o {
                ShowdownHand::TwoPair(..) => true,
                _ => false
            },
            ShowdownHand::Pair(..) => match o {
                ShowdownHand::Pair(..) => true,
                _ => false
            },
            ShowdownHand::HighCard(..) => match o {
                ShowdownHand::HighCard(..) => true,
                _ => false
            }
        }
    }
}

impl fmt::Display for ShowdownHand {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ShowdownHand::RoyalFlush(a) => write!(fmt, "[Royal Flush {} {} {} {} {}]", a[0], a[1], a[2], a[3], a[4]),
            ShowdownHand::StraightFlush(a) => write!(fmt, "[Straight Flush {} {} {} {} {}]", a[0], a[1], a[2], a[3], a[4]),
            ShowdownHand::FourOfAKind(a) => write!(fmt, "[Four of a Kind {} {} {} {}]", a[0], a[1], a[2], a[3]),
            ShowdownHand::FullHouse(a) => write!(fmt, "[Full House {} {} {} {} {}]", a[0], a[1], a[2], a[3], a[4]),
            ShowdownHand::Flush(a) => write!(fmt, "[Flush {} {} {} {} {}]", a[0], a[1], a[2], a[3], a[4]),
            ShowdownHand::Straight(a) => write!(fmt, "[Straight {} {} {} {} {}]", a[0], a[1], a[2], a[3], a[4]),
            ShowdownHand::ThreeOfAKind(a) => write!(fmt, "[Three of a Kind {} {} {}]", a[0], a[1], a[2]),
            ShowdownHand::TwoPair(a) => write!(fmt, "[Two Pair {} {} {} {}]", a[0], a[1], a[2], a[3]),
            ShowdownHand::Pair(a) => write!(fmt, "[Pair {} {}]", a[0], a[1]),
            ShowdownHand::HighCard(a) => write!(fmt, "[High Card {}]", a)
        }
    }
}

/// Detects the best hand out of the given cards
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

impl ShowdownEngine {
    pub fn new(ordering: [CardValue; 13]) -> ShowdownEngine {
        ShowdownEngine {
            ordering
        }
    }

    fn verify_hand(hand: &[Card]) -> bool {
        hand.len() >= MIN_CARDS && hand.len() <= MAX_CARDS
    }

    pub fn process_hand(&self, hand: &[Card]) -> ShowdownHand {
        assert!(ShowdownEngine::verify_hand(hand));

        //println!("Hand: {:?}", hand);
       
        let pf = self.detect_flushes(hand);
        let ps = self.detect_straights(hand);
        //println!("PF: {:?}", pf);
        //println!("PS: {:?}", ps);
        let mut fs: Vec<Vec<Card>> = vec![];
        for flush in &pf {
            for straight in &ps {
                let shared: Vec<_> = flush.iter().filter(|x| straight.contains(x)).cloned().collect();
                if shared.len() >= 5 {
                    fs.push(shared);
                }
            }
        }
        if !fs.is_empty() {
            let highest = fs.into_iter().max_by(|a, b| {
                let ahc = self.highest_card(&a[..]);
                let bhc = self.highest_card(&b[..]);
                let oa = self.ordering.iter().position(|x| *x == ahc.value()).unwrap();
                let ob = self.ordering.iter().position(|x| *x == bhc.value()).unwrap();
                oa.cmp(&ob)
            }).unwrap();
            if highest.iter().any(|x| x.value() == self.ordering[8]) && highest.iter().any(|x| x.value() == self.ordering[12]) {
                return ShowdownHand::RoyalFlush(self.best_hand(&highest))
            } else {
                return ShowdownHand::StraightFlush(self.best_hand(&highest))
            }
        } else {
            if !pf.is_empty() {
                let highest = pf.iter().cloned().max_by(|a, b| {
                    let ahc = self.highest_card(&a);
                    let bhc = self.highest_card(&b);
                    let oa = self.ordering.iter().position(|x| *x == ahc.value()).unwrap();
                    let ob = self.ordering.iter().position(|x| *x == bhc.value()).unwrap();
                    oa.cmp(&ob)
                }).unwrap().clone();
                return ShowdownHand::Flush(self.best_hand(&highest))
            } 
            if !ps.is_empty() {
                let highest = ps.iter().cloned().max_by(|a, b| {
                    let ahc = self.highest_card(&a);
                    let bhc = self.highest_card(&b);
                    let oa = self.ordering.iter().position(|x| *x == ahc.value()).unwrap();
                    let ob = self.ordering.iter().position(|x| *x == bhc.value()).unwrap();
                    oa.cmp(&ob)
                }).unwrap().clone();
                return ShowdownHand::Straight(self.best_hand(&highest))
            }
        }
        if let Some(foak) = self.detect_of_a_kind(hand, 4) {
            return ShowdownHand::FourOfAKind(foak)
        }
        if let Some(toak) = self.detect_of_a_kind(hand, 3) {
            let mut dhand = hand.to_vec();
            for card in &toak {
                let index = dhand.iter().position(|x| *x == *card).unwrap();
                dhand.remove(index);
            }
            if let Some(pair) = self.detect_of_a_kind(&dhand, 2) {
                let mut total = toak;
                total.extend(pair.iter());
                return ShowdownHand::FullHouse(total)
            } else {
                return ShowdownHand::ThreeOfAKind(toak)
            }
        }
        if let Some(pair) = self.detect_of_a_kind(hand, 2) {
            let mut dhand = hand.to_vec();
            for card in &pair {
                let index = dhand.iter().position(|x| *x == *card).unwrap();
                dhand.remove(index);
            }
            if let Some(pair2) = self.detect_of_a_kind(&dhand, 2) {
                let mut total = pair;
                total.extend(pair2.iter());
                return ShowdownHand::TwoPair(total)
            } else {
                return ShowdownHand::Pair(pair)
            }
        }
        ShowdownHand::HighCard(self.highest_card(hand))
    }

    /// From the given hand, select at most the five highest cards
    fn best_hand(&self, hand: &[Card]) -> Vec<Card> {
        let mut ohand = hand.to_vec();
        let mut hand = vec![];
        while hand.len() < 5 && ohand.len() > 0 {
            let high_card = self.highest_card(&ohand);
            let index = ohand.iter().position(|x| *x == high_card).unwrap();
            ohand.remove(index);
            hand.push(high_card);
        }
        hand
    }

    fn detect_straights(&self, hand: &[Card]) -> Vec<Vec<Card>> {
        let mut possible_straights: Vec<Vec<Card>> = vec![];
        for card in hand {
            let mut flag = false;
            for ps in &mut possible_straights {
                let bottom = self.ordering.iter().position(|x| *x == ps.iter().nth(0).unwrap().value()).unwrap();
                let top = self.ordering.iter().position(|x| *x == ps.iter().last().unwrap().value()).unwrap();
                let cardv = self.ordering.iter().position(|x| *x == card.value()).unwrap();
                if cardv + 1 == bottom {
                    ps.insert(0, *card);
                    flag = true;
                } else if cardv == top + 1 {
                    ps.push(*card);
                    flag = true;
                } else {
                    flag = flag || false;
                }
            }
            if !flag {
                possible_straights.push(vec![*card]);
            }
        }
        possible_straights.into_iter().filter(|x| x.len() >= 5).collect::<Vec<Vec<_>>>()
    }

    fn detect_flushes(&self, hand: &[Card]) -> Vec<Vec<Card>> {
        let mut possible_flushes = vec![];
        let mut tried_suits = vec![];
        for card in hand {
            let suit = card.suit();
            if tried_suits.contains(&suit) {
                continue
            }
            tried_suits.push(suit);

            let cards = hand.iter().fold(vec![], |mut cards, x| {
                if x.suit() == suit {
                    cards.push(*x);
                    
                }
                cards
            });

            if cards.len() >= 5 {
                possible_flushes.push(cards);
            }
        }
        possible_flushes
    }

    /// Detect the highest of-a-kind with <number> cards 
    fn detect_of_a_kind(&self, hand: &[Card], number: usize) -> Option<Vec<Card>> {
        let mut oak = vec![];
        for card in hand.iter() {
            let (count, cards) = hand.iter().fold((0, vec![]), |(count, mut cards), x| {
                if x.value() == card.value() {
                    cards.push(*x);
                    (count + 1, cards)
                } else {
                    (count, cards)
                }
            });
            if count == number {
                oak.push(cards)
            }
        }
        oak.iter().max_by(|a, b| {
            let ac = self.highest_card(&a);
            let bc = self.highest_card(&b);
            let ac = self.ordering.iter().position(|x| *x == ac.value()).unwrap();
            let bc = self.ordering.iter().position(|x| *x == bc.value()).unwrap();
            ac.cmp(&bc)
        }).cloned()
    }

    pub fn highest_card(&self, hand: &[Card]) -> Card {
        *hand.iter()
            .map(|x| (x, self.ordering.iter().position(|y| *y == x.value())))
            .max_by(|x, y| x.1.cmp(&y.1))
            .expect("Expected non-empty hand").0
    }

    pub fn compare_hands(&self, a: &ShowdownHand, b: &ShowdownHand) -> Ordering {
        let resolve_conflict = |a, b| {
            let ahc = self.highest_card(a);
            let bhc = self.highest_card(b);
            let oa = self.ordering.iter().position(|x| *x == ahc.value()).unwrap();
            let ob = self.ordering.iter().position(|x| *x == bhc.value()).unwrap();
            oa.cmp(&ob)
        };
        match a {
            ShowdownHand::RoyalFlush(ref a) => match b {
                ShowdownHand::RoyalFlush(ref b) => resolve_conflict(a, b),
                _ => Ordering::Greater
            },
            ShowdownHand::StraightFlush(ref a) => match b {
                ShowdownHand::RoyalFlush(..) => Ordering::Less,
                ShowdownHand::StraightFlush(ref b) => resolve_conflict(a, b),
                _ => Ordering::Greater,
            },
            ShowdownHand::FourOfAKind(ref a) => match b {
                ShowdownHand::RoyalFlush(..) | ShowdownHand::StraightFlush(..) => Ordering::Less,
                ShowdownHand::FourOfAKind(ref b) => resolve_conflict(a, b),
                _ => Ordering::Greater,
            },
            ShowdownHand::FullHouse(ref a) => match b {
                ShowdownHand::RoyalFlush(..) | ShowdownHand::StraightFlush(..) | ShowdownHand::FourOfAKind(..) => Ordering::Less,
                ShowdownHand::FullHouse(ref b) => resolve_conflict(a, b),
                _ => Ordering::Greater
            },
            ShowdownHand::Flush(ref a) => match b {
                ShowdownHand::RoyalFlush(..) | ShowdownHand::StraightFlush(..) | ShowdownHand::FourOfAKind(..) | ShowdownHand::FullHouse(..) => Ordering::Less,
                ShowdownHand::Flush(ref b) => resolve_conflict(a, b),
                _ => Ordering::Greater
            },
            ShowdownHand::Straight(ref a) => match b {
                ShowdownHand::RoyalFlush(..) | ShowdownHand::StraightFlush(..) | ShowdownHand::FourOfAKind(..) | ShowdownHand::FullHouse(..) | ShowdownHand::Flush(..) 
                    => Ordering::Less,
                ShowdownHand::Straight(ref b) => resolve_conflict(a, b),
                _ => Ordering::Greater
            },
            ShowdownHand::ThreeOfAKind(ref a) =>  match b {
                ShowdownHand::RoyalFlush(..) | ShowdownHand::StraightFlush(..) | ShowdownHand::FourOfAKind(..) | ShowdownHand::FullHouse(..) | ShowdownHand::Flush(..) |
                ShowdownHand::Straight(..) => Ordering::Less,
                ShowdownHand::ThreeOfAKind(ref b) => resolve_conflict(a, b),
                _ => Ordering::Greater
            },
            ShowdownHand::TwoPair(ref a) => match b {
                ShowdownHand::RoyalFlush(..) | ShowdownHand::StraightFlush(..) | ShowdownHand::FourOfAKind(..) | ShowdownHand::FullHouse(..) | ShowdownHand::Flush(..) |
                ShowdownHand::Straight(..) | ShowdownHand::ThreeOfAKind(..) => Ordering::Less,
                ShowdownHand::TwoPair(ref b) => resolve_conflict(a, b),
                _ => Ordering::Greater
            },
            ShowdownHand::Pair(ref a) => match b {
                ShowdownHand::RoyalFlush(..) | ShowdownHand::StraightFlush(..) | ShowdownHand::FourOfAKind(..) | ShowdownHand::FullHouse(..) | ShowdownHand::Flush(..) |
                ShowdownHand::Straight(..) | ShowdownHand::ThreeOfAKind(..) | ShowdownHand::TwoPair(..) => Ordering::Less,
                ShowdownHand::Pair(ref b) => resolve_conflict(a, b),
                _ => Ordering::Greater
            },
            ShowdownHand::HighCard(ref a) => match b {
                ShowdownHand::HighCard(ref b) => {
                    let oa = self.ordering.iter().position(|x| *x == a.value()).unwrap();
                    let ob = self.ordering.iter().position(|x| *x == b.value()).unwrap();
                    oa.cmp(&ob)
                },
                _ => Ordering::Less
            }
        }
    }
}

#[test]
fn showdown_test() {
    let mut ordering = [CardValue::Two; 13];
    let order: Vec<_> = "2,3,4,5,6,7,8,9,T,J,Q,K,A".split(",").map(|x| x.parse::<CardValue>().unwrap()).collect();
    for i in 0..13 {
        ordering[i] = order[i];
    }
    let showdown = ShowdownEngine {
        ordering
    };

    let tests = [
        ("2c,2h,Ac,Ah", ShowdownHand::TwoPair(into_cards!("2c,2h,Ac,Ah"))), 
        ("2c,2h,Ac,Ah,As", ShowdownHand::FullHouse(into_cards!("2c,2h,Ac,Ah,As"))),
        ("2c,Ac,2h,Ah,As", ShowdownHand::FullHouse(into_cards!("As,Ah,Ac,2c,2h"))),
        ("2c,3c,4c,5c,7c", ShowdownHand::Flush(into_cards!("2c,3c,4c,5c,7c"))),
    ];

    for (hand, best) in tests.into_iter() {
        let hand = into_cards!(hand);
        let detected = showdown.process_hand(&hand);
        assert_eq!(showdown.compare_hands(&detected, &best), Ordering::Equal);
    }
}