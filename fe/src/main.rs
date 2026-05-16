mod components;
mod context;
pub mod utils;
mod errors;
mod models;
mod services;

use yew::prelude::*;
use crate::components::game::Game;

#[function_component]
fn App() -> Html {

    html! {
        <Game/>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}