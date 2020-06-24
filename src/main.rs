use anyhow::{bail, Result};
use futures::prelude::*;
use gilrs::{Axis, Button, Event, EventType};
use log::*;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use toio::{Cube, LightOp, Position, SoundPresetId};
use tokio::sync::Mutex;

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

enum PlayerStatus {
    Fine,
    Caution,
    Danger,
    Death,
}

#[derive(Debug)]
struct GameContext {
    health: usize,
    hit: Option<Instant>,
}

impl GameContext {
    fn new() -> Self {
        Self {
            health: 100,
            hit: None,
        }
    }

    fn damage(&mut self, damage: usize) -> Option<usize> {
        if self.hit.is_none() {
            self.hit = Some(Instant::now());
            self.health = self.health.saturating_sub(damage);
            Some(self.health)
        } else {
            None
        }
    }

    fn status(&mut self) -> PlayerStatus {
        if let Some(hit) = self.hit.as_ref() {
            if hit.elapsed() > Duration::from_secs(2) {
                self.hit = None;
            }
        }

        match self.health {
            0 => PlayerStatus::Death,
            1..=29 => PlayerStatus::Danger,
            30..=59 => PlayerStatus::Caution,
            _ => PlayerStatus::Fine,
        }
    }

    fn gameover(&self) -> bool {
        self.health == 0
    }
}

type Context = Arc<Mutex<GameContext>>;

async fn player_loop(mut cube: Cube, ctx: Context) -> Result<()> {
    info!("Starting player...");

    let mut inputs = gil::start()?;
    let mut state = ButtonState::default();
    let mut ds = State::default();

    while let Some(Event { id, event, time }) = inputs.next().await {
        debug!("{:?} New event from {}: {:?}", time, id, event);

        state.update(&event);

        let mut s = State::default();

        match ctx.lock().await.status() {
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

#[derive(Debug)]
struct Enemy {
    id: usize,
    cube: Cube,
    inst: Instant,
    motion: (isize, isize),
    ctx: Context,
}

impl Enemy {
    fn new(id: usize, cube: Cube, ctx: Context) -> Self {
        Self {
            id,
            cube,
            inst: Instant::now(),
            motion: (0, 0),
            ctx,
        }
    }

    async fn init(&mut self) -> Result<()> {
        self.cube.light_on(0, 0, 255, None).await?;
        Ok(())
    }

    async fn update(&mut self, pos: &Position) -> Result<()> {
        if self.inst.elapsed() < Duration::from_millis(200) {
            return Ok(());
        }
        self.inst = Instant::now();

        let p = match self.cube.position().await? {
            Some(p) => p,
            None => {
                warn!("Enemy is out of field");
                return Ok(());
            }
        };

        let pi = std::f32::consts::PI;
        let x0 = p.x as f32;
        let y0 = p.y as f32;
        let x1 = pos.x as f32;
        let y1 = pos.y as f32;

        let dx = (x1 - x0).abs().powf(2.0);
        let dy = (y1 - y0).abs().powf(2.0);
        let d = dx + dy;

        if d < 1000.0 {
            if let Some(rem) = self.ctx.lock().await.damage(10) {
                self.cube.play_preset(SoundPresetId::Enter).await?;
                info!(
                    "Player was caught by enemy {}: damange={}, remain={}",
                    self.id, 10, rem
                );
            }
        }

        let xd = (x1 - x0).abs();
        let yd = (y1 - y0).abs();
        let r = match (x1 > x0, y1 > y0) {
            (true, true) => (yd / xd).atan(),
            (false, true) => (xd / yd).atan() + pi / 2.0,
            (false, false) => (yd / xd).atan() + pi,
            (true, false) => (xd / yd).atan() + pi * 3.0 / 2.0,
        };

        let r0 = p.angle as isize;
        let r1 = (r * (180.0 / pi)) as isize;

        let right = if r0 < 180 {
            r0 < r1 && r1 < r0 + 180
        } else {
            r0 < r1 || r1 < (r0 + 180) % 360
        };

        let (l, r) = if (r0 - r1).abs() > 10 {
            if right {
                (2, -2)
            } else {
                (-2, 2)
            }
        } else {
            (6, 6)
        };
        if (l, r) != self.motion {
            self.cube.go(l, r, None).await?;
            self.motion = (l, r);
        }

        Ok(())
    }
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

    let ctx = Arc::new(Mutex::new(GameContext::new()));

    let mut enemies: Vec<_> = enemies
        .into_iter()
        .enumerate()
        .map(|(i, e)| Enemy::new(i, e, ctx.clone()))
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
        if ctx.lock().await.gameover() {
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
