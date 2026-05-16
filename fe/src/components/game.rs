use crate::components::in_game::InGame;
use crate::components::post_game::PostGame;
use crate::components::pre_game::PreGame;
use crate::context::game_state::{GameState, GameStatus};
use crate::models::api_data::{GameRequestAction, GameResponse, MessageType, PlayerData, RequestPayload};
use crate::models::players::Player;
use crate::services::connection::{connect_ws, create_game, join_game_http, send_message};
use futures_util::future::{AbortHandle, Abortable};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use std::cell::RefCell;
use std::rc::Rc;
use web_sys::window;
use yew::platform::spawn_local;
use yew::{html, Component, Context, ContextProvider, Html};

pub struct Game {
    state_ref: Rc<GameState>,
    writer: Rc<RefCell<Option<SplitSink<WebSocket, Message>>>>,
    reader_abort: Rc<RefCell<Option<AbortHandle>>>,
}

pub enum Msg {
    CreateGame(String),
    StartGame,
    Disconnect,
    JoinGame(String, String),
    Listener(SplitStream<WebSocket>),
    GameJoined(String),
    GameUpdate(GameResponse),
    Draw,
    TakeBin,
    Discard(String),
    Close(String),
}

fn log_err(msg: &str) {
    web_sys::console::error_1(&msg.into());
}

fn alert_err(msg: &str) {
    if let Some(w) = window() {
        w.alert_with_message(msg).ok();
    }
}

fn send_action(
    writer: Rc<RefCell<Option<SplitSink<WebSocket, Message>>>>,
    action: GameRequestAction,
    card: Option<String>,
) {
    spawn_local(async move {
        let payload = RequestPayload { action, card };
        let Ok(json) = serde_json::to_string(&payload) else { return };
        let mut binding = writer.borrow_mut();
        if let Some(w) = binding.as_mut() {
            let _ = send_message(w, json).await;
        }
    });
}

impl Component for Game {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let create_game = ctx.link().callback(|name| Msg::CreateGame(name));
        let join_game = ctx.link().callback(|(game_id, name)| Msg::JoinGame(game_id, name));
        let disconnect = ctx.link().callback(|_| Msg::Disconnect);
        let start_game = ctx.link().callback(|_| Msg::StartGame);
        let draw = ctx.link().callback(|_| Msg::Draw);
        let take_bin = ctx.link().callback(|_| Msg::TakeBin);
        let discard = ctx.link().callback(|card| Msg::Discard(card));
        let close = ctx.link().callback(|card| Msg::Close(card));
        let game_state = Rc::new(GameState::new(
            create_game, join_game, disconnect, start_game, draw, take_bin, discard, close,
        ));

        Self {
            state_ref: game_state,
            writer: Rc::new(RefCell::new(None)),
            reader_abort: Rc::new(RefCell::new(None)),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        let state_mut = Rc::make_mut(&mut self.state_ref);
        match msg {
            Msg::CreateGame(name) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    let game_id = match create_game().await {
                        Ok(game_id) => game_id,
                        Err(e) => {
                            log_err(&format!("create_game failed: {e}"));
                            alert_err(&format!("Failed to create game: {e}"));
                            return;
                        }
                    };
                    link.send_message(Msg::JoinGame(game_id, name));
                });
                false
            }
            Msg::JoinGame(game_id, name) => {
                let link = ctx.link().clone();
                let wr = self.writer.clone();
                spawn_local(async move {
                    let join_res = match join_game_http(&game_id, &name).await {
                        Ok(r) => r,
                        Err(e) => {
                            log_err(&format!("join_game_http failed: {e}"));
                            alert_err(&format!("Failed to join game: {e}"));
                            return;
                        }
                    };

                    let socket = match connect_ws(&game_id, &join_res.token) {
                        Ok(s) => s,
                        Err(e) => {
                            log_err(&format!("connect_ws failed: {e}"));
                            alert_err(&format!("Failed to open WebSocket: {e}"));
                            return;
                        }
                    };

                    let (writer, reader) = socket.split();
                    *wr.borrow_mut() = Some(writer);
                    link.send_message(Msg::GameJoined(game_id));
                    link.send_message(Msg::Listener(reader));
                });
                false
            }
            Msg::GameJoined(game_id) => {
                Rc::make_mut(&mut self.state_ref).game_id = Some(game_id);
                Rc::make_mut(&mut self.state_ref).counter += 1;
                true
            }
            Msg::Listener(mut reader) => {
                let link = ctx.link().clone();
                let (abort_handle, abort_registration) = AbortHandle::new_pair();
                *self.reader_abort.borrow_mut() = Some(abort_handle);

                let task = async move {
                    while let Some(msg) = reader.next().await {
                        match msg {
                            Ok(Message::Text(message)) => {
                                match serde_json::from_str::<GameResponse>(&message) {
                                    Ok(response) => link.send_message(Msg::GameUpdate(response)),
                                    Err(_) => {}
                                }
                            }
                            Ok(Message::Bytes(_)) => {}
                            Err(_) => {
                                link.send_message(Msg::Disconnect);
                            }
                        }
                    }
                };

                spawn_local(async move {
                    let _ = Abortable::new(task, abort_registration).await;
                });
                false
            }
            Msg::GameUpdate(response) => {
                if response.status != "success" {
                    return false;
                }
                match response.message_type {
                    MessageType::PlayerJoin | MessageType::PlayerLeft => {
                        if state_mut.game_status != GameStatus::PostGame {
                            if let Some(data) = response.data {
                                state_mut.players = data.players.iter().map(|p| Player {
                                    name: p.name.clone(),
                                    bin: vec![],
                                    hand: vec![],
                                    score: 0,
                                }).collect();
                            }
                        }
                    }
                    MessageType::GameEvent => {
                        if let Some(data) = response.data {
                            let player_index = data.player_pos.unwrap_or(0) as usize;
                            state_mut.game_status = GameStatus::InProgress;
                            state_mut.players = data.players.iter().map(|p| Player {
                                name: p.name.clone(),
                                bin: p.bin.clone(),
                                hand: p.hand.clone(),
                                score: 0,
                            }).collect();
                            state_mut.player_index = player_index;
                            if let Some(name) = data.players.get(player_index).map(|p| p.name.clone()) {
                                state_mut.player_name = name;
                            }
                            if let Some(turn) = data.current_turn {
                                state_mut.current_turn_index = turn as usize;
                            }
                            if let Some(phase) = data.current_phase {
                                state_mut.current_turn_phase = phase;
                            }
                        }
                    }
                    MessageType::EndGame => {
                        if let Some(data) = response.data {
                            state_mut.game_status = GameStatus::PostGame;
                            state_mut.winner = data.winner_name.clone();
                            state_mut.players = data.players.iter().map(|p: &PlayerData| Player {
                                name: p.name.clone(),
                                bin: vec![],
                                hand: p.hand.clone(),
                                score: p.score,
                            }).collect();
                        }
                    }
                    _ => {}
                }
                state_mut.counter += 1;
                true
            }
            Msg::Disconnect => {
                state_mut.clear();
                if let Some(handle) = self.reader_abort.borrow_mut().take() {
                    handle.abort();
                }
                let wr = self.writer.borrow_mut().take();
                spawn_local(async move {
                    if let Some(mut writer) = wr {
                        let _ = writer.close().await;
                    }
                });
                true
            }
            Msg::StartGame => {
                send_action(self.writer.clone(), GameRequestAction::StartGame, None);
                true
            }
            Msg::Draw => {
                send_action(self.writer.clone(), GameRequestAction::Draw, None);
                false
            }
            Msg::TakeBin => {
                send_action(self.writer.clone(), GameRequestAction::TakeBin, None);
                false
            }
            Msg::Discard(card) => {
                send_action(self.writer.clone(), GameRequestAction::Discard, Some(card));
                false
            }
            Msg::Close(card) => {
                send_action(self.writer.clone(), GameRequestAction::Close, Some(card));
                false
            }
        }
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        let game_data = self.state_ref.clone();
        html! {
            <div class="game-container">
                <ContextProvider<Rc<GameState>> context={game_data.clone()}>
                    {
                        if game_data.game_status == GameStatus::PreGame {
                            html!{<PreGame/>}
                        } else {
                            html!{<InGame/>}
                        }
                    }
                    {
                        if game_data.game_status == GameStatus::PostGame {
                            html!{<PostGame/>}
                        } else {
                            html!{}
                        }
                    }
                </ContextProvider<Rc<GameState>>>
            </div>
        }
    }
}
