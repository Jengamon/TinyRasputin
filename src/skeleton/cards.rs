//! Conversion utilities for cards to and from standard format strings

// TODO Maybe implement serde?

use std::fmt;
use std::str::FromStr;
use std::error::Error;
use itertools::Itertools;

/// Encodes card suit
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, PartialOrd, Ord)]
pub enum CardSuit {
    Spades,
    Hearts,
    Diamonds,
    Clubs,
}

impl FromStr for CardSuit {
    type Err = CardConversionError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
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

/// Encodes card value
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, PartialOrd, Ord)]
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
        let s = s.trim();
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

/// Encodes a valid poker card
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, PartialOrd, Ord)]
pub struct Card {
    value: CardValue,
    suit: CardSuit,
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
        let s = s.trim();
        if s.len() > 2 {
            return Err(CardConversionError::TooLong(s.to_string()))
        } else if s.len() < 2 {
            return Err(CardConversionError::NotACard(s.to_string()))
        }
        let mut chars = s.chars();
        let value = chars.next().map(|x| x.to_string()).unwrap().parse::<CardValue>()?;
        let suit = chars.next().map(|x| x.to_string()).unwrap().parse::<CardSuit>()?;
        Ok(Card { suit, value })
    }
}

impl fmt::Display for Card {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}{}", self.value, self.suit)
    }
}

/// Wraps a deck and makes it printable
#[derive(Debug, Clone)]
pub struct CardDeck(pub Vec<Card>);

impl fmt::Display for CardDeck {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        if self.0.is_empty() {
            write!(fmt, "<empty>")
        } else {
            write!(fmt, "[{}]", self.0.iter().format(", "))
        }
    }
}

/// Wraps a hand and makes it printable
#[derive(Debug, Clone, Copy)]
pub struct CardHand(pub [Card; 2]);

impl fmt::Display for CardHand {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "[{}, {}]", self.0[0], self.0[1])
    }
}

/// Allows for Option<CardHand> to be easily printed
pub trait CardHandExt {
    fn print(&self) -> String;
}

impl CardHandExt for Option<CardHand> {
    fn print(&self) -> String {
        self.map(|x| format!("{}", x)).unwrap_or("<empty>".into())
    }
}

/// Converts a string into a Vec<Card>
#[macro_export]
macro_rules! into_cards {
    ($t:literal) => ($t.split(",").map(|x| x.parse::<Card>().unwrap()).collect::<Vec<_>>());
    ($t:expr) =>  ($t.split(",").map(|x| x.parse::<Card>().unwrap()).collect::<Vec<_>>());
}

/// Describes various errors that can occur in conversion to Card{Suit, Hand} from strings
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


