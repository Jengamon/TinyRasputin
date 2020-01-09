//! Conversion utilities for cards to and from standard format strings

// TODO Maybe implement serde?

use std::fmt;
use std::str::FromStr;
use std::error::Error;

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub enum CardSuit {
    Hearts,
    Diamonds,
    Spades,
    Clubs,
}

impl FromStr for CardSuit {
    type Err = CardConversionError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() > 1 {
            return Err(CardConversionError::TooLong(s.to_string()))
        }
        let chr = s.chars().nth(0);
        if let Some(chr) = chr {
            match chr {
                'h' => Ok(CardSuit::Hearts),
                'd' => Ok(CardSuit::Diamonds),
                's' => Ok(CardSuit::Spades),
                'c' => Ok(CardSuit::Clubs),
                c => Err(CardConversionError::InvalidSuit(c))
            }
        } else {
            Err(CardConversionError::Empty)
        }
    }
}

impl fmt::Display for CardSuit {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CardSuit::Hearts => write!(fmt, "h"),
            CardSuit::Diamonds => write!(fmt, "d"),
            CardSuit::Spades => write!(fmt, "s"),
            CardSuit::Clubs => write!(fmt, "c"),
        }
    }
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub enum CardValue {
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
    Ace
}

impl FromStr for CardValue {
    type Err = CardConversionError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() > 1 {
            return Err(CardConversionError::TooLong(s.to_string()))
        }
        let chr = s.chars().nth(0);
        if let Some(chr) = chr {
            match chr {
                '2' => Ok(CardValue::Two),
                '3' => Ok(CardValue::Three),
                '4' => Ok(CardValue::Four),
                '5' => Ok(CardValue::Five),
                '6' => Ok(CardValue::Six),
                '7' => Ok(CardValue::Seven),
                '8' => Ok(CardValue::Eight),
                '9' => Ok(CardValue::Nine),
                'T' => Ok(CardValue::Ten),
                'J' => Ok(CardValue::Jack),
                'Q' => Ok(CardValue::Queen),
                'K' => Ok(CardValue::King),
                'A' => Ok(CardValue::Ace),
                c => Err(CardConversionError::InvalidValue(c))
            }
        } else {
            Err(CardConversionError::Empty)
        }
    }
}

impl fmt::Display for CardValue {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CardValue::Two => write!(fmt, "2"),
            CardValue::Three => write!(fmt, "3"),
            CardValue::Four => write!(fmt, "4"),
            CardValue::Five => write!(fmt, "5"),
            CardValue::Six => write!(fmt, "6"),
            CardValue::Seven => write!(fmt, "7"),
            CardValue::Eight => write!(fmt, "8"),
            CardValue::Nine => write!(fmt, "9"),
            CardValue::Ten => write!(fmt, "T"),
            CardValue::Jack => write!(fmt, "J"),
            CardValue::Queen => write!(fmt, "Q"),
            CardValue::King => write!(fmt, "K"),
            CardValue::Ace => write!(fmt, "A"),
        }
    }
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct Card {
    suit: CardSuit,
    value: CardValue,
}

impl Card {
    pub fn new(suit: CardSuit, value: CardValue) -> Card {
        Card { suit, value }
    }

    pub fn suit(&self) -> CardSuit {
        self.suit
    }

    pub fn value(&self) -> CardValue {
        self.value
    }
}

impl FromStr for Card {
    type Err = CardConversionError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() > 2 {
            return Err(CardConversionError::TooLong(s.to_string()))
        } else if s.len() < 2 {
            return Err(CardConversionError::NotACard(s.to_string()))
        }
        let value = s[..=0].parse::<CardValue>()?;
        let suit = s[1..].parse::<CardSuit>()?;
        Ok(Card { suit, value })
    }
}

impl fmt::Display for Card {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}{}", self.value, self.suit)
    }
}


#[derive(Debug)]
pub enum CardConversionError {
    InvalidSuit(char),
    InvalidValue(char),
    Empty,
    TooLong(String),
    NotACard(String),
}

impl Error for CardConversionError {}

impl fmt::Display for CardConversionError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CardConversionError::InvalidSuit(s) => write!(fmt, "Invalid suit: {}", s),
            CardConversionError::InvalidValue(v) => write!(fmt, "Invalid value: {}", v),
            CardConversionError::Empty => write!(fmt, "Unexpected empty string"),
            CardConversionError::TooLong(s) => write!(fmt, "String too long: {}", s),
            CardConversionError::NotACard(s) => write!(fmt, "String too short for card: {}", s),
        }
    }
}


