use crate::context::game_state::GameState;
use gloo_timers::future::TimeoutFuture;
use std::rc::Rc;
use web_sys::wasm_bindgen::JsCast;
use web_sys::window;
use yew::platform::spawn_local;
use yew::{function_component, html, use_context, use_effect_with, use_state, Callback, Html, Properties, SubmitEvent};

#[derive(Clone, PartialEq)]
enum PreGamePhase {
    Home,
    Waiting,
    Join,
    Create,
}

#[derive(Clone, PartialEq)]
enum Action {
    Join,
    Create,
}

#[derive(Properties, PartialEq, Clone)]
pub struct PreGameProps {}

#[function_component]
pub fn PreGame(_: &PreGameProps) -> Html {
    let game_state: Rc<GameState> = use_context::<Rc<GameState>>().unwrap();
    let phase = use_state(|| PreGamePhase::Home);

    {
        let phase = phase.clone();
        use_effect_with(game_state.game_id.clone(), move |game_id| {
            if game_id.is_some() {
                phase.set(PreGamePhase::Waiting);
            } else if game_id.is_none() && *phase == PreGamePhase::Waiting {
                phase.set(PreGamePhase::Home);
            }
        })
    }

    let onclick = {
        let phase = phase.clone();
        move |action: Action| {
            Callback::from(move |_| {
                if action == Action::Join {
                    phase.set(PreGamePhase::Join);
                } else {
                    phase.set(PreGamePhase::Create);
                }
            })
        }
    };

    let join_onclick = { let onclick = onclick.clone(); onclick(Action::Join) };
    let create_onclick = onclick(Action::Create);

    let create_callback = {
        let state = game_state.clone();
        Callback::from(move |name| {
            state.create_game.emit(name);
        })
    };

    let join_callback = {
        let state = game_state.clone();
        Callback::from(move |(game_id, name)| {
            state.join.emit((game_id, name));
        })
    };

    let back_callback = {
        let phase = phase.clone();
        Callback::from(move |_| {
            phase.set(PreGamePhase::Home);
        })
    };

    html! {
        <div class="pre-game">
            {
                if *phase == PreGamePhase::Home {
                    html! {
                        <div class="options">
                            <button onclick={create_onclick}>{"Create Game"}</button>
                            <button onclick={join_onclick}>{"Join Game"}</button>
                        </div>
                    }
                } else if *phase == PreGamePhase::Waiting {
                    html!(<WaitingGame/>)
                } else if *phase == PreGamePhase::Create {
                    html!(<CreateGame callback={create_callback} back={back_callback}/>)
                } else {
                    html!(<JoinGame callback={join_callback}/>)
                }
            }
        </div>
    }
}

fn validate_name(name: &str) -> Result<String, &'static str> {
    let trimmed = name.trim().to_string();
    if trimmed.is_empty() {
        return Err("Name cannot be empty.");
    }
    if trimmed.len() > 32 {
        return Err("Name must be 32 characters or less.");
    }
    Ok(trimmed)
}

fn validate_game_id(game_id: &str) -> Result<(), &'static str> {
    if game_id.is_empty() || game_id.len() > 12 {
        return Err("Invalid game ID.");
    }
    if !game_id.chars().all(|c| c.is_alphanumeric()) {
        return Err("Invalid game ID.");
    }
    Ok(())
}

#[derive(Properties, PartialEq, Clone)]
pub struct JoinGameProps {
    callback: Callback<(String, String)>,
}

#[function_component]
pub fn JoinGame(props: &JoinGameProps) -> Html {
    let callback = props.callback.clone();
    let error = use_state(|| Option::<String>::None);

    let onsubmit = {
        let error = error.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let target = e.target();
            let form = target
                .and_then(|t| t.dyn_into::<web_sys::HtmlFormElement>().ok())
                .expect("Couldn't get HtmlFormElement");

            let name_element = form
                .get_with_name("name")
                .and_then(|n| n.dyn_into::<web_sys::HtmlInputElement>().ok())
                .unwrap();
            let game_id_element = form
                .get_with_name("game-id")
                .and_then(|n| n.dyn_into::<web_sys::HtmlInputElement>().ok())
                .unwrap();

            let name = match validate_name(&name_element.value()) {
                Ok(n) => n,
                Err(msg) => { error.set(Some(msg.to_string())); return; }
            };

            let game_id = game_id_element.value();
            if let Err(msg) = validate_game_id(&game_id) {
                error.set(Some(msg.to_string()));
                return;
            }

            error.set(None);
            callback.emit((game_id, name));
        })
    };

    html! {
        <div class="game-form">
            <form onsubmit={onsubmit}>
                <input name="game-id" type="text" maxlength="12" placeholder="Game ID"/>
                <input name="name" type="text" maxlength="32" placeholder="Your name"/>
                if let Some(err) = (*error).clone() {
                    <p class="form-error">{ err }</p>
                }
                <button type="submit">{"Join Game"}</button>
            </form>
        </div>
    }
}

#[derive(Properties, PartialEq, Clone)]
pub struct CreateGameProps {
    callback: Callback<String>,
    back: Callback<()>,
}

#[function_component]
pub fn CreateGame(props: &CreateGameProps) -> Html {
    let back_cb = props.back.clone();
    let callback = props.callback.clone();
    let error = use_state(|| Option::<String>::None);

    let onsubmit = {
        let error = error.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let target = e.target();
            let form = target
                .and_then(|t| t.dyn_into::<web_sys::HtmlFormElement>().ok())
                .expect("Couldn't get HtmlFormElement");
            let name_element = form
                .get_with_name("name")
                .and_then(|n| n.dyn_into::<web_sys::HtmlInputElement>().ok())
                .unwrap();

            let name = match validate_name(&name_element.value()) {
                Ok(n) => n,
                Err(msg) => { error.set(Some(msg.to_string())); return; }
            };

            error.set(None);
            callback.emit(name);
        })
    };
    let onback = Callback::from(move |_| back_cb.emit(()));

    html! {
        <div class="game-form">
            <form onsubmit={onsubmit}>
                <input name="name" type="text" maxlength="32" placeholder="Your name"/>
                if let Some(err) = (*error).clone() {
                    <p class="form-error">{ err }</p>
                }
                <button type="submit">{"Create Game"}</button>
            </form>
            <button onclick={onback}>{"Back"}</button>
        </div>
    }
}

#[function_component]
pub fn WaitingGame() -> Html {
    let game_state: Rc<GameState> = use_context::<Rc<GameState>>().unwrap();
    let copy_button_label = use_state(|| "📋");
    let game_id = match &game_state.game_id {
        Some(game_id) => game_id.clone(),
        _ => return html!(),
    };

    let players = game_state
        .players
        .iter()
        .map(|player| html!(<div class="wg-player">{player.name.clone()}</div>))
        .collect::<Html>();

    let copy_id = {
        let game_id = game_id.clone();
        let copy_button_label = copy_button_label.clone();
        Callback::from(move |_| {
            if let Some(w) = window() {
                let _ = w.navigator().clipboard().write_text(&game_id);
                copy_button_label.set("✅");
                {
                    let copy_button_label = copy_button_label.clone();
                    spawn_local(async move {
                        TimeoutFuture::new(1000).await;
                        copy_button_label.set("📋");
                    });
                }
            }
        })
    };

    let leave_onclick = {
        let disconnect = game_state.disconnect.clone();
        Callback::from(move |_| disconnect.emit(()))
    };

    let start_onclick = {
        let start_game = game_state.start_game.clone();
        Callback::from(move |_| start_game.emit(()))
    };

    html! {
        <>
            <div class="waiting-game">
                <div class="box">
                    <h3>{"Waiting for players"}</h3>
                    <div class="wg-players">
                        {players}
                    </div>
                </div>
                <div class="box">
                    <div class="game-id-container">
                        <span class="game-id" id="game-id">{game_id}</span>
                        <button class="copy-id" title="Copy" onclick={copy_id}>
                            { *copy_button_label }
                        </button>
                    </div>
                    {
                        if game_state.players.len() > 1 {
                            html! {<button onclick={start_onclick}>{"Start Game"}</button>}
                        } else {
                            html!{}
                        }
                    }
                    <button onclick={leave_onclick}>{"Leave"}</button>
                </div>
            </div>
        </>
    }
}
