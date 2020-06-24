use super::EnemyOp;
use crate::context::Context;
use anyhow::Result;
use log::*;
use toio::{Cube, Position, SoundPresetId};

pub struct Easy {
    motion: (isize, isize),
}

impl Easy {
    pub fn new() -> Self {
        Self { motion: (0, 0) }
    }
}

#[async_trait::async_trait]
impl EnemyOp for Easy {
    async fn init(&mut self, _: usize, cube: &mut Cube, _: &Context) -> Result<()> {
        cube.light_on(0, 0, 255, None).await?;
        Ok(())
    }

    async fn update(
        &mut self,
        id: usize,
        cube: &mut Cube,
        ctx: &Context,
        player_pos: &Position,
        enemy_pos: &Position,
    ) -> Result<()> {
        let pi = std::f32::consts::PI;
        let x0 = enemy_pos.x as f32;
        let y0 = enemy_pos.y as f32;
        let x1 = player_pos.x as f32;
        let y1 = player_pos.y as f32;

        let dx = (x1 - x0).abs().powf(2.0);
        let dy = (y1 - y0).abs().powf(2.0);
        let d = dx + dy;

        if d < 1000.0 {
            if let Some(rem) = ctx.damage(10).await {
                cube.play_preset(SoundPresetId::Enter).await?;
                info!(
                    "Player was caught by enemy {}: damange={}, remain={}",
                    id, 10, rem
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

        let r0 = enemy_pos.angle as isize;
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
            cube.go(l, r, None).await?;
            self.motion = (l, r);
        }

        Ok(())
    }
}
