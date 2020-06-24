use crate::{
    context::{Context, PlayerStatus},
    enemies::{easy::Easy, Enemy},
};
use anyhow::{bail, Result};
use futures::prelude::*;
use gilrs::{Axis, Button, Event, EventType};
use log::*;
use std::time::Duration;
use toio::{Cube, LightOp};

mod context;
mod enemies;
mod gil;

#[derive(Debug, Clone, Default)]
struct ButtonState {
    north: bool,
    south: bool,
    east: bool,
    west: bool,
    x: f32,
    y: f32,
    boost: f32,
}

impl ButtonState {
    fn update(&mut self, event: &EventType) {
        match event {
            EventType::ButtonPressed(button, _) => match button {
                Button::North => {
                    self.north = true;
                }
                Button::South => {
                    self.south = true;
                }
                Button::East => {
                    self.east = true;
                }
                Button::West => {
                    self.west = true;
                }
                Button::RightTrigger2 | Button::LeftTrigger2 => {
                    self.boost += 1.0;
                }
                _ => {}
            },
            EventType::ButtonReleased(button, _) => match button {
                Button::North => {
                    self.north = false;
                }
                Button::South => {
                    self.south = false;
                }
                Button::East => {
                    self.east = false;
                }
                Button::West => {
                    self.west = false;
                }
                Button::RightTrigger2 | Button::LeftTrigger2 => {
                    self.boost -= 1.0;
                }
                _ => {}
            },
            EventType::AxisChanged(axis, val, _) => match axis {
                Axis::LeftStickX => {
                    self.x = *val;
                }
                Axis::LeftStickY => {
                    self.y = *val;
                }
                _ => {}
            },
            _ => {}
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct ColorState {
    r: u8,
    g: u8,
    b: u8,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct SpeedState {
    l: isize,
    r: isize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct State {
    col: ColorState,
    speed: SpeedState,
}

async fn player_loop(mut cube: Cube, ctx: Context) -> Result<()> {
    info!("Starting player...");

    let mut inputs = gil::start()?;
    let mut state = ButtonState::default();
    let mut ds = State::default();

    while let Some(Event { id, event, time }) = inputs.next().await {
        debug!("{:?} New event from {}: {:?}", time, id, event);

        if ctx.gameover().await {
            break;
        }

        match event {
            EventType::ButtonPressed(button, _) => match button {
                Button::Start => {
                    ctx.pause().await;
                }
                _ => {}
            },
            _ => {}
        }

        state.update(&event);

        let mut s = State::default();

        match ctx.status().await {
            PlayerStatus::Fine => {
                s.col.g = 255;
            }
            PlayerStatus::Caution => {
                s.col.r = 255;
                s.col.g = 255;
            }
            PlayerStatus::Danger => {
                s.col.r = 255;
            }
            _ => {}
        }

        if !ctx.paused().await {
            let w = state.boost;
            if state.x.abs() / 2.0 > state.y.abs() {
                let x = state.x * (10.0 + w * 10.0);
                s.speed.r = (-1.0 * x) as isize;
                s.speed.l = (1.0 * x) as isize;
            } else {
                let x = state.x * (80.0 - (20.0 * w));
                let y = state.y * ((40.0 * w) + 20.0);
                s.speed.r = (y * (100.0 - x.max(0.0).abs()) / 100.0) as isize;
                s.speed.l = (y * (100.0 - x.min(0.0).abs()) / 100.0) as isize;
            }
        }

        if ds.col != s.col {
            cube.light(
                0,
                vec![
                    LightOp::new(s.col.r, s.col.g, s.col.b, Some(Duration::from_millis(500))),
                    LightOp::new(0, 0, 0, Some(Duration::from_millis(500))),
                ],
            )
            .await?;

            ds.col = s.col;
        }

        if ds.speed != s.speed {
            cube.go(s.speed.l, s.speed.r, None).await?;
            ds.speed = s.speed;
        }
    }

    info!("Player stopped");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::from_env(
        env_logger::Env::default().default_filter_or(format!("{}=info", module_path!())),
    )
    .init();

    let cubes = Cube::search().all().await?;

    let mut iter = cubes.into_iter();
    let mut player = match iter.next() {
        Some(c) => c,
        None => bail!("No player cube"),
    };
    let mut enemies: Vec<_> = iter.collect();
    if enemies.is_empty() {
        bail!("No enemies");
    }

    player.connect().await?;
    info!("Player cube connected");
    for (i, enemy) in enemies.iter_mut().enumerate() {
        enemy.connect().await?;
        info!("Enemy cube {} connected", i);
    }

    let ctx = Context::new();

    let mut enemies: Vec<_> = enemies
        .into_iter()
        .enumerate()
        .map(|(i, e)| Enemy::new(i, e, ctx.clone(), Easy::new()))
        .collect();

    let mut events = player.events().await?;

    // Initialize enemies
    for enemy in &mut enemies {
        enemy.init().await?;
    }

    let ctxp = ctx.clone();
    tokio::spawn(async move { player_loop(player, ctxp).await });

    // Start event loop
    while let Some(e) = events.next().await {
        if ctx.gameover().await {
            info!("Player is dead. Game over...");
            break;
        }

        match e {
            toio::Event::Position(pos) => {
                for en in &mut enemies {
                    if let Some(pos) = pos.as_ref() {
                        en.update(pos).await?;
                    }
                }
            }
            _ => {}
        }
    }

    Ok(())
}
