use crate::context::game_state::{GameState, GameStatus, PlayerPhase};
use crate::utils::card_class;
use std::rc::Rc;
use web_sys::MouseEvent;
use yew::{classes, html, Callback, Component, Context, ContextHandle, Html, Properties};

#[derive(Clone, PartialEq, Properties)]
pub struct CurrentPlayerProps {
    pub on_bin_click: Callback<usize>,
}

pub enum Msg {
    StateChanged(Rc<GameState>),
    HoverCard(Option<String>),
}

pub struct ThePlayer {
    index: usize,
    is_turn: bool,
    player_phase: PlayerPhase,
    name: String,
    hand: Vec<String>,
    bin: Vec<String>,
    take_bin_cb: Callback<()>,
    discard_cb: Callback<String>,
    close_cb: Callback<String>,
    hovered_card: Option<String>,
    _listener: ContextHandle<Rc<GameState>>,
}

fn card_points(card: &str) -> i32 {
    if card.len() < 2 {
        return 0;
    }
    match &card[1..] {
        "A" => 11,
        "2" => 2,
        "3" => 3,
        "4" => 4,
        "5" => 5,
        "6" => 6,
        "7" => 7,
        "8" => 8,
        "9" => 9,
        _ => 10, // X, J, Q, K
    }
}

fn compute_score(hand: &[String]) -> i32 {
    let mut suit_totals = [0i32; 4]; // H=0, D=1, S=2, C=3
    let mut total = 0i32;
    for card in hand {
        if card.len() < 2 {
            continue;
        }
        let suit_idx = match &card[..1] {
            "H" => 0,
            "D" => 1,
            "S" => 2,
            "C" => 3,
            _ => continue,
        };
        let pts = card_points(card);
        suit_totals[suit_idx] += pts;
        total += pts;
    }
    let max_suit = suit_totals.iter().copied().max().unwrap_or(0);
    max_suit * 2 - total
}

fn can_close_with_discard(hand: &[String], discard: &str) -> bool {
    let mut removed = false;
    let remaining: Vec<String> = hand
        .iter()
        .filter(|c| {
            if !removed && c.as_str() == discard {
                removed = true;
                false
            } else {
                true
            }
        })
        .cloned()
        .collect();
    compute_score(&remaining) >= 38
}

impl Component for ThePlayer {
    type Message = Msg;
    type Properties = CurrentPlayerProps;

    fn create(ctx: &Context<Self>) -> Self {
        let (state, _listener) = ctx
            .link()
            .context::<Rc<GameState>>(ctx.link().callback(Msg::StateChanged))
            .expect("context to be set");
        let is_turn = state.current_turn_index == state.player_index;
        Self {
            index: state.player_index,
            is_turn,
            player_phase: {
                if is_turn {
                    state.current_turn_phase.clone()
                } else {
                    PlayerPhase::Waiting
                }
            },
            name: state.player_name.clone(),
            hand: state.players[state.player_index].hand.clone(),
            bin: state.players[state.player_index].bin.clone(),
            take_bin_cb: state.take_bin.clone(),
            discard_cb: state.discard.clone(),
            close_cb: state.close.clone(),
            hovered_card: None,
            _listener,
        }
    }

    fn update(&mut self, _: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::StateChanged(state) => {
                if state.game_status != GameStatus::InProgress {
                    return false;
                }
                let is_turn = state.current_turn_index == state.player_index;
                self.is_turn = is_turn;
                self.player_phase = {
                    if is_turn {
                        state.current_turn_phase.clone()
                    } else {
                        PlayerPhase::Waiting
                    }
                };
                self.hand = state.players[state.player_index].hand.clone();
                self.bin = state.players[state.player_index].bin.clone();
                self.hovered_card = None;
                true
            }
            Msg::HoverCard(card) => {
                self.hovered_card = card;
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let last_five_bin: Vec<_> = self.bin.iter().rev().take(5).clone().collect();
        let bin_click = {
            let index = self.index;
            ctx.props().on_bin_click.reform(move |_| index)
        };

        let is_draw_phase = self.is_turn && self.player_phase == PlayerPhase::P1;
        let is_discard_phase = self.is_turn && self.player_phase == PlayerPhase::P2;
        let take_bin_cb = self.take_bin_cb.clone();
        let discard_cb = self.discard_cb.clone();
        let close_cb = self.close_cb.clone();

        let on_take_bin = {
            let take_bin_cb = take_bin_cb.clone();
            Callback::from(move |e: MouseEvent| {
                e.prevent_default();
                take_bin_cb.emit(());
            })
        };

        html! {
            <>
                <div class="current-player">
                    <div class="discard-pile bottom-discard" onclick={bin_click}>
                        {
                            last_five_bin.iter().rev().map(|x| {
                                let card_class = card_class(x);
                                html! {
                                    <div key={(*x).clone()} class={classes!("discard-card", card_class)}></div>
                                }
                            }).collect::<Html>()
                        }
                    </div>
                    {
                        if self.player_phase != PlayerPhase::GameEnded {
                            html! {
                                <div class="game-info">
                                    {
                                        if !self.is_turn {
                                            html! { "Waiting for the other player's turn!" }
                                        } else if self.player_phase == PlayerPhase::P1 {
                                            html! { <p>{ "Your turn — draw from the deck or take the bin." }</p> }
                                        } else {
                                            html! { <p>{ "Discard a card from your hand." }</p> }
                                        }
                                    }
                                </div>
                            }
                        } else {
                            html! {}
                        }
                    }
                    {
                        if is_draw_phase && !self.bin.is_empty() {
                            html! {
                                <div class="player-actions">
                                    <button class="take-bin-btn" onclick={on_take_bin}>{"Take Bin"}</button>
                                </div>
                            }
                        } else {
                            html! {}
                        }
                    }
                    <div class="player-area">
                        {
                            self.hand.iter().map(|h| {
                                let card_class = card_class(h);
                                let is_hovered = self.hovered_card.as_deref() == Some(h.as_str());
                                let closeable = is_discard_phase && can_close_with_discard(&self.hand, h);

                                let onmouseenter = {
                                    let h = h.clone();
                                    ctx.link().callback(move |_: MouseEvent| Msg::HoverCard(Some(h.clone())))
                                };
                                let onmouseleave = ctx.link().callback(|_: MouseEvent| Msg::HoverCard(None));

                                let discard_onclick = {
                                    let discard_cb = discard_cb.clone();
                                    let h = h.clone();
                                    Callback::from(move |e: MouseEvent| {
                                        e.stop_propagation();
                                        if is_discard_phase {
                                            discard_cb.emit(h.clone());
                                        }
                                    })
                                };

                                let close_onclick = {
                                    let close_cb = close_cb.clone();
                                    let h = h.clone();
                                    Callback::from(move |e: MouseEvent| {
                                        e.stop_propagation();
                                        if is_discard_phase {
                                            close_cb.emit(h.clone());
                                        }
                                    })
                                };

                                html! {
                                    <div
                                        key={h.clone()}
                                        class={classes!("card-wrapper", closeable.then_some("closeable"))}
                                        onmouseenter={onmouseenter}
                                        onmouseleave={onmouseleave}
                                    >
                                        <div class={classes!("card", card_class)}></div>
                                        {
                                            if is_discard_phase && is_hovered {
                                                html! {
                                                    <div class="card-actions">
                                                        if closeable {
                                                            <button class="card-action-btn close-btn" onclick={close_onclick}>{"Close"}</button>
                                                        }
                                                        <button class="card-action-btn discard-btn" onclick={discard_onclick}>{"Discard"}</button>
                                                    </div>
                                                }
                                            } else {
                                                html! {}
                                            }
                                        }
                                    </div>
                                }
                            }).collect::<Html>()
                        }
                    </div>
                    <div class="player-name">
                        <span>{self.name.clone()}</span>
                    </div>
                </div>
            </>
        }
    }
}
