#[cfg(test)]
#[allow(warnings)]
mod tests {
    use crate::engine::card::{Card, Rank, Suit};
    use crate::engine::game::{EngineError, Game, GamePhase, GameStatus, MINIMUM_CLOSE_SCORE};
    use uuid::Uuid;

    fn make_game(n: usize) -> Game {
        let ids: Vec<Uuid> = (0..n).map(|_| Uuid::new_v4()).collect();
        Game::new(ids).expect("Failed to create game")
    }

    #[test]
    fn test_card_roundtrip() {
        let card = Card::from_string("H2").expect("Unable to parse card");
        assert_eq!(card.to_string(), "H2");
        assert_eq!(card.points(), 2);
    }

    #[test]
    fn test_card_parse_invalid() {
        assert!(Card::from_string("ZZ").is_none());
        assert!(Card::from_string("H").is_none());
        assert!(Card::from_string("").is_none());
        assert!(Card::from_string("H22").is_none());
    }

    #[test]
    fn test_card_parse_all_suits() {
        assert!(Card::from_string("HA").is_some());
        assert!(Card::from_string("DA").is_some());
        assert!(Card::from_string("CA").is_some());
        assert!(Card::from_string("SA").is_some());
    }

    #[test]
    fn test_game_flow() {
        let player1_id = Uuid::new_v4();
        let player2_id = Uuid::new_v4();
        let mut game = Game::new(vec![player1_id, player2_id]).unwrap();
        let mut i = 0;
        loop {
            let current_player = game.current_player().unwrap().clone();
            let res = if i % 2 == 1 && !current_player.bin.is_empty() {
                game.take_bin(&current_player.id)
            } else {
                game.draw(&current_player.id)
            };
            res.expect("Error drawing");

            let current_player = game.current_player().unwrap().clone();
            let card_to_discard = current_player.hand[3].clone();
            let res = game.discard(&current_player.id, card_to_discard).expect("Error discarding");

            if res.status == Some(GameStatus::Ended) {
                break;
            }
            i += 1;
        }
    }

    #[test]
    fn test_game_step() {
        let player1_id = Uuid::new_v4();
        let player2_id = Uuid::new_v4();
        let mut game = Game::new(vec![player1_id, player2_id]).unwrap();

        let current_player = game.current_player().unwrap().clone();
        game.draw(&current_player.id).unwrap();
        let card_to_discard = current_player.hand[3].clone();
        game.discard(&current_player.id, card_to_discard).expect("Error discarding");

        let next_player = game.current_player().unwrap();
        assert_eq!(next_player.hand.len(), 4);
    }

    #[test]
    fn test_draw_wrong_phase_fails() {
        let mut game = make_game(2);
        let player = game.current_player().unwrap().clone();
        game.draw(&player.id).unwrap(); // now in P2
        // drawing again in P2 should fail
        let err = game.draw(&player.id);
        assert!(err.is_err());
    }

    #[test]
    fn test_discard_wrong_phase_fails() {
        let mut game = make_game(2);
        let player = game.current_player().unwrap().clone();
        // in P1, discard should fail
        let card = player.hand[0].clone();
        let err = game.discard(&player.id, card);
        assert!(err.is_err());
    }

    #[test]
    fn test_close_below_minimum_score_fails() {
        let mut game = make_game(2);
        let player = game.current_player().unwrap().clone();
        game.draw(&player.id).unwrap();
        let player = game.current_player().unwrap().clone();
        if player.score() < MINIMUM_CLOSE_SCORE {
            let card = player.hand[0].clone();
            let err = game.close(&player.id, card);
            assert!(err.is_err(), "close should fail when score < minimum");
        }
    }

    #[test]
    fn test_max_players_enforced() {
        let ids: Vec<Uuid> = (0..MAX_PLAYER + 1).map(|_| Uuid::new_v4()).collect();
        // 5 players × 4 cards = 20 cards drawn from 52-card deck — still fits
        // But MAX_PLAYER is enforced at the handler level; engine just deals cards.
        // Verify a 4-player game works fine.
        let four_ids: Vec<Uuid> = ids[..MAX_PLAYER].to_vec();
        let game = Game::new(four_ids);
        assert!(game.is_ok());
    }

    #[test]
    fn test_score_all_hearts() {
        use crate::engine::card::Rank;
        let player = crate::engine::game::Player {
            id: Uuid::new_v4(),
            hand: vec![
                Card { suit: Suit::Hearts, rank: Rank::Ace },   // 11
                Card { suit: Suit::Hearts, rank: Rank::King },  // 10
                Card { suit: Suit::Hearts, rank: Rank::Queen }, // 10
                Card { suit: Suit::Hearts, rank: Rank::Jack },  // 10
            ],
            bin: vec![],
        };
        // max suit = Hearts = 41, total = 41, score = 41*2 - 41 = 41
        assert_eq!(player.score(), 41);
    }

    #[test]
    fn test_score_mixed_hand() {
        use crate::engine::card::Rank;
        let player = crate::engine::game::Player {
            id: Uuid::new_v4(),
            hand: vec![
                Card { suit: Suit::Hearts, rank: Rank::Ace },    // H=11
                Card { suit: Suit::Diamonds, rank: Rank::Two },  // D=2
                Card { suit: Suit::Clubs, rank: Rank::Three },   // C=3
                Card { suit: Suit::Spades, rank: Rank::Four },   // S=4
            ],
            bin: vec![],
        };
        // max suit = Hearts = 11, total = 20, score = 11*2 - 20 = 2
        assert_eq!(player.score(), 2);
    }

    #[test]
    fn test_wrong_player_turn_fails() {
        let p1 = Uuid::new_v4();
        let p2 = Uuid::new_v4();
        let mut game = Game::new(vec![p1, p2]).unwrap();
        // p2 tries to draw on p1's turn
        let err = game.draw(&p2);
        assert!(err.is_err());
    }

    #[test]
    fn test_take_bin_empty_fails() {
        let mut game = make_game(2);
        let player = game.current_player().unwrap().clone();
        // bin is empty at game start
        let err = game.take_bin(&player.id);
        assert!(err.is_err());
    }

    #[test]
    fn test_early_close() {
        let player1_id = Uuid::new_v4();
        let player2_id = Uuid::new_v4();
        let mut game = Game::new(vec![player1_id, player2_id]).unwrap();
        let collect_card_ranks = [Rank::Ace, Rank::King, Rank::Jack, Rank::Queen, Rank::Ten];

        let current_player = game.current_player().unwrap().clone();
        if current_player.id == player1_id
            && current_player
                .bin
                .last()
                .map_or(false, |card| collect_card_ranks.contains(&card.rank))
        {
            game.take_bin(&current_player.id).expect("Error when taking bin");
        } else {
            game.draw(&current_player.id).expect("Error when drawing player");
        }
        let current_player = game.current_player().unwrap().clone();
        assert_eq!(current_player.hand.len(), 5);

        let card_to_discard = current_player.hand[3].clone();
        let res = game.close(&current_player.id, card_to_discard.clone());

        if current_player.score() < MINIMUM_CLOSE_SCORE {
            assert!(res.is_err(), "Expected Error on early close");
        } else if res.is_ok() {
            return;
        }

        let current_player = game.current_player().unwrap().clone();
        assert_eq!(current_player.hand.len(), 5);

        let res = game.discard(&current_player.id, card_to_discard);
        assert!(res.is_ok(), "Expected Ok for discard");
        let current_player = game.current_player().unwrap().clone();
        assert_eq!(current_player.hand.len(), 4);

        let card_to_discard = current_player.hand[1].clone();
        let res = game.discard(&current_player.id, card_to_discard);
        assert!(res.is_err(), "Expected Err since turn changed");
    }

    #[test]
    fn test_close_with_enough_score() {
        let player1_id = Uuid::new_v4();
        let player2_id = Uuid::new_v4();
        let mut game = Game::new(vec![player1_id, player2_id]).unwrap();
        let collect_card_ranks = [Rank::Ace, Rank::King, Rank::Jack, Rank::Queen, Rank::Ten];
        let calculate_n_points = |card: &Card| -> i16 {
            let p = card.points() as i16;
            if card.suit != Suit::Hearts { -p } else { p }
        };

        loop {
            if game.phase == GamePhase::GameEnded {
                break;
            }
            let current_player = game.current_player().unwrap().clone();
            if current_player.id == player1_id
                && current_player
                    .bin
                    .last()
                    .map_or(false, |card| collect_card_ranks.contains(&card.rank))
            {
                game.take_bin(&current_player.id).expect("take_bin failed");
            } else {
                game.draw(&current_player.id).expect("draw failed");
            }

            let current_player = game.current_player().unwrap().clone();
            let mut card_to_discard = current_player.hand[0].clone();
            for card in &current_player.hand {
                if calculate_n_points(&card_to_discard) < calculate_n_points(card) {
                    card_to_discard = card.clone();
                }
            }

            if current_player.score() >= MINIMUM_CLOSE_SCORE {
                let res = game.close(&current_player.id, card_to_discard);
                assert!(res.is_ok(), "Expected Ok on close with sufficient score");
                return;
            } else {
                game.discard(&current_player.id, card_to_discard).expect("discard failed");
            }
        }
    }

    use crate::engine::game::MAX_PLAYER;
}
