use crate::console_graphics::GraphicsEngine;
use crate::networking::{Broadcaster, Receiver};
use std::sync::{Arc, Mutex};

pub struct UserInterface {
    pub graphics_engine: Arc<Mutex<GraphicsEngine>>,
    pub receiver: Arc<Mutex<Receiver>>,
    pub broadcaster: Broadcaster,
    pub username: String,
}

impl Clone for UserInterface {
    fn clone(&self) -> Self {
        Self {
            graphics_engine: self.graphics_engine.clone(),
            receiver: self.receiver.clone(),
            broadcaster: self.broadcaster.clone(),
            username: self.username.clone(),
        }
    }
}

impl UserInterface {
    pub fn new(
        receiver: Receiver,
        broadcaster: Broadcaster,
        graphics_engine: GraphicsEngine,
    ) -> Self {
        Self {
            graphics_engine: Arc::new(Mutex::new(graphics_engine)),
            receiver: Arc::new(Mutex::new(receiver)),
            broadcaster,
            username: String::new(),
        }
    }
}
