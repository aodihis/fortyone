use crate::context::game_state::GameState;
use std::rc::Rc;
use yew::{function_component, html, use_context, Callback, Html};

#[function_component]
pub fn Alert() -> Html {
    let game_state: Rc<GameState> = use_context::<Rc<GameState>>().unwrap();

    let Some(msg) = game_state.alert.clone() else {
        return html! {};
    };

    let dismiss = game_state.dismiss_alert.clone();
    let on_ok = Callback::from(move |_| dismiss.emit(()));

    html! {
        <div class="overlay alert-overlay">
            <div class="alert-box">
                <p class="alert-message">{ msg }</p>
                <button onclick={on_ok}>{"OK"}</button>
            </div>
        </div>
    }
}
