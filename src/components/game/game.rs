use std::iter::Iterator;
use leptos::{component, view, IntoView};
use leptos::prelude::{signal, ClassAttribute, ElementChild, Get, ReadSignal, StyleAttribute};

enum EnemyPos {
    Left,
    Right,
    Top
}
#[component]
pub fn Game() -> impl IntoView {
    let (deck, set_deck) = signal(52);
    view! {
        <div class="game-container">
            <CardDistribute/>
            <Deck total={deck}/>
            <CurrentPlayer/>
            <Enemy position={EnemyPos::Left}/>
            <Enemy position={EnemyPos::Right}/>
            <Enemy position={EnemyPos::Top}/>
        </div>
    }
}

#[component]
fn CardDistribute() -> impl IntoView {
    view! {
        <>
            <div class="starting-card card card-back" style="animation: throw-card-bottom 1s ease-in-out forwards;"></div>
            <div class="starting-card card card-back" style="animation: throw-card-right 1s ease-in-out 1s forwards;"></div>
            <div class="starting-card card card-back" style="animation: throw-card-top 1s ease-in-out 2s forwards;"></div>
            <div class="starting-card card card-back" style="animation: throw-card-left 1s ease-in-out 3s forwards;"></div>
            <div class="starting-card card card-back" style="animation: throw-card-bottom 1s ease-in-out 4s forwards;"></div>
            <div class="starting-card card card-back" style="animation: throw-card-right 1s ease-in-out 5s forwards;"></div>
            <div class="starting-card card card-back" style="animation: throw-card-top 1s ease-in-out 6s forwards;"></div>
            <div class="starting-card card card-back" style="animation: throw-card-left 1s ease-in-out 7s forwards;"></div>
            <div class="starting-card card card-back" style="animation: throw-card-bottom 1s ease-in-out 8s forwards;"></div>
            <div class="starting-card card card-back" style="animation: throw-card-right 1s ease-in-out 9s forwards;"></div>
            <div class="starting-card card card-back" style="animation: throw-card-top 1s ease-in-out 10s forwards;"></div>
            <div class="starting-card card card-back" style="animation: throw-card-left 1s ease-in-out 11s forwards;"></div>
            <div class="starting-card card card-back" style="animation: throw-card-bottom 1s ease-in-out 12s forwards;"></div>
            <div class="starting-card card card-back" style="animation: throw-card-right 1s ease-in-out 13s forwards;"></div>
            <div class="starting-card card card-back" style="animation: throw-card-top 1s ease-in-out 14s forwards;"></div>
            <div class="starting-card card card-back" style="animation: throw-card-left 1s ease-in-out 15s forwards;"></div>

        </>

    }
}
#[component]
fn Deck(total: ReadSignal<u8>) -> impl IntoView {
    view! {
        <div class="deck">
            {(0..total.get()).into_iter()
                .map(|n:u8| {
                let tr = n/4;
                let mut class = "card card-back";
                if n == 51 {
                    class = "card card-back card-throw";
                }
                view! {<div class={class} style=format!("transform: translate(-{tr}px, -{tr}px);")></div>}
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

