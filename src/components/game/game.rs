use std::iter::Iterator;
use leptos::{component, view, IntoView};
use leptos::prelude::{ClassAttribute, ElementChild, ReadSignal, StyleAttribute};

enum EnemyPos {
    Left,
    Right,
    Top
}
#[component]
pub fn Game() -> impl IntoView {
    let deck_total: u8 = 52;
    view! {
        <div class="game-container">
            <Deck total={deck_total}/>
            <CurrentPlayer/>
            <Enemy position={EnemyPos::Left}/>
            <Enemy position={EnemyPos::Right}/>
            <Enemy position={EnemyPos::Top}/>
        </div>
    }
}

#[component]
fn Deck(total: u8) -> impl IntoView {
    view! {
        <div class="deck">
                {(0..total).into_iter()
                    .map(|n:u8| {
                    let tr = n/4;
                    view! {<div class="card card-back" style=format!("transform: translate(-{tr}px, -{tr}px);")></div>}
                })
                    .collect::<Vec<_>>()
                }
            </div>
    }
}
#[component]
pub fn CurrentPlayer() -> impl IntoView {
    view! {
        <div class="player-area current-player">
            <div class="card card-back"></div>
            <div class="card card-back"></div>
            <div class="card card-back"></div>
            <div class="card card-back"></div>
        </div>
    }
}

#[component]
fn Enemy(position:EnemyPos) -> impl IntoView {

    let class = match position {
        EnemyPos::Left => {"left-enemy player-area"}
        EnemyPos::Right => {"right-enemy player-area"}
        EnemyPos::Top => {"top-enemy player-area"}
    };

    view! {
        <div class={class}>
            <div class="card card-back"></div>
            <div class="card card-back"></div>
            <div class="card card-back"></div>
            <div class="card card-back"></div>
        </div>
    }
}

