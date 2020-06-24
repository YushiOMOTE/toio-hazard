use anyhow::{anyhow, Result};
use gilrs::{Event, Gilrs};
use tokio::sync::mpsc::{self, UnboundedReceiver};

pub fn start() -> Result<UnboundedReceiver<Event>> {
    let mut gilrs = Gilrs::new().map_err(|e| anyhow!("Gil error: {}", e))?;

    let (tx, rx) = mpsc::unbounded_channel();

    std::thread::spawn(move || loop {
        while let Some(e) = gilrs.next_event() {
            if tx.send(e).is_err() {
                break;
            }
        }
    });

    Ok(rx)
}
