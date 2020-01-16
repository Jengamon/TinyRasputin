use crate::skeleton::cards::CardValue;
#[derive(Clone,Copy,Debug)]
pub struct Guess {
    guess: [f32;13],
}

const SCALE_CONST: f32 = 3.0;

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

    fn clamp(val: f32) -> f32 {
        if val > 13.0 {
            13.0
        } else if val < 1.0 {
            1.0
        } else {
            val
        }
    }

    pub fn update(&mut self, card1: CardValue, card2: CardValue, round: u32, update_value: f32) {
        let index1 = Guess::index(card1);
        let index2 = Guess::index(card2);
        if card1 != card2 && round > 200 {
            self.guess[index1] = Guess::clamp(self.guess[index1] - update_value * ((self.guess[index2] - self.guess[index2]).abs() + round as f32 / 1000.0) / (13.0 * SCALE_CONST));
            self.guess[index2] = Guess::clamp(self.guess[index2] + update_value * ((self.guess[index1] - self.guess[index2]).abs() + round as f32 / 1000.0) / (13.0 * SCALE_CONST));
        } else {
            self.guess[index1] = Guess::clamp(self.guess[index1] - update_value);
            self.guess[index2] = Guess::clamp(self.guess[index2] + update_value);
        }
    }

    pub fn predicted_value(&self, card: CardValue) -> f32 {
        let index = Guess::index(card);
        self.guess[index]
    }

    pub fn new () -> Guess {
        Guess {
            guess: [7.0; 13]
        }
    }
}
