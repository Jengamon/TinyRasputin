#![allow(unused_variables, unused_imports)] // Temporary, while in dev
mod test;
mod lesson1;
mod lesson2;
mod tourney;

pub use test::TestBot;
pub use lesson1::Lesson1Bot;
pub use lesson2::Lesson2Bot;
pub use tourney::TourneyBot;

use super::skeleton::cards::{CardValue};
use rand::prelude::*;

fn generate_ordering() -> [CardValue; 13] {
    let mut original = "2,3,4,5,6,7,8,9,T,J,Q,K,A".split(",").map(|x| x.parse::<CardValue>().unwrap()).collect::<Vec<_>>();
    let mut new = [CardValue::Two; 13];
    let gen_index = || -> usize {
        let mut rng = rand::thread_rng();
        for i in 0..13 {
            if rng.gen_bool(0.25) {
                return i
            }
        }
        return 12
    };
    for index in 0..13 {
        let selected = gen_index() % original.len();
        new[index] = original.remove(selected)
    }
    new
}

fn ensure_relation(ordering: &mut [CardValue; 13], (ref a, ref b): &(CardValue, CardValue)) {
    let mut new_order = ordering.iter().cloned().collect::<Vec<_>>();
    let a_index = ordering.iter().position(|x| *x == *a).unwrap();
    let b_index = ordering.iter().position(|x| *x == *b).unwrap();
    if b_index < a_index { // The ordering is wrong
        let nb = new_order.remove(b_index);
        new_order.insert(a_index, nb);
    }
    assert!(new_order.len() == 13);
    for (i, value) in new_order.iter().enumerate() {
        ordering[i] = *value;
    }
}