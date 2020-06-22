use anyhow::{anyhow, Result};
use gilrs::{Axis, Button, Event, EventType, Gilrs};
use log::*;
use toio::Cube;

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

impl ColorState {
    fn none(&self) -> bool {
        self.r == 0 && self.g == 0 && self.b == 0
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::from_env(
        env_logger::Env::default().default_filter_or(format!("{}=info", module_path!())),
    )
    .init();

    let mut cube = Cube::search().nearest().await?;

    cube.connect().await?;

    info!("Cube connected");

    let mut gilrs = Gilrs::new().map_err(|e| anyhow!("Gilrs error: {}", e))?;
    let mut state = ButtonState::default();
    let mut ds = State::default();

    info!("Polling events...");

    loop {
        while let Some(Event { id, event, time }) = gilrs.next_event() {
            debug!("{:?} New event from {}: {:?}", time, id, event);

            state.update(&event);

            let mut s = State::default();

            if state.north {
                s.col.g |= 0x80;
            }
            if state.south {
                s.col.b |= 0x80;
            }
            if state.east {
                s.col.r |= 0x80;
            }
            if state.west {
                s.col.r |= 0x80;
                s.col.g |= 0x40;
                s.col.b |= 0x40;
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
                if s.col.none() {
                    cube.light_off().await?;
                } else {
                    cube.light_on(s.col.r, s.col.g, s.col.b, None).await?;
                }
                ds.col = s.col;
            }

            if ds.speed != s.speed {
                cube.go(s.speed.l, s.speed.r, None).await?;
                ds.speed = s.speed;
            }
        }
    }
}
