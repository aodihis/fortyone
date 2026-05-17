use crate::auth::{create_token, validate_token};
use crate::engine::card::Card;
use crate::engine::game::{Game, GamePhase, MAX_PLAYER};
use crate::error::AppError;
use crate::state::redis_types::PersistedGameState;
use crate::state::state::{AppState, GameStateStatus};
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
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
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
    PlayerDisconnected,
    PlayerReconnected,
    Reply,
    GameEvent,
    EndGame,
    GameAbandoned,
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
    Reconnect,
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

#[derive(Debug, Serialize, Deserialize)]
struct PlayerDisconnectData {
    players: Vec<PlayerData>,
    reconnect_timeout_secs: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct PlayerDisconnectMessage {
    message_type: MessageType,
    status: String,
    data: PlayerDisconnectData,
    message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GameAbandonedMessage {
    message_type: MessageType,
    status: String,
    winner_name: Option<String>,
    message: String,
}

// ─── HTTP handlers ────────────────────────────────────────────────────────────

pub async fn create_game(
    State(state): State<AppState>,
) -> Result<Json<CreateGameResponse>, AppError> {
    let game_id = state.create_game().await?;
    tracing::info!(game_id = %game_id, "Game created");
    Ok(Json(CreateGameResponse { game_id }))
}

#[derive(Debug, Deserialize)]
pub struct JoinParams {
    player_name: Option<String>,
}

pub async fn join_game(
    Path(game_id): Path<String>,
    Query(params): Query<JoinParams>,
    State(state): State<AppState>,
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

    let persisted = state
        .get_game(&game_id)
        .await?
        .ok_or(AppError::GameNotFound)?;

    if persisted.status != GameStateStatus::Lobby {
        return Err(AppError::GameAlreadyStarted);
    }
    if persisted.players.len() >= MAX_PLAYER {
        return Err(AppError::GameFull);
    }
    if persisted.players.iter().any(|(_, name)| name == &player_name) {
        return Err(AppError::InvalidInput("Name already taken".into()));
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

    let persisted = match state.get_game(&game_id).await {
        Ok(Some(p)) => p,
        Ok(None) => return (StatusCode::NOT_FOUND, "Game not found").into_response(),
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Storage error").into_response(),
    };

    let is_reconnect = persisted.players.iter().any(|(id, _)| *id == player_id);

    if is_reconnect {
        if persisted.status == GameStateStatus::Finished {
            return (StatusCode::BAD_REQUEST, "Game already finished").into_response();
        }
    } else {
        if persisted.status != GameStateStatus::Lobby {
            return (StatusCode::BAD_REQUEST, "Game already started").into_response();
        }
        if persisted.players.len() >= MAX_PLAYER {
            return (StatusCode::BAD_REQUEST, "Game is full").into_response();
        }
        if persisted.players.iter().any(|(_, name)| name == &player_name) {
            return (StatusCode::BAD_REQUEST, "Name already taken").into_response();
        }
    }

    ws.on_upgrade(move |socket| {
        handle_game_connection(socket, state, player_id, player_name, game_id, is_reconnect)
    })
}

// ─── WebSocket connection lifecycle ──────────────────────────────────────────

async fn handle_game_connection(
    socket: WebSocket,
    state: AppState,
    player_id: Uuid,
    player_name: String,
    game_id: String,
    is_reconnect: bool,
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

    if is_reconnect {
        let lock = state.game_lock(&game_id);
        let _guard = lock.lock().await;

        let persisted = match state.get_game(&game_id).await {
            Ok(Some(p)) => p,
            _ => {
                send_task.abort();
                return;
            }
        };

        // Player may have been removed by the timer between ws_handler and here
        if !persisted.players.iter().any(|(id, _)| *id == player_id) {
            send_task.abort();
            return;
        }

        // Cancel pending disconnect timer (abort while holding lock — safe cancellation point)
        if let Some(game_timers) = state.reconnect_timers.get(&game_id) {
            if let Some((_, handle)) = game_timers.remove(&player_id) {
                handle.abort();
            }
        }

        state
            .senders
            .entry(game_id.clone())
            .or_default()
            .insert(player_id, (player_name.clone(), tx.clone()));

        let reconnect_msg = PlayerInfoMessage {
            message_type: MessageType::PlayerReconnected,
            status: "success".to_string(),
            data: PlayerInfoData { players: player_list(&persisted) },
            message: Some(format!("{player_name} reconnected")),
        };
        broadcast_text(&state, &game_id, &reconnect_msg);

        // Resend full game state so the reconnecting client catches up
        if persisted.status == GameStateStatus::InProgress {
            if let Some(game) = &persisted.game {
                if game.player_pos(&player_id).is_some() {
                    send_game_message_to_player(
                        &persisted,
                        player_id,
                        &tx,
                        GameEvent { event_type: GameEventType::Reconnect, from: None, to: None },
                    );
                }
            }
        }

        tracing::info!(game_id = %game_id, player_id = %player_id, "Player reconnected");
    } else {
        // New join: register sender + add to persisted state
        let lock = state.game_lock(&game_id);
        let _guard = lock.lock().await;

        let mut persisted = match state.get_game(&game_id).await {
            Ok(Some(p)) => p,
            _ => {
                send_task.abort();
                return;
            }
        };

        state
            .senders
            .entry(game_id.clone())
            .or_default()
            .insert(player_id, (player_name.clone(), tx.clone()));

        persisted.players.push((player_id, player_name.clone()));

        if let Err(e) = state.save_game(&persisted).await {
            tracing::error!("Failed to register player in Redis: {e}");
            send_task.abort();
            return;
        }

        let join_msg = PlayerInfoMessage {
            status: "success".to_string(),
            message_type: MessageType::PlayerJoin,
            data: PlayerInfoData { players: player_list(&persisted) },
            message: Some(format!("{player_name} joined game")),
        };
        broadcast_text(&state, &game_id, &join_msg);
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
                        message: None,
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
        let lock = state.game_lock(&game_id);
        let _guard = lock.lock().await;

        if let Some(game_senders) = state.senders.get(&game_id) {
            game_senders.remove(&player_id);
        }

        match state.get_game(&game_id).await {
            Ok(Some(persisted)) => {
                if persisted.status == GameStateStatus::InProgress {
                    // Grace period: keep player in Redis, start reconnect timer
                    let timeout_secs = state.reconnect_timeout_secs;
                    let disconnect_msg = PlayerDisconnectMessage {
                        message_type: MessageType::PlayerDisconnected,
                        status: "success".to_string(),
                        data: PlayerDisconnectData {
                            players: player_list(&persisted),
                            reconnect_timeout_secs: timeout_secs,
                        },
                        message: Some(format!(
                            "{player_name} disconnected, {timeout_secs}s to reconnect"
                        )),
                    };
                    broadcast_text(&state, &game_id, &disconnect_msg);

                    let timer_state = state.clone();
                    let timer_game_id = game_id.clone();
                    let timer_player_name = player_name.clone();
                    let task = tokio::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_secs(timeout_secs)).await;
                        handle_reconnect_timeout(
                            &timer_state,
                            &timer_game_id,
                            player_id,
                            &timer_player_name,
                        )
                        .await;
                    });
                    state
                        .reconnect_timers
                        .entry(game_id.clone())
                        .or_default()
                        .insert(player_id, task.abort_handle());

                    tracing::info!(
                        game_id = %game_id,
                        player_id = %player_id,
                        timeout_secs,
                        "Reconnect timer started"
                    );
                } else {
                    // Lobby: remove immediately
                    let mut persisted = persisted;
                    persisted.players.retain(|(id, _)| *id != player_id);

                    let leave_msg = PlayerInfoMessage {
                        message_type: MessageType::PlayerLeft,
                        status: "success".to_string(),
                        data: PlayerInfoData { players: player_list(&persisted) },
                        message: Some(format!("{player_name} left game")),
                    };
                    broadcast_text(&state, &game_id, &leave_msg);

                    if persisted.players.is_empty() {
                        if let Err(e) = state.delete_game(&game_id).await {
                            tracing::error!("Failed to delete game {game_id}: {e}");
                        } else {
                            tracing::info!(game_id = %game_id, "Game removed (all players left)");
                        }
                    } else if let Err(e) = state.save_game(&persisted).await {
                        tracing::error!("Failed to save game after disconnect: {e}");
                    }
                }
            }
            Ok(None) => {}
            Err(e) => tracing::error!("Redis error on disconnect cleanup: {e}"),
        }
    }

    send_task.abort();
}

// ─── Reconnect timeout handler ────────────────────────────────────────────────

async fn handle_reconnect_timeout(
    state: &AppState,
    game_id: &str,
    player_id: Uuid,
    player_name: &str,
) {
    let lock = state.game_lock(game_id);
    let _guard = lock.lock().await;

    if let Some(game_timers) = state.reconnect_timers.get(game_id) {
        game_timers.remove(&player_id);
    }

    let mut persisted = match state.get_game(game_id).await {
        Ok(Some(p)) => p,
        Ok(None) => return,
        Err(e) => {
            tracing::error!("Redis error in reconnect timeout for {game_id}: {e}");
            return;
        }
    };

    persisted.players.retain(|(id, _)| *id != player_id);
    if let Some(game) = &mut persisted.game {
        let _ = game.remove_player(&player_id);
    }

    let remaining = persisted.players.len();
    tracing::info!(
        game_id = %game_id,
        player_id = %player_id,
        remaining,
        "Reconnect timeout expired, removing player"
    );

    match remaining {
        0 => {
            if let Err(e) = state.delete_game(game_id).await {
                tracing::error!("Failed to delete game {game_id}: {e}");
            }
        }
        1 => {
            // Single player left — they auto-win
            let winner_name = persisted.players.first().map(|(_, n)| n.clone());
            persisted.status = GameStateStatus::Finished;
            let msg = GameAbandonedMessage {
                message_type: MessageType::GameAbandoned,
                status: "success".to_string(),
                winner_name: winner_name.clone(),
                message: format!(
                    "{player_name} did not reconnect. {} wins!",
                    winner_name.as_deref().unwrap_or("Remaining player")
                ),
            };
            broadcast_text(state, game_id, &msg);
            if let Err(e) = state.save_game(&persisted).await {
                tracing::error!("Failed to save game after auto-win: {e}");
            }
        }
        _ => {
            // Multiple players remain but one timed out — abandon
            persisted.status = GameStateStatus::Finished;
            let msg = GameAbandonedMessage {
                message_type: MessageType::GameAbandoned,
                status: "success".to_string(),
                winner_name: None,
                message: format!("{player_name} did not reconnect. Game abandoned."),
            };
            broadcast_text(state, game_id, &msg);
            if let Err(e) = state.save_game(&persisted).await {
                tracing::error!("Failed to save game after abandon: {e}");
            }
        }
    }
}

// ─── Game action dispatcher ───────────────────────────────────────────────────

async fn handle_game_data(
    state: &AppState,
    player_id: Uuid,
    game_id: &str,
    data: GameRequest,
) {
    let lock = state.game_lock(game_id);
    let _guard = lock.lock().await;

    let mut persisted = match state.get_game(game_id).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            tracing::warn!(game_id = %game_id, "handle_game_data: game not found");
            return;
        }
        Err(e) => {
            tracing::error!("Storage error in handle_game_data: {e}");
            send_storage_error_reply(state, game_id, &player_id, &e);
            return;
        }
    };

    if data.action == GameRequestAction::StartGame {
        if persisted.game.is_none() {
            let player_list: Vec<Uuid> = persisted.players.iter().map(|(id, _)| *id).collect();
            match Game::new(player_list) {
                Ok(game) => {
                    persisted.game = Some(game);
                    persisted.status = GameStateStatus::InProgress;
                    let event = GameEvent {
                        event_type: GameEventType::GameStart,
                        from: None,
                        to: None,
                    };
                    broadcast_game_message(state, game_id, &persisted, event);
                    if let Err(e) = state.save_game(&persisted).await {
                        tracing::error!("Failed to save game after start: {e}");
                        send_storage_error_reply(state, game_id, &player_id, &e);
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to start game: {e:?}");
                    send_failed_reply(state, game_id, &player_id);
                }
            }
        } else {
            send_failed_reply(state, game_id, &player_id);
        }
        return;
    }

    if persisted.game.is_none() {
        send_failed_reply(state, game_id, &player_id);
        return;
    }

    let game = persisted.game.as_mut().unwrap();
    let player_pos = match game.player_pos(&player_id) {
        Some(p) => p as u8,
        None => {
            tracing::warn!(player_id = %player_id, "Player not in game");
            send_failed_reply(state, game_id, &player_id);
            return;
        }
    };

    let mut save = true;
    match data.action {
        GameRequestAction::Draw => match game.draw(&player_id) {
            Ok(_) => broadcast_game_message(
                state,
                game_id,
                &persisted,
                GameEvent { event_type: GameEventType::Draw, from: None, to: Some(player_pos) },
            ),
            Err(e) => {
                tracing::debug!("Draw failed: {e:?}");
                send_failed_reply(state, game_id, &player_id);
                save = false;
            }
        },

        GameRequestAction::TakeBin => match game.take_bin(&player_id) {
            Ok(_) => broadcast_game_message(
                state,
                game_id,
                &persisted,
                GameEvent {
                    event_type: GameEventType::TakeBin,
                    from: Some(player_pos),
                    to: Some(player_pos),
                },
            ),
            Err(e) => {
                tracing::debug!("TakeBin failed: {e:?}");
                send_failed_reply(state, game_id, &player_id);
                save = false;
            }
        },

        GameRequestAction::Discard => {
            let card = match parse_card(&data.card) {
                Some(c) => c,
                None => {
                    send_failed_reply(state, game_id, &player_id);
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
                        broadcast_game_message(state, game_id, &persisted, event);
                        persisted.status = GameStateStatus::Finished;
                        broadcast_end_game_message(state, game_id, &persisted);
                    } else {
                        let event = GameEvent {
                            event_type: GameEventType::Discard,
                            from: Some(player_pos),
                            to: Some(res.next_turn),
                        };
                        broadcast_game_message(state, game_id, &persisted, event);
                    }
                }
                Err(e) => {
                    tracing::debug!("Discard failed: {e:?}");
                    send_failed_reply(state, game_id, &player_id);
                    save = false;
                }
            }
        }

        GameRequestAction::Close => {
            let card = match parse_card(&data.card) {
                Some(c) => c,
                None => {
                    send_failed_reply(state, game_id, &player_id);
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
                    broadcast_game_message(state, game_id, &persisted, event);
                    persisted.status = GameStateStatus::Finished;
                    broadcast_end_game_message(state, game_id, &persisted);
                }
                Err(e) => {
                    tracing::debug!("Close failed: {e:?}");
                    send_failed_reply(state, game_id, &player_id);
                    save = false;
                }
            }
        }

        GameRequestAction::StartGame => {} // handled above
    }

    if save {
        if let Err(e) = state.save_game(&persisted).await {
            tracing::error!("Failed to save game state: {e}");
            send_storage_error_reply(state, game_id, &player_id, &e);
        }
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn parse_card(card_str: &Option<String>) -> Option<Card> {
    card_str.as_deref().and_then(Card::from_string)
}

fn player_list(persisted: &PersistedGameState) -> Vec<PlayerData> {
    persisted
        .players
        .iter()
        .map(|(_, name)| PlayerData {
            name: name.clone(),
            hand: vec![],
            bin: vec![],
        })
        .collect()
}

fn send_reply(state: &AppState, game_id: &str, player_id: &Uuid, status: &str, message: Option<String>) {
    let res = GameResponse {
        status: status.to_string(),
        message_type: MessageType::Reply,
        message,
    };
    let Ok(text) = serde_json::to_string(&res) else { return };
    if let Some(game_senders) = state.senders.get(game_id) {
        if let Some(entry) = game_senders.get(player_id) {
            let (_, tx) = entry.value();
            if let Err(e) = tx.send(Message::Text(text.into())) {
                tracing::warn!("Failed to send reply to {player_id}: {e}");
            }
        }
    }
}

fn send_failed_reply(state: &AppState, game_id: &str, player_id: &Uuid) {
    send_reply(state, game_id, player_id, "failed", None);
}

fn send_storage_error_reply(state: &AppState, game_id: &str, player_id: &Uuid, e: &crate::error::AppError) {
    use crate::error::{classify_pool_error, classify_redis_error};
    let msg = match e {
        crate::error::AppError::Redis(pool_err) => classify_pool_error(pool_err).1,
        crate::error::AppError::RedisCmd(redis_err) => classify_redis_error(redis_err).1,
        _ => "Service temporarily unavailable — please try again",
    };
    send_reply(state, game_id, player_id, "error", Some(msg.to_string()));
}

fn broadcast_text<T: Serialize>(state: &AppState, game_id: &str, msg: &T) {
    let Ok(text) = serde_json::to_string(msg) else { return };
    let Some(game_senders) = state.senders.get(game_id) else { return };
    for entry in game_senders.iter() {
        let (_, tx) = entry.value();
        if let Err(e) = tx.send(Message::Text(text.clone().into())) {
            tracing::warn!("Broadcast send error: {e}");
        }
    }
}

fn broadcast_end_game_message(
    state: &AppState,
    game_id: &str,
    persisted: &PersistedGameState,
) {
    let Some(game) = &persisted.game else { return };
    let scores: Vec<EndGameScores> = game
        .players
        .iter()
        .map(|player| EndGameScores {
            name: persisted
                .players
                .iter()
                .find(|(id, _)| *id == player.id)
                .map(|(_, n)| n.clone())
                .unwrap_or_default(),
            score: player.score(),
            hand: player.hand.iter().map(|c| c.to_string()).collect(),
        })
        .collect();
    let winner_name = game.winner().and_then(|w| {
        persisted
            .players
            .iter()
            .find(|(id, _)| *id == w.id)
            .map(|(_, n)| n.clone())
    });
    let msg = EndGameMessage {
        status: "success".to_string(),
        message_type: MessageType::EndGame,
        data: EndGameData { winner_name, players: scores },
    };
    broadcast_text(state, game_id, &msg);
}

fn broadcast_game_message(
    state: &AppState,
    game_id: &str,
    persisted: &PersistedGameState,
    game_event: GameEvent,
) {
    let Some(game) = &persisted.game else { return };
    let Some(game_senders) = state.senders.get(game_id) else { return };

    for sender_entry in game_senders.iter() {
        let id = *sender_entry.key();
        let Some(pos) = game.player_pos(&id) else { continue };
        let mut players = vec![];
        for (i, p) in game.players.iter().enumerate() {
            let name = persisted
                .players
                .iter()
                .find(|(pid, _)| *pid == p.id)
                .map(|(_, n)| n.clone())
                .unwrap_or_default();
            players.push(PlayerData {
                name,
                hand: if p.id == id {
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
                player_id: id,
                player_pos: pos as u8,
                num_of_players: persisted.players.len() as u8,
                card_left: game.card_left(),
                current_turn: game.current_turn as u8,
                current_phase: game.phase.clone(),
                event: game_event.clone(),
                players,
            }),
        };
        let Ok(text) = serde_json::to_string(&msg) else { continue };
        let (_, tx) = sender_entry.value();
        if let Err(e) = tx.send(Message::Text(text.into())) {
            tracing::warn!("Failed to send game message to {id}: {e}");
        }
    }
}

fn send_game_message_to_player(
    persisted: &PersistedGameState,
    player_id: Uuid,
    tx: &tokio::sync::mpsc::UnboundedSender<Message>,
    game_event: GameEvent,
) {
    let Some(game) = &persisted.game else { return };
    let Some(pos) = game.player_pos(&player_id) else { return };

    let mut players = vec![];
    for (i, p) in game.players.iter().enumerate() {
        let name = persisted
            .players
            .iter()
            .find(|(pid, _)| *pid == p.id)
            .map(|(_, n)| n.clone())
            .unwrap_or_default();
        players.push(PlayerData {
            name,
            hand: if p.id == player_id {
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
            player_id,
            player_pos: pos as u8,
            num_of_players: persisted.players.len() as u8,
            card_left: game.card_left(),
            current_turn: game.current_turn as u8,
            current_phase: game.phase.clone(),
            event: game_event,
            players,
        }),
    };

    let Ok(text) = serde_json::to_string(&msg) else { return };
    if let Err(e) = tx.send(Message::Text(text.into())) {
        tracing::warn!("Failed to send game state to reconnecting player {player_id}: {e}");
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
