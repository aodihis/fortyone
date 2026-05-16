use crate::auth::{create_token, validate_token};
use crate::engine::card::Card;
use crate::engine::game::{Game, GamePhase, MAX_PLAYER};
use crate::error::AppError;
use crate::state::state::{GameManager, GameState, GameStateStatus};
use axum::extract::Query;
use axum::http::StatusCode;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::IntoResponse,
    Json,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

// ─── Response types ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct CreateGameResponse {
    pub game_id: String,
}

#[derive(Debug, Serialize)]
pub struct JoinGameResponse {
    pub player_id: String,
    pub token: String,
}

// ─── WS message types ─────────────────────────────────────────────────────────

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
struct GameRequest {
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
    data: EndGameData,
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
    message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GameMessage {
    message_type: MessageType,
    status: String,
    data: Option<GameData>,
    message: Option<String>,
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

// ─── Shared state type alias ──────────────────────────────────────────────────

type AppState = Arc<RwLock<GameManager>>;

// ─── HTTP handlers ────────────────────────────────────────────────────────────

pub async fn create_game(State(state): State<AppState>) -> Result<Json<CreateGameResponse>, AppError> {
    let game = state.write().await.create_game();
    tracing::info!(game_id = %game.id, "Game created");
    Ok(Json(CreateGameResponse { game_id: game.id }))
}

#[derive(Debug, Deserialize)]
pub struct JoinParams {
    player_name: Option<String>,
}

pub async fn join_game(
    Path(game_id): Path<String>,
    Query(params): Query<JoinParams>,
    State(state): State<AppState>,
    // JWT secret injected via extension
    axum::Extension(jwt_secret): axum::Extension<Arc<String>>,
) -> Result<Json<JoinGameResponse>, AppError> {
    validate_game_id(&game_id)?;

    let player_name = params
        .player_name
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty())
        .ok_or_else(|| AppError::InvalidInput("player_name is required".into()))?;

    if player_name.len() > 32 {
        return Err(AppError::InvalidInput(
            "player_name must be 32 characters or fewer".into(),
        ));
    }

    let player_id = Uuid::new_v4();

    {
        let game_manager = state.read().await;
        let game_state = game_manager
            .games
            .get(&game_id)
            .ok_or(AppError::GameNotFound)?;

        if game_state.status != GameStateStatus::Lobby {
            return Err(AppError::GameAlreadyStarted);
        }
        if game_state.players.len() >= MAX_PLAYER {
            return Err(AppError::GameFull);
        }
        if game_state.players.values().any(|(name, _)| name == &player_name) {
            return Err(AppError::InvalidInput("Name already taken".into()));
        }
    }

    let token = create_token(&game_id, player_id, &player_name, &jwt_secret)?;
    tracing::info!(game_id = %game_id, player_id = %player_id, "Player joined lobby");

    Ok(Json(JoinGameResponse {
        player_id: player_id.to_string(),
        token,
    }))
}

// ─── WebSocket handler ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct WsParams {
    token: String,
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(game_id): Path<String>,
    Query(params): Query<WsParams>,
    State(state): State<AppState>,
    axum::Extension(jwt_secret): axum::Extension<Arc<String>>,
) -> impl IntoResponse {
    let claims = match validate_token(&params.token, &jwt_secret) {
        Ok(c) => c,
        Err(_) => {
            return (StatusCode::UNAUTHORIZED, "Invalid or expired token").into_response()
        }
    };

    if claims.game_id != game_id {
        return (StatusCode::UNAUTHORIZED, "Token game mismatch").into_response();
    }

    let player_id = match Uuid::parse_str(&claims.player_id) {
        Ok(id) => id,
        Err(_) => return (StatusCode::UNAUTHORIZED, "Invalid player id in token").into_response(),
    };
    let player_name = claims.player_name;

    {
        let game_manager = state.read().await;
        if !game_manager.games.contains_key(&game_id) {
            return (StatusCode::NOT_FOUND, "Game not found").into_response();
        }
        let gs = &game_manager.games[&game_id];
        if gs.status != GameStateStatus::Lobby {
            return (StatusCode::BAD_REQUEST, "Game already started").into_response();
        }
        if gs.players.len() >= MAX_PLAYER {
            return (StatusCode::BAD_REQUEST, "Game is full").into_response();
        }
        if gs.players.values().any(|(name, _)| name == &player_name) {
            return (StatusCode::BAD_REQUEST, "Name already taken").into_response();
        }
    }

    ws.on_upgrade(move |socket| {
        handle_game_connection(socket, state, player_id, player_name, game_id)
    })
}

// ─── WebSocket connection lifecycle ──────────────────────────────────────────

async fn handle_game_connection(
    socket: WebSocket,
    state: AppState,
    player_id: Uuid,
    player_name: String,
    game_id: String,
) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Message>();

    let send_task = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            if sender.send(message).await.is_err() {
                break;
            }
        }
    });

    // Register player
    {
        let mut write_state = state.write().await;
        let Some(game_state) = write_state.games.get_mut(&game_id) else {
            send_task.abort();
            return;
        };
        game_state.players.insert(player_id, (player_name.clone(), tx.clone()));

        let join_json = PlayerInfoMessage {
            status: "success".to_string(),
            message_type: MessageType::PlayerJoin,
            data: PlayerInfoData {
                players: player_list(game_state),
            },
            message: Some(format!("{player_name} joined game")),
        };
        broadcast_text(game_state, &join_json);
    }

    // Message loop
    while let Some(Ok(message)) = receiver.next().await {
        match message {
            Message::Text(msg) => match serde_json::from_str::<GameRequest>(&msg) {
                Ok(data) => {
                    handle_game_data(&state, player_id, &game_id, data).await;
                }
                Err(e) => {
                    tracing::warn!(player_id = %player_id, "Invalid message: {e}");
                    let err_msg = GameResponse {
                        status: "error".to_string(),
                        message_type: MessageType::Reply,
                    };
                    if let Ok(text) = serde_json::to_string(&err_msg) {
                        let _ = tx.send(Message::Text(text.into()));
                    }
                }
            },
            Message::Close(_) => break,
            _ => {}
        }
    }

    // Cleanup on disconnect
    {
        let mut write_state = state.write().await;
        if let Some(game_state) = write_state.games.get_mut(&game_id) {
            game_state.players.remove(&player_id);
            if let Some(game) = &mut game_state.game {
                let _ = game.remove_player(&player_id);
            }
            let leave_json = PlayerInfoMessage {
                message_type: MessageType::PlayerLeft,
                status: "success".to_string(),
                data: PlayerInfoData {
                    players: player_list(game_state),
                },
                message: Some(format!("{player_name} left game")),
            };
            broadcast_text(game_state, &leave_json);

            if game_state.players.is_empty() {
                write_state.games.remove(&game_id);
                tracing::info!(game_id = %game_id, "Game removed (all players left)");
            }
        }
    }

    send_task.abort();
}

// ─── Game action dispatcher ───────────────────────────────────────────────────

async fn handle_game_data(
    state: &AppState,
    player_id: Uuid,
    game_id: &str,
    data: GameRequest,
) {
    let mut write_state = state.write().await;
    let Some(game_state) = write_state.games.get_mut(game_id) else {
        tracing::warn!(game_id = %game_id, "handle_game_data: game not found");
        return;
    };

    if data.action == GameRequestAction::StartGame {
        if game_state.game.is_none() {
            let player_list: Vec<Uuid> = game_state.players.keys().cloned().collect();
            match Game::new(player_list) {
                Ok(game) => {
                    game_state.game = Some(game);
                    game_state.status = GameStateStatus::InProgress;
                    let event = GameEvent {
                        event_type: GameEventType::GameStart,
                        from: None,
                        to: None,
                    };
                    broadcast_game_message(game_state, event);
                }
                Err(e) => {
                    tracing::error!("Failed to start game: {e:?}");
                    send_failed_reply(game_state, &player_id);
                }
            }
        } else {
            send_failed_reply(game_state, &player_id);
        }
        return;
    }

    if game_state.game.is_none() {
        send_failed_reply(game_state, &player_id);
        return;
    }

    let game = game_state.game.as_mut().unwrap();
    let player_pos = match game.player_pos(&player_id) {
        Some(p) => p as u8,
        None => {
            tracing::warn!(player_id = %player_id, "Player not in game");
            send_failed_reply(game_state, &player_id);
            return;
        }
    };

    match data.action {
        GameRequestAction::Draw => match game.draw(&player_id) {
            Ok(_) => broadcast_game_message(
                game_state,
                GameEvent { event_type: GameEventType::Draw, from: None, to: Some(player_pos) },
            ),
            Err(e) => {
                tracing::debug!("Draw failed: {e:?}");
                send_failed_reply(game_state, &player_id);
            }
        },

        GameRequestAction::TakeBin => match game.take_bin(&player_id) {
            Ok(_) => broadcast_game_message(
                game_state,
                GameEvent {
                    event_type: GameEventType::TakeBin,
                    from: Some(player_pos),
                    to: Some(player_pos),
                },
            ),
            Err(e) => {
                tracing::debug!("TakeBin failed: {e:?}");
                send_failed_reply(game_state, &player_id);
            }
        },

        GameRequestAction::Discard => {
            let card = match parse_card(&data.card) {
                Some(c) => c,
                None => {
                    send_failed_reply(game_state, &player_id);
                    return;
                }
            };
            let next_turn = (game.current_turn + 1) % game.players.len();
            match game.discard(&player_id, card) {
                Ok(res) => {
                    if game.phase == GamePhase::GameEnded {
                        let event = GameEvent {
                            event_type: GameEventType::Close,
                            from: Some(player_pos),
                            to: Some(next_turn as u8),
                        };
                        broadcast_game_message(game_state, event);
                        game_state.status = GameStateStatus::Finished;
                        broadcast_end_game_message(game_state);
                    } else {
                        let event = GameEvent {
                            event_type: GameEventType::Discard,
                            from: Some(player_pos),
                            to: Some(res.next_turn),
                        };
                        broadcast_game_message(game_state, event);
                    }
                }
                Err(e) => {
                    tracing::debug!("Discard failed: {e:?}");
                    send_failed_reply(game_state, &player_id);
                }
            }
        }

        GameRequestAction::Close => {
            let card = match parse_card(&data.card) {
                Some(c) => c,
                None => {
                    send_failed_reply(game_state, &player_id);
                    return;
                }
            };
            let next_turn = (game.current_turn + 1) % game.players.len();
            match game.close(&player_id, card) {
                Ok(_) => {
                    let event = GameEvent {
                        event_type: GameEventType::Close,
                        from: Some(player_pos),
                        to: Some(next_turn as u8),
                    };
                    broadcast_game_message(game_state, event);
                    game_state.status = GameStateStatus::Finished;
                    broadcast_end_game_message(game_state);
                }
                Err(e) => {
                    tracing::debug!("Close failed: {e:?}");
                    send_failed_reply(game_state, &player_id);
                }
            }
        }

        GameRequestAction::StartGame => {} // handled above
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn parse_card(card_str: &Option<String>) -> Option<Card> {
    card_str.as_deref().and_then(Card::from_string)
}

fn player_list(game_state: &GameState) -> Vec<PlayerData> {
    game_state
        .players
        .values()
        .map(|(name, _)| PlayerData {
            name: name.clone(),
            hand: vec![],
            bin: vec![],
        })
        .collect()
}

fn send_failed_reply(game_state: &mut GameState, player_id: &Uuid) {
    let res = GameResponse {
        status: "failed".to_string(),
        message_type: MessageType::Reply,
    };
    let Ok(text) = serde_json::to_string(&res) else { return };
    if let Some((_, tx)) = game_state.players.get(player_id) {
        if let Err(e) = tx.send(Message::Text(text.into())) {
            tracing::warn!("Failed to send reply to {player_id}: {e}");
        }
    }
}

fn broadcast_text<T: Serialize>(game_state: &mut GameState, msg: &T) {
    let Ok(text) = serde_json::to_string(msg) else { return };
    for (_, (_, tx)) in &game_state.players {
        if let Err(e) = tx.send(Message::Text(text.clone().into())) {
            tracing::warn!("Broadcast send error: {e}");
        }
    }
}

fn broadcast_end_game_message(game_state: &mut GameState) {
    let Some(game) = &game_state.game else { return };
    let scores: Vec<EndGameScores> = game
        .players
        .iter()
        .map(|player| EndGameScores {
            name: game_state
                .players
                .get(&player.id)
                .map(|(n, _)| n.clone())
                .unwrap_or_default(),
            score: player.score(),
            hand: player.hand.iter().map(|c| c.to_string()).collect(),
        })
        .collect();
    let winner_name = game.winner().and_then(|w| {
        game_state.players.get(&w.id).map(|(n, _)| n.clone())
    });
    let msg = EndGameMessage {
        status: "success".to_string(),
        message_type: MessageType::EndGame,
        data: EndGameData { winner_name, players: scores },
    };
    broadcast_text(game_state, &msg);
}

fn broadcast_game_message(game_state: &mut GameState, game_event: GameEvent) {
    let Some(game) = &game_state.game else { return };
    let player_ids: Vec<Uuid> = game_state.players.keys().cloned().collect();
    for id in &player_ids {
        let Some(pos) = game.player_pos(id) else { continue };
        let mut players = vec![];
        for (i, p) in game.players.iter().enumerate() {
            let name = game_state
                .players
                .get(&p.id)
                .map(|(n, _)| n.clone())
                .unwrap_or_default();
            players.push(PlayerData {
                name,
                hand: if p.id == *id {
                    p.hand.iter().map(|c| c.to_string()).collect()
                } else {
                    vec!["".to_string(); game.players[i].hand.len()]
                },
                bin: p.bin.iter().map(|c| c.to_string()).collect(),
            });
        }
        let msg = GameMessage {
            message_type: MessageType::GameEvent,
            status: "success".to_string(),
            message: None,
            data: Some(GameData {
                player_id: *id,
                player_pos: pos as u8,
                num_of_players: game_state.players.len() as u8,
                card_left: game.card_left(),
                current_turn: game.current_turn as u8,
                current_phase: game.phase.clone(),
                event: game_event.clone(),
                players,
            }),
        };
        let Ok(text) = serde_json::to_string(&msg) else { continue };
        if let Some((_, tx)) = game_state.players.get(id) {
            if let Err(e) = tx.send(Message::Text(text.into())) {
                tracing::warn!("Failed to send game message to {id}: {e}");
            }
        }
    }
}

fn validate_game_id(game_id: &str) -> Result<(), AppError> {
    if game_id.is_empty()
        || game_id.len() > 12
        || !game_id.chars().all(|c| c.is_ascii_alphanumeric())
    {
        return Err(AppError::InvalidInput("Invalid game ID format".into()));
    }
    Ok(())
}
