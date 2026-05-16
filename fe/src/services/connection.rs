use crate::errors::game_error::GameError;
use crate::models::api_data::{ConnectResponse, JoinResponse};
use futures_util::SinkExt;
use futures_util::stream::SplitSink;
use gloo_net::http::Request;
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use js_sys::encode_uri_component;

const API_URL: &str = env!("API_URL");

pub async fn create_game() -> Result<String, GameError> {
    let response = Request::get(&format!("{}/create", API_URL))
        .send()
        .await
        .map_err(|e| GameError::CreationFailed(e.to_string()))?;

    let data = response
        .json::<ConnectResponse>()
        .await
        .map_err(|e| GameError::CreationFailed(e.to_string()))?;

    Ok(data.game_id)
}

pub async fn join_game_http(game_id: &str, name: &str) -> Result<JoinResponse, GameError> {
    let encoded_name = encode_uri_component(name);
    let url = format!("{}/{}/join?player_name={}", API_URL, game_id, encoded_name);

    let response = Request::post(&url)
        .send()
        .await
        .map_err(|e| GameError::JoinHttpFailed(e.to_string()))?;

    if !response.ok() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        let body = body.trim();
        let msg = if body.is_empty() {
            format!("HTTP {status}")
        } else {
            format!("HTTP {status}: {body}")
        };
        return Err(GameError::JoinHttpFailed(msg));
    }

    response
        .json::<JoinResponse>()
        .await
        .map_err(|e| GameError::JoinHttpFailed(e.to_string()))
}

pub fn connect_ws(game_id: &str, token: &str) -> Result<WebSocket, GameError> {
    let ws_url = API_URL.replace("http://", "ws://").replace("https://", "wss://");
    let url = format!("{}/{}/ws?token={}", ws_url, game_id, token);
    WebSocket::open(&url).map_err(|e| GameError::JoinFailed(e.to_string()))
}

pub async fn send_message(
    writer: &mut SplitSink<WebSocket, Message>,
    payload: String,
) -> Result<(), GameError> {
    writer
        .send(Message::Text(payload))
        .await
        .map_err(|e| GameError::SendFailed(e.to_string()))
}
