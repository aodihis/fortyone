use crate::engine::card::{Card, Suit};
use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use std::cmp::max;
use uuid::Uuid;

pub const MAX_PLAYER: usize = 4;
pub const MINIMUM_CLOSE_SCORE: i16 = 38;

#[derive(Debug)]
pub enum EngineError {
    InvalidPlayer,
    InvalidTurn,
    InvalidMove,
    CardNotFound,
    InsufficientDeck,
}

#[allow(dead_code)]
#[derive(PartialEq)]
pub enum GameStatus {
    InProgress,
    Ended,
}

#[allow(dead_code)]
pub struct EndPhaseResponse {
    pub status: Option<GameStatus>,
    pub next_turn: u8,
    pub winner: Option<Player>,
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
    pub fn new(players_uuid: Vec<Uuid>) -> Result<Game, EngineError> {
        let mut deck = Self::create_deck();
        let mut players: Vec<Player> = Vec::with_capacity(players_uuid.len());
        for &uuid in &players_uuid {
            let mut hand = vec![];
            for _ in 0..4 {
                match deck.pop() {
                    Some(card) => hand.push(card),
                    None => return Err(EngineError::InsufficientDeck),
                }
            }
            players.push(Player {
                id: uuid,
                hand,
                bin: vec![],
            });
        }
        Ok(Game {
            id: Uuid::new_v4(),
            players,
            deck,
            current_turn: 0,
            phase: GamePhase::P1,
        })
    }

    pub fn close(
        &mut self,
        player_uuid: &Uuid,
        card: Card,
    ) -> Result<EndPhaseResponse, EngineError> {
        if self.players[self.current_turn].id != *player_uuid || self.phase != GamePhase::P2 {
            return Err(EngineError::InvalidMove);
        }

        if let Err(EngineError::CardNotFound) = self.remove_card(&card) {
            return Err(EngineError::CardNotFound);
        }

        if self.players[self.current_turn].score() < MINIMUM_CLOSE_SCORE {
            self.players[self.current_turn].hand.push(card);
            return Err(EngineError::InvalidMove);
        }

        self.current_turn = (self.current_turn + 1) % self.players.len();
        self.phase = GamePhase::GameEnded;
        Ok(EndPhaseResponse {
            next_turn: self.current_turn as u8,
            status: Some(GameStatus::Ended),
            winner: self.winner(),
        })
    }

    pub fn discard(
        &mut self,
        player_uuid: &Uuid,
        card: Card,
    ) -> Result<EndPhaseResponse, EngineError> {
        if self.players[self.current_turn].id != *player_uuid || self.phase != GamePhase::P2 {
            return Err(EngineError::InvalidMove);
        }

        if let Err(EngineError::CardNotFound) = self.remove_card(&card) {
            return Err(EngineError::CardNotFound);
        }

        self.current_turn = (self.current_turn + 1) % self.players.len();

        if !self.deck.is_empty() {
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

    pub fn take_bin(&mut self, player_uuid: &Uuid) -> Result<(), EngineError> {
        if self.players[self.current_turn].id != *player_uuid || self.phase != GamePhase::P1 {
            return Err(EngineError::InvalidMove);
        }

        let card = match self.players[self.current_turn].bin.pop() {
            Some(card) => card,
            None => return Err(EngineError::InvalidMove),
        };

        self.players[self.current_turn].hand.push(card);
        self.phase = GamePhase::P2;
        Ok(())
    }

    pub fn draw(&mut self, player_uuid: &Uuid) -> Result<(), EngineError> {
        if self.players[self.current_turn].id != *player_uuid || self.phase != GamePhase::P1 {
            return Err(EngineError::InvalidMove);
        }

        let card = match self.deck.pop() {
            Some(card) => card,
            None => return Err(EngineError::InvalidMove),
        };

        if let Some(current_player) = self.players.get_mut(self.current_turn) {
            current_player.hand.push(card);
            self.phase = GamePhase::P2;
            Ok(())
        } else {
            self.deck.push(card);
            Err(EngineError::InvalidTurn)
        }
    }

    pub fn remove_player(&mut self, player_uuid: &Uuid) -> Result<(), EngineError> {
        if let Some(index) = self.players.iter().position(|c| c.id == *player_uuid) {
            if self.current_turn == index && self.phase == GamePhase::P2 {
                if let Some(card) = self.players[index].hand.first().cloned() {
                    let _ = self.discard(player_uuid, card);
                }
            }
            self.players.remove(index);
            if self.current_turn >= self.players.len() && !self.players.is_empty() {
                self.current_turn = 0;
            }
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn scores(&self) -> Vec<i16> {
        self.players.iter().map(|p| p.score()).collect()
    }

    #[allow(dead_code)]
    pub fn score(&self, player_uuid: &Uuid) -> Result<i16, EngineError> {
        self.players
            .iter()
            .find(|p| p.id == *player_uuid)
            .map(|p| p.score())
            .ok_or(EngineError::InvalidPlayer)
    }

    pub fn winner(&self) -> Option<Player> {
        if self.phase != GamePhase::GameEnded {
            return None;
        }
        let mut winner = None;
        let mut max_score = 0;
        for player in &self.players {
            let score = player.score();
            if score > max_score {
                winner = Some(player.clone());
                max_score = score;
            } else if score == max_score {
                winner = None;
            }
        }
        winner
    }

    #[allow(dead_code)]
    pub fn current_player(&self) -> Option<&Player> {
        self.players.get(self.current_turn)
    }

    pub fn player_pos(&self, player_uuid: &Uuid) -> Option<usize> {
        self.players.iter().position(|c| c.id == *player_uuid)
    }

    pub fn card_left(&self) -> u8 {
        self.deck.len() as u8
    }

    fn remove_card(&mut self, card: &Card) -> Result<(), EngineError> {
        let hand = &mut self.players[self.current_turn].hand;
        match hand.iter().position(|c| c == card) {
            Some(i) => {
                hand.remove(i);
                Ok(())
            }
            None => Err(EngineError::CardNotFound),
        }
    }

    fn create_deck() -> Vec<Card> {
        use crate::engine::card::Rank;
        let mut cards = Vec::with_capacity(52);
        for suit in [Suit::Hearts, Suit::Diamonds, Suit::Clubs, Suit::Spades].iter() {
            for rank in [
                Rank::Ace,
                Rank::Two,
                Rank::Three,
                Rank::Four,
                Rank::Five,
                Rank::Six,
                Rank::Seven,
                Rank::Eight,
                Rank::Nine,
                Rank::Ten,
                Rank::Jack,
                Rank::Queen,
                Rank::King,
            ]
            .iter()
            {
                cards.push(Card {
                    suit: suit.clone(),
                    rank: rank.clone(),
                });
            }
        }
        cards.shuffle(&mut thread_rng());
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
        let mut points: [u16; 4] = [0, 0, 0, 0];
        let mut max_point: u16 = 0;
        for card in &self.hand {
            let ip = match card.suit {
                Suit::Hearts => 0,
                Suit::Diamonds => 1,
                Suit::Clubs => 2,
                Suit::Spades => 3,
            };
            points[ip] += card.points();
            max_point = max(max_point, points[ip]);
        }
        (max_point as i16) * 2 - points.iter().sum::<u16>() as i16
    }
}
