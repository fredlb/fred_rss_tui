extern crate rss;
use crate::app::App;

use std::sync::Arc;
use tokio::sync::Mutex;

pub enum IoEvent {
    GetChannel(String),
}

pub struct Network {
    pub app: Arc<Mutex<App>>,
}

impl Network {
    pub fn new(app: Arc<Mutex<App>>) -> Network {
        Network { app }
    }

    pub async fn handle_network_event(&mut self, io_event: IoEvent) {
        match io_event {
            IoEvent::GetChannel(url) => {
                self.get_channel(url).await;
            }
        }
        let mut app = self.app.lock().await;
        app.is_loading = false;
    }

    async fn get_channel(&mut self, url: String) {
        let result = reqwest::get(url).await;
        match result {
            Ok(result) => match result.bytes().await {
                Ok(result) => {
                    let channel = rss::Channel::read_from(&result[..]);
                    let mut app = self.app.lock().await;
                    app.set_feed(channel.unwrap());
                }
                Err(_e) => {}
            },
            Err(_e) => {}
        }
    }
}
