use crate::context::game_state::{GameState, GameStatus};
use crate::utils::card_class;
use std::rc::Rc;
use yew::{classes, html, Callback, Component, Context, ContextHandle, Html, Properties};

#[derive(Clone, PartialEq)]
pub enum EnemyPos {
    Left,
    Right,
    Top
}


pub enum Msg {
    StateChanged(Rc<GameState>),
}

#[derive(Clone, PartialEq, Properties)]
pub struct EnemyProps {
    pub index: u8,
    pub pos: EnemyPos,
    pub on_bin_click : Callback<usize>,
}
pub struct Enemy {
    index: u8,
    pos: EnemyPos,
    name: String,
    total_cards: u8,
    bin: Vec<String>,
    is_turn: bool,
    _listener: ContextHandle<Rc<GameState>>
}

impl Component for Enemy {
    type Message = Msg;
    type Properties = EnemyProps;

    fn create(ctx: &Context<Self>) -> Self {
        let (state, _listener) = ctx.link()
            .context::<Rc< GameState>>(ctx.link().callback(Msg::StateChanged))
            .expect("context to be set");
        let index = ctx.props().index;
        let pos = ctx.props().pos.clone();
        Self{
            index,
            pos,
            name: state.players[index as usize].name.clone(),
            total_cards: state.players[index as usize].hand.len() as u8,
            bin: state.players[index as usize].bin.clone(),
            is_turn: state.player_index == index as usize,
            _listener,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::StateChanged(state) => {
                if state.game_status != GameStatus::InProgress {
                    return false;
                }
                let index = self.index;
                self.total_cards = state.players[index as usize].hand.len() as u8;
                self.bin = state.players[index as usize].bin.clone();
                self.is_turn = state.player_index == index as usize;
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let class = match self.pos {
            EnemyPos::Left => {"left-enemy"}
            EnemyPos::Right => {"right-enemy"}
            EnemyPos::Top => {"top-enemy"}
        };

        let items = (0..self.total_cards).collect::<Vec<_>>();
        let last_five_bin: Vec<_> = self.bin.iter().rev().take(5).clone().collect();
        let bin_click = {
            let index = self.index;
            ctx.props().on_bin_click.reform(move |_| index as usize)
        };
        html! {
             <div class={class}>
               <div class="discard-pile bottom-discard" onclick={bin_click}>
                    {
                            last_five_bin.iter().rev().map(|x| {
                                let card_class = card_class(x);
                                html! {
                                    <div class={classes!("discard-card", card_class)} ></div>
                                }
                            }).collect::<Html>()
                    }
                </div>
                <div class="player-area">
                    {
                        items.iter().map(|_| html!{<div class="card card-back"></div>}).collect::<Html>()
                    }
                </div>
                <div class="player-name"><span>{self.name.clone()}</span>
                    {
                        if self.is_turn {
                            html!{<span class="loader"></span>}
                        } else {
                            html!{}
                        }
                    }
                </div>
            </div>
        }

    }
}