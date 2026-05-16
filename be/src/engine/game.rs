use crate::engine::card::{Card, Rank, Suit};
use rand::rng;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::cmp::{max, PartialEq};
use uuid::Uuid;

pub const MAX_PLAYER : usize = 4;
pub const MINIMUM_CLOSE_SCORE : i16 = 38;
#[derive(Debug)]
pub enum GameError {
    #[allow(dead_code)]
    InvalidPlayer,
    InvalidTurn,
    InvalidMove,
    CardNotFound,
}

#[allow(dead_code)]
#[derive(PartialEq)]
pub enum GameStatus {
    InProgress,
    Ended
}
#[allow(dead_code)]
pub struct EndPhaseResponse {
    pub status: Option<GameStatus>,
    pub next_turn: u8,
    pub winner: Option<Player>
}
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum GamePhase {
    GameEnded,
    P1,
    P2,
}



#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Game {
    pub id: Uuid,
    pub players: Vec<Player>,
    pub deck: Vec<Card>,
    pub current_turn: usize,
    pub phase: GamePhase,
}

impl Game {
    pub fn new( players_uuid: Vec<Uuid>) -> Game {

        let mut deck = Self::create_deck();
        let players: Vec<Player> = players_uuid.iter().map(|&uuid| {
            let mut hand = vec![];
            for _ in 0..4 {
                if let Some(card) = deck.pop() {
                    hand.push(card);
                } else {
                    panic!("Invalid configuration!");
                }
            }
            Player {
                id: uuid,
                hand,
                bin: vec![],
            }
        }).collect();

        Game {
            id: Uuid::new_v4(),
            players,
            deck,
            current_turn: 0,
            phase: GamePhase::P1,
        }
    }

    pub fn close(&mut self, player_uuid: &Uuid, card: Card) -> Result<EndPhaseResponse, GameError> {
        if self.players[self.current_turn].id != *player_uuid || self.phase != GamePhase::P2 {
            return Err(GameError::InvalidMove);
        }

        if let Err(GameError::CardNotFound) = self.remove_card(&card) {
            return Err(GameError::CardNotFound);
        }

        if self.players[self.current_turn].score() < MINIMUM_CLOSE_SCORE {
            self.players[self.current_turn].hand.push(card);
            return Err(GameError::InvalidMove);
        }

        self.current_turn = (self.current_turn + 1) % self.players.len();

        self.phase = GamePhase::GameEnded;
        Ok(EndPhaseResponse {
            next_turn: self.current_turn as u8,
            status: Some(GameStatus::Ended),
            winner: self.winner()       ,
        })
    }

    pub fn discard(&mut self, player_uuid: &Uuid, card: Card) -> Result<EndPhaseResponse, GameError> {
        if self.players[self.current_turn].id != *player_uuid || self.phase != GamePhase::P2 {
            return Err(GameError::InvalidMove);
        }

        if let Err(GameError::CardNotFound) = self.remove_card(&card) {
            return Err(GameError::CardNotFound);
        }

        self.current_turn = (self.current_turn + 1) % self.players.len();

        if self.deck.len() > 0 {
            self.phase = GamePhase::P1;
            self.players[self.current_turn].bin.push(card);
            Ok(EndPhaseResponse {
                next_turn: self.current_turn as u8,
                status: Some(GameStatus::InProgress),
                winner: None,
            })
        } else {
            self.phase = GamePhase::GameEnded;
            Ok(EndPhaseResponse {
                next_turn: self.current_turn as u8,
                status: Some(GameStatus::Ended),
                winner: self.winner(),
            })
        }
    }
    pub fn take_bin(&mut self, player_uuid: &Uuid) -> Result<(), GameError> {
        // println!("Taking bin: {:?}", self.deck);
        if self.players[self.current_turn].id != *player_uuid || self.phase != GamePhase::P1 {
            return Err(GameError::InvalidMove);
        }

        let card = match self.players[self.current_turn].bin.pop() {
            Some(card) => card,
            None => return Err(GameError::InvalidMove),
        };

        self.players[self.current_turn].hand.push(card);
        self.phase = GamePhase::P2;
        Ok(())
    }

    pub fn draw(&mut self, player_uuid: &Uuid) -> Result<(), GameError>  {
        if self.players[self.current_turn].id != *player_uuid || self.phase != GamePhase::P1  {
            return Err(GameError::InvalidMove);
        }

        let card = match self.deck.pop() {
            Some(card) => card,
            None => return Err(GameError::InvalidMove),
        };

        if let Some(current_player) = self.players.get_mut(self.current_turn) {
            current_player.hand.push(card);
            self.phase = GamePhase::P2;
            Ok(())
        } else {
            self.deck.push(card);
            Err(GameError::InvalidTurn)
        }
    }

    pub fn remove_player(&mut self, player_uuid: &Uuid) -> Result<(), GameError> {
        if let Some(index) = self.players.iter().position(|c| c.id == *player_uuid) {
            if self.current_turn == index {
                self.phase = GamePhase::P2;
                self.discard(player_uuid, self.players[index].hand[0].clone())?;
            }
            self.players.remove(index);
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub fn scores(&self) -> Vec<i16> {
        self.players.iter().map(|player: &Player| {player.score()}).collect()
    }

    #[allow(dead_code)]
    pub fn score(&self, player_uuid: &Uuid) -> Result<i16, GameError> {
        let index = match self.players.iter().position(|c| c.id == *player_uuid) {
            Some(i) => i,
            None => return Err(GameError::InvalidPlayer)
        };

        Ok(self.players[index].score())
    }

    pub fn winner(&self) -> Option<Player> {
        if self.phase != GamePhase::GameEnded {
            return None;
        }
        let mut winner = None;
        let mut max_score = 0;
        for player in &self.players {
            let score = player.score();
            if max_score < score {
                winner = Some(player.clone());
                max_score = score;
            } else if score == max_score {
                winner = None
            }
        }

        winner
    }

    #[allow(dead_code)]
    pub fn current_player(&self) -> Player {
        self.players[self.current_turn].clone()
    }

    pub fn player_pos(&self, player_uuid: &Uuid) -> Option<usize> {
        self.players.iter().position(|c| c.id == *player_uuid)
    }

    pub fn card_left(&self) -> u8 {
        self.deck.len() as u8
    }

    fn remove_card(&mut self, card: &Card) -> Result<(), GameError> {
        let index = match self.players[self.current_turn].hand.iter().position(|c| c == card) {
            Some(i) => i,
            None => return Err(GameError::CardNotFound)
        };

        self.players[self.current_turn].hand.remove(index);
        Ok(())
    }

    fn create_deck() -> Vec<Card> {
        let mut cards = Vec::with_capacity(52);

        for suit in [Suit::Hearts, Suit::Diamonds, Suit::Clubs, Suit::Spades].iter() {
            for rank in [
                Rank::Ace, Rank::Two, Rank::Three, Rank::Four, Rank::Five,
                Rank::Six, Rank::Seven, Rank::Eight, Rank::Nine, Rank::Ten,
                Rank::Jack, Rank::Queen, Rank::King
            ].iter() {
                cards.push(Card {
                    suit: suit.clone(),
                    rank: rank.clone(),
                })
            }
        }
        cards.shuffle(&mut rng());
        cards
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Player {
    pub id: Uuid,
    pub hand: Vec<Card>,
    pub bin: Vec<Card>,
}

impl Player {
    pub fn score(&self) -> i16 {

        let mut points:[u16;4] = [0, 0, 0, 0];
        let mut max_point:u16 = 0;
        for i in 0..4 {
            let card = self.hand.get(i).unwrap();
            let point = card.points();
            let ip = match self.hand[i].suit {
                Suit::Hearts => {0},
                Suit::Diamonds => {1},
                Suit::Clubs => {2},
                Suit::Spades => {3},
            };
            points[ip] += point;
            max_point = max(max_point, points[ip]);
        }
        ((max_point as i16) *2) - points.iter().sum::<u16>() as i16
    }
}


