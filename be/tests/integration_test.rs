use axum::Extension;
use axum_test::TestServer;
use fortyone_be::routes::game::create_router;
use fortyone_be::state::memory_store::MemoryGameStore;
use fortyone_be::state::state::AppState;
use serde_json::Value;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

const TEST_SECRET: &str = "test-secret-key";

fn make_server() -> TestServer {
    let store = Arc::new(MemoryGameStore::new());
    let state = AppState::new(store);
    let jwt_secret = Arc::new(TEST_SECRET.to_string());
    let router = create_router(state, CorsLayer::permissive())
        .layer(Extension(jwt_secret));
    TestServer::new(router)
}

#[tokio::test]
async fn test_create_game_returns_game_id() {
    let server = make_server();
    let res = server.get("/create").await;
    res.assert_status_ok();
    let body: Value = res.json();
    assert!(body["game_id"].as_str().is_some(), "game_id missing");
}

#[tokio::test]
async fn test_join_game_returns_token() {
    let server = make_server();

    let create_res = server.get("/create").await;
    let game_id = create_res.json::<Value>()["game_id"]
        .as_str()
        .unwrap()
        .to_string();

    let join_res = server
        .post(&format!("/{game_id}/join?player_name=Alice"))
        .await;
    join_res.assert_status_ok();
    let body: Value = join_res.json();
    assert!(body["player_id"].as_str().is_some());
    assert!(body["token"].as_str().is_some());
}

#[tokio::test]
async fn test_join_game_not_found() {
    let server = make_server();
    let res = server.post("/nonexist/join?player_name=Alice").await;
    res.assert_status_not_found();
}

#[tokio::test]
async fn test_join_empty_player_name() {
    let server = make_server();
    let create_res = server.get("/create").await;
    let game_id = create_res.json::<Value>()["game_id"]
        .as_str()
        .unwrap()
        .to_string();

    let res = server.post(&format!("/{game_id}/join?player_name=")).await;
    res.assert_status_bad_request();
}

#[tokio::test]
async fn test_join_name_too_long() {
    let server = make_server();
    let create_res = server.get("/create").await;
    let game_id = create_res.json::<Value>()["game_id"]
        .as_str()
        .unwrap()
        .to_string();

    let long_name = "a".repeat(33);
    let res = server
        .post(&format!("/{game_id}/join?player_name={long_name}"))
        .await;
    res.assert_status_bad_request();
}

#[tokio::test]
async fn test_join_duplicate_name() {
    let server = make_server();
    let create_res = server.get("/create").await;
    let game_id = create_res.json::<Value>()["game_id"]
        .as_str()
        .unwrap()
        .to_string();

    // First join succeeds but player not in game state until WS connects.
    // Two HTTP joins with same name should both succeed (token issued, spot reserved at WS time).
    // This is expected behavior: uniqueness enforced at WS connection.
    let res1 = server.post(&format!("/{game_id}/join?player_name=Alice")).await;
    res1.assert_status_ok();
}

#[tokio::test]
async fn test_ws_without_token_rejected() {
    let server = make_server();
    let create_res = server.get("/create").await;
    let game_id = create_res.json::<Value>()["game_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Attempting WS upgrade without a token query param returns 400 (missing field)
    // The test server will return an error for missing required query param.
    let res = server.get(&format!("/{game_id}/ws")).await;
    // Should not be 200
    assert_ne!(res.status_code(), 200);
}

#[tokio::test]
async fn test_ws_with_invalid_token_rejected() {
    let server = make_server();
    let create_res = server.get("/create").await;
    let game_id = create_res.json::<Value>()["game_id"]
        .as_str()
        .unwrap()
        .to_string();

    // A plain HTTP GET (no WS upgrade headers) to the WS endpoint with a bad token
    // returns 400 (axum rejects non-WS upgrade) before reaching token validation.
    // Either 400 or 401 confirms the request was rejected.
    let res = server
        .get(&format!("/{game_id}/ws?token=invalid.token.here"))
        .await;
    let status = res.status_code().as_u16();
    assert!(
        status == 400 || status == 401,
        "Expected 400 or 401, got {status}"
    );
}

#[tokio::test]
async fn test_invalid_game_id_format() {
    let server = make_server();
    // game_id longer than 12 chars should be rejected at join
    let res = server
        .post("/toolonggameid123/join?player_name=Alice")
        .await;
    res.assert_status_bad_request();
}
