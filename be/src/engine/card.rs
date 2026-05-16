use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Suit {
    Hearts,
    Diamonds,
    Clubs,
    Spades,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Rank {
    Ace,
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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Card {
    pub suit: Suit,
    pub rank: Rank,
}

impl Card {
    pub fn points(&self) -> u16 {
        match self.rank {
            Rank::Ace => {11}
            Rank::Two => {2}
            Rank::Three => {3}
            Rank::Four => {4}
            Rank::Five => {5}
            Rank::Six => {6}
            Rank::Seven => {7}
            Rank::Eight => {8}
            Rank::Nine => {9}
            Rank::Ten => {10}
            Rank::Jack => {10}
            Rank::Queen => {10}
            Rank::King => {10}
        }
    }

    pub fn to_string(&self) -> String {
        let suit = match self.suit {
            Suit::Hearts => "H",
            Suit::Diamonds => "D",
            Suit::Clubs => "C",
            Suit::Spades => "S",
        };

        let rank = match self.rank {
            Rank::Ace => "A",
            Rank::Two => "2",
            Rank::Three => "3",
            Rank::Four => "4",
            Rank::Five => "5",
            Rank::Six => "6",
            Rank::Seven => "7",
            Rank::Eight => "8",
            Rank::Nine => "9",
            Rank::Ten => "X",
            Rank::Jack => "J",
            Rank::Queen => "Q",
            Rank::King => "K",
        };

        format!("{}{}", suit, rank)
    }

    pub fn from_string(input: &str) -> Option<Self> {
        if input.len() != 2 {
            return None;
        }

        let (suit_char, rank_str) = input.split_at(1);
        let suit = match suit_char {
            "H" => Suit::Hearts,
            "D" => Suit::Diamonds,
            "C" => Suit::Clubs,
            "S" => Suit::Spades,
            _ => return None,
        };

        let rank = match rank_str {
            "A" => Rank::Ace,
            "2" => Rank::Two,
            "3" => Rank::Three,
            "4" => Rank::Four,
            "5" => Rank::Five,
            "6" => Rank::Six,
            "7" => Rank::Seven,
            "8" => Rank::Eight,
            "9" => Rank::Nine,
            "X" => Rank::Ten,
            "J" => Rank::Jack,
            "Q" => Rank::Queen,
            "K" => Rank::King,
            _ => return None,
        };

        Some(Card { suit, rank })
    }
}