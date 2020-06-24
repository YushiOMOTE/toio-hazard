use crate::context::Context;
use anyhow::Result;
use log::*;
use std::time::{Duration, Instant};
use toio::{Cube, Position};

pub mod easy;

#[async_trait::async_trait]
pub trait EnemyOp {
    async fn init(&mut self, id: usize, cube: &mut Cube, ctx: &Context) -> Result<()>;

    async fn update(
        &mut self,
        id: usize,
        cube: &mut Cube,
        ctx: &Context,
        player_pos: &Position,
        enemy_pos: &Position,
    ) -> Result<()>;
}

#[derive(Debug)]
pub struct Enemy<T> {
    id: usize,
    cube: Option<Cube>,
    ctx: Context,
    inst: Instant,
    paused: bool,
    op: T,
}

impl<T> Enemy<T>
where
    T: EnemyOp,
{
    pub fn new(id: usize, cube: Cube, ctx: Context, op: T) -> Self {
        Self {
            id,
            cube: Some(cube),
            inst: Instant::now(),
            ctx,
            op,
            paused: false,
        }
    }

    pub async fn init(&mut self) -> Result<()> {
        let mut cube = self.cube.take().unwrap();
        let ctx = self.ctx.clone();
        self.op.init(self.id, &mut cube, &ctx).await?;
        self.cube = Some(cube);
        Ok(())
    }

    pub async fn update(&mut self, player_pos: &Position) -> Result<()> {
        let mut cube = self.cube.take().unwrap();
        let ctx = self.ctx.clone();
        self.do_update(&mut cube, &ctx, player_pos).await?;
        self.cube = Some(cube);
        Ok(())
    }

    async fn do_update(
        &mut self,
        cube: &mut Cube,
        ctx: &Context,
        player_pos: &Position,
    ) -> Result<()> {
        if ctx.paused().await {
            if !self.paused {
                cube.go(0, 0, None).await?;
                self.paused = true;
            }
            return Ok(());
        } else {
            self.paused = false;
        }

        if self.inst.elapsed() < Duration::from_millis(200) {
            return Ok(());
        }
        self.inst = Instant::now();

        let enemy_pos = match cube.position().await? {
            Some(p) => p,
            None => {
                warn!("Enemy is out of field");
                return Ok(());
            }
        };

        self.op
            .update(self.id, cube, ctx, player_pos, &enemy_pos)
            .await?;

        Ok(())
    }
}
