use crate::skeleton::cards::CardValue;
#[derive(Clone,Copy,Debug)]
pub struct Guess {
    guess: [f32;13],
}

impl Guess {
    fn index(val: CardValue) -> usize {
        match val {
            CardValue::Two => 0,
            CardValue::Three => 1,
            CardValue::Four => 2,
            CardValue::Five => 3,
            CardValue::Six => 4,
            CardValue::Seven => 5,
            CardValue::Eight => 6,
            CardValue::Nine => 7,
            CardValue::Ten => 8,
            CardValue::Jack => 9,
            CardValue::Queen => 10,
            CardValue::King => 11,
            CardValue::Ace => 12
        }
    }

    pub fn update(&mut self, card1: CardValue, card2: CardValue, update_value: f32) {
        let index1 = Guess::index(card1);
        let index2 = Guess::index(card2);
        if self.guess[index1] > 1.0 {
            self.guess[index1] -= update_value
        }
        if self.guess[index2] < 13.0 {
            self.guess[index2] += update_value
        }
    }
    pub fn new () -> Guess {
        Guess {
            guess: [7; 13]
        }
    }
}
