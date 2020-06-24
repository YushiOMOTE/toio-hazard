use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

pub enum PlayerStatus {
    Fine,
    Caution,
    Danger,
    Death,
}

#[derive(Debug)]
pub struct Inner {
    health: usize,
    hit: Option<Instant>,
    pause: bool,
}

impl Inner {
    fn new() -> Self {
        Self {
            health: 100,
            hit: None,
            pause: false,
        }
    }

    fn pause(&mut self) {
        self.pause = !self.pause;
    }

    fn paused(&self) -> bool {
        self.pause
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

#[derive(Debug, Clone)]
pub struct Context {
    inner: Arc<Mutex<Inner>>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner::new())),
        }
    }

    pub async fn pause(&self) {
        self.inner.lock().await.pause()
    }

    pub async fn paused(&self) -> bool {
        self.inner.lock().await.paused()
    }

    pub async fn damage(&self, damage: usize) -> Option<usize> {
        self.inner.lock().await.damage(damage)
    }

    pub async fn status(&self) -> PlayerStatus {
        self.inner.lock().await.status()
    }

    pub async fn gameover(&self) -> bool {
        self.inner.lock().await.gameover()
    }
}
