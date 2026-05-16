use crate::engine::card::Card;
use crate::engine::game::{Game, GamePhase, MAX_PLAYER};
use crate::handlers::error::GameError;
use crate::state::state::{GameManager, GameState, GameStateStatus};
use axum::extract::Query;
use axum::http::StatusCode;
use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, Path, State},
    response::IntoResponse,
    Json,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::cmp::PartialEq;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct CreateGameResponse {
    game_id: String,
    num_of_players: usize,
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
enum GameRequestAction {
    StartGame,
    Draw,
    TakeBin,
    Discard,
    Close,
}


#[derive(Debug, Deserialize)]
struct GameRequest  {
    action: GameRequestAction,
    card: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
struct GameResponse {
    status: String,
    message_type: MessageType,
}

#[derive(Debug, Serialize, Deserialize)]
struct EndGameScores {
    name: String,
    score: i16,
    hand: Vec<String>,
}
#[derive(Debug, Serialize, Deserialize)]
struct EndGameData {
    winner_name: Option<String>,
    players: Vec<EndGameScores>,
}

#[derive(Debug, Serialize, Deserialize)]
struct EndGameMessage {
    status: String,
    message_type: MessageType,
    data: EndGameData
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum MessageType {
    PlayerJoin,
    PlayerLeft,
    Reply,
    GameEvent,
    EndGame,
}

#[derive(Debug, Serialize, Deserialize)]
struct PlayerInfoData {
    players: Vec<PlayerData>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PlayerInfoMessage {
    message_type: MessageType,
    status: String,
    data: PlayerInfoData,
    message: Option<String>
}

#[derive(Debug, Serialize, Deserialize)]
struct GameMessage {
    message_type: MessageType,
    status: String,
    data: Option<GameData>,
    message: Option<String>
}
#[derive(Debug, Serialize, Deserialize)]
struct GameData {
    player_id: Uuid,
    player_pos: u8,
    num_of_players: u8,
    card_left: u8,
    current_turn: u8,
    current_phase: GamePhase,
    event: GameEvent,
    players: Vec<PlayerData>,

}
#[derive(Debug, Serialize, Deserialize)]
struct PlayerData {
    name: String,
    hand: Vec<String>,
    bin: Vec<String>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
enum GameEventType {
    GameStart,
    Draw,
    TakeBin,
    Discard,
    Close,

}
#[derive(Debug, Serialize, Deserialize, Clone)]
struct GameEvent {
    event_type: GameEventType,
    from: Option<u8>,
    to: Option<u8>,
}


pub async fn create_game(State(state): State<Arc<RwLock<GameManager>>>) -> Result<Json<CreateGameResponse>, GameError>{
    let game = state.write().await.create_game();
    Ok(Json(CreateGameResponse{
        game_id: game.id.clone(),
        num_of_players: game.players.len(),
    }))
}


pub async fn game(ws: WebSocketUpgrade, Path(game_id): Path<String>, Query(params): Query<HashMap<String, String>>, State(state): State<Arc<RwLock<GameManager>>>) -> impl IntoResponse {

    let player_id = Uuid::new_v4();
    let player_name: String;
    {
        let game_manager = state.write().await;

        if !game_manager.games.contains_key(&game_id) {
            return Err((StatusCode::BAD_REQUEST, "Game not found.").into_response());
        }

        if game_manager.games[&game_id].players.len() >= MAX_PLAYER {
            return Err((StatusCode::BAD_REQUEST, "Max player has been reached.").into_response());
        }

        if game_manager.games[&game_id].status != GameStateStatus::Lobby {
            return Err((StatusCode::BAD_REQUEST, "Game already started.").into_response());
        }

        player_name = {
            match params.get("player_name") {
                Some(name) => name.to_string(),
                None => {format!("Player {}",game_manager.games[&game_id].players.len() )}
            }
        };

        if game_manager.games[&game_id].players.values().any(|p| {
            p.0 == player_name
        }) {
            return Err((StatusCode::BAD_REQUEST, "Name already taken.").into_response());
        }

        if game_manager.games.get(&game_id).is_none() {
            eprintln!("Game with ID {} not found", game_id);
        }
    }
    Ok(ws.on_upgrade(move |socket| handle_game_connection(socket, state, player_id,player_name, game_id)))
}


async fn handle_game_connection(socket: WebSocket, state: Arc<RwLock<GameManager>>, player_id: Uuid, player_name: String, game_id: String) {


    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Message>();

    let send_task = tokio::spawn(async move {
       while let Some(message) = rx.recv().await {
           if sender.send(message).await.is_err() {
               continue;
           }
       }
    });

    {
        let mut write_state = state.write().await;
        let game_state = write_state.games.get_mut(&game_id).unwrap();
        game_state.players.insert(player_id, (player_name.clone(), tx));
        let join_message = format!("{} joined game", player_name);
        let join_json = PlayerInfoMessage {
            status: "success".to_string(),
            message_type: MessageType::PlayerJoin,
            data: PlayerInfoData {
                players: game_state.players.iter().map(|(_, v)| {
                    PlayerData {
                        name: v.0.clone(),
                        hand: vec![],
                        bin: vec![],
                    }
                }).collect(),
            },
            message: Some(join_message),
        };
        broadcast_message(serde_json::to_string(&join_json).unwrap().to_string(), game_state).await;



    }

    while let Some(Ok(message)) = receiver.next().await {

        match message {
            Message::Text(msg) => {
                match serde_json::from_str::<GameRequest>(&msg) {
                    Ok(data) => {
                        handle_game_data(&state, player_id, &game_id, Json::from(data)).await;
                    }
                    Err(_) => {}
                }
            },
            Message::Close(_) => {
            }
            _ => {}
        }
    }

    {
        let mut write_state = state.write().await;
        if let Some(game_state) = write_state.games.get_mut(&game_id) {
            game_state.players.remove(&player_id);
            if let Some(game) = &mut game_state.game {
                game.remove_player(&player_id).unwrap();
            }
            let leave_message = format!("{} left game", player_name.clone());
            let leave_json = PlayerInfoMessage {
                message_type: MessageType::PlayerLeft,
                status: "success".to_string(),
                data: PlayerInfoData {
                    players: game_state.players.iter().map(|(_, v)| {
                        PlayerData {
                            name: v.0.clone(),
                            hand: vec![],
                            bin: vec![],
                        }
                    }).collect(),
                },
                message: Some(leave_message),
            };
            broadcast_message(serde_json::to_string(&leave_json).unwrap().to_string(), game_state).await;
        }
    }

    send_task.abort();
}

async fn broadcast_message(message: String, game_state: &mut GameState) {
    // println!("broadcasting message: {}", message);
    for (_, (_name, tx)) in game_state.players.iter() {
        if let Err(e) = tx.send(Message::Text(message.clone().into())) {
            eprintln!("Error sending message: {:?}", e.to_string());
        }
    }
}

async fn handle_game_data( state: &Arc<RwLock<GameManager>>, player_id: Uuid, game_id : &String, data: Json<GameRequest>) {
    let mut write_state = state.write().await;
    let game_state: &mut GameState = write_state.games.get_mut(game_id).unwrap();
    let game_res: &mut Option<Game> = &mut game_state.game;

    if data.action == GameRequestAction::StartGame {
        match game_res {
            None => {
                let player_list = game_state.players.keys().cloned().collect();
                let game = Game::new(player_list);
                game_state.game = Some(game);
                game_state.status = GameStateStatus::InProgress;
                let game_event = GameEvent {
                    event_type: GameEventType::GameStart,
                    from: None,
                    to: None,
                };
                broadcast_game_message(game_state, game_event);
            }

            Some(_game) => {
                let res = GameResponse { status: "failed".to_string(), message_type: MessageType::Reply  };
                let (_, rx) = game_state.players.get_mut(&player_id).unwrap();
                if let Err(e) = rx.send(Message::Text(serde_json::to_string(&res).unwrap().into())) {
                    eprintln!("Error sending message: {:?}", e);
                }
            }
        };
        return;
    }

    if game_res.is_none() {
        send_failed_reply(game_state, &player_id);
        return;
    };
    let game = game_res.as_mut().unwrap();
    let player_pos = game.player_pos(&player_id).unwrap();
    match data.action {
        GameRequestAction::Draw => {
            match game.draw(&player_id) {
                Ok(_) => {
                    let game_event = GameEvent {
                        event_type: GameEventType::Draw,
                        from: None,
                        to: Option::from(player_pos as u8),
                    };
                    broadcast_game_message(game_state, game_event);
                }
                Err(_) => { send_failed_reply(game_state, &player_id);}
            }
        },
        GameRequestAction::TakeBin => {
            match game.take_bin(&player_id) {
                Ok(_) => {
                    let game_event = GameEvent {
                        event_type: GameEventType::TakeBin,
                        from: Option::from(player_pos as u8),
                        to: Option::from(player_pos as u8),
                    };
                    broadcast_game_message(game_state, game_event);
                }
                Err(_) => { send_failed_reply(game_state, &player_id);}
            }
        },
        GameRequestAction::Discard => {
            let card_data = match &data.card {
                Some(card_data) => card_data,
                None => {
                    send_failed_reply(game_state, &player_id);
                return;
                }
            };
            let card = {
                match Card::from_string(card_data) {
                    Some(card) => card,
                    _ => {
                        send_failed_reply(game_state, &player_id);
                        return;
                    }
                }
            };
            match game.discard(&player_id, card) {
                Ok(_) => {
                    if game.phase == GamePhase::GameEnded {
                        let game_event = GameEvent {
                            event_type: GameEventType::Close,
                            from: Option::from(player_pos as u8),
                            to: Option::from(game.current_turn as u8),
                        };
                        broadcast_game_message(game_state, game_event);
                        game_state.status = GameStateStatus::Finished;
                        broadcast_end_game_message(game_state);
                    } else {
                        let game_event = GameEvent {
                            event_type: GameEventType::Discard,
                            from: Option::from(player_pos as u8),
                            to: Option::from(game.current_turn as u8),
                        };
                        broadcast_game_message(game_state, game_event);
                    }
                }
                Err(e) => {
                    println!("Error discarding game: {:?}", e);
                    send_failed_reply(game_state, &player_id);
                }
            }
        },
        GameRequestAction::Close => {
            let card_data = match &data.card {
                Some(card_data) => card_data,
                None => {
                    send_failed_reply(game_state, &player_id);
                    return;
                }
            };
            let card = {
                match Card::from_string(card_data) {
                    Some(card) => card,
                    _ => {
                        send_failed_reply(game_state, &player_id);
                        return;
                    }
                }
            };

            match game.close(&player_id, card) {
                Ok(_) => {
                    let game_event = GameEvent {
                        event_type: GameEventType::Close,
                        from: Option::from(player_pos as u8),
                        to: Option::from(game.current_turn as u8),
                    };
                    broadcast_game_message(game_state, game_event);
                    game_state.status = GameStateStatus::Finished;
                    broadcast_end_game_message(game_state);
                }
                Err(_) => { send_failed_reply(game_state, &player_id);}
            }
        },
        _ => {}
    }
}


fn send_failed_reply(game_state: &mut GameState, player_id: &Uuid) {
    let res = GameResponse { status: "failed".to_string(), message_type: MessageType::Reply };
    let (_, rx) = game_state.players.get_mut(player_id).unwrap();
    if let Err(e) = rx.send(Message::Text(serde_json::to_string(&res).unwrap().into())) {
        eprintln!("Error sending message: {:?}", e);
    }
}

fn broadcast_end_game_message(game_state: &mut GameState) {
    let game = game_state.game.as_ref().unwrap();
    let scores = game.players.iter().map(|player|  {
        EndGameScores {
            name: game_state.players[&player.id].0.clone(),
            score: player.score(),
            hand: player.hand.iter().map(|card|card.to_string()).collect(),
        }
    }).collect();
    let winner = game.winner();
    let msg = EndGameMessage {
        status: "success".to_string(),
        message_type: MessageType::EndGame,
        data: EndGameData {
            winner_name: if let Some(winner) = winner {
                let name = game_state.players[&winner.id].0.clone();
                Some(name)
            }else {
                None
            },
            players: scores
        },
    };

    for (_, (_name, con)) in game_state.players.iter() {
        if let Err(e) = con.send(Message::Text(serde_json::to_string(&msg).unwrap().into())) {
            eprintln!("Error sending message: {:?}", e);
        }
    }

}
fn broadcast_game_message(game_state: &mut GameState, game_event: GameEvent) {
        let game = game_state.game.as_ref().unwrap();
        for (id, (_name, con)) in game_state.players.iter() {
            let msg = build_game_message(id, game, game_state, game_event.clone());

            if let Err(e) = con.send(Message::Text(serde_json::to_string(&msg).unwrap().into())) {
                eprintln!("Error sending message: {:?}", e);
            }
        }
}

fn build_game_message(id: &Uuid, game: &Game, game_state: &GameState, game_event: GameEvent) -> GameMessage {
    let player_pos = match game.player_pos(&id){
        None => {panic!("Player {} not found", id)}
        Some(i) => {i as u8}
    };

    let mut players = vec![];

    for i in 0..game.players.len() {
        let p_id = game.players[i].id;
        let (name,_) = game_state.players.get(&p_id).unwrap();
        players.push(PlayerData {
            name: name.to_string(),
            hand: {
                if p_id  == *id {
                    game.players[i].hand.iter().map(|card| card.to_string()).collect()
                } else {
                    vec!["".to_string();4]
                }

            },
            bin: game.players[i].bin.iter().map(|card| card.to_string()).collect(),
        })
    }


    let game_data =  GameData{
        player_id: id.clone(),
        player_pos,
        num_of_players: game_state.players.len() as u8,
        card_left: game.card_left(),
        current_turn: game.current_turn as u8,
        current_phase: game.phase.clone(),
        event: game_event,
        players,
    };

    let msg = GameMessage{
        message_type: MessageType::GameEvent,
        status: "success".to_string(),
        message: None,
        data: Some(game_data),
    };

    msg
}
