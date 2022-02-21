use crate::app::{App, Feed};

use rss::Channel;
use std::sync::Arc;
use tokio::sync::Mutex;

pub enum IoEvent {
    GetChannel(Feed),
}

pub struct Network<'a> {
    pub app: &'a Arc<Mutex<App>>,
}

impl<'a> Network<'a> {
    pub fn new(app: &'a Arc<Mutex<App>>) -> Network {
        Network { app }
    }

    pub async fn handle_network_event(&mut self, io_event: IoEvent) {
        match io_event {
            IoEvent::GetChannel(feed) => {
                self.get_channel(feed).await;
            }
        }
        let mut app = self.app.lock().await;
        app.is_loading = false;
    }

    async fn get_channel(&mut self, feed: Feed) {
        let result = reqwest::get(feed.url.clone()).await;
        match result {
            Ok(result) => match result.bytes().await {
                Ok(result) => {
                    let channel = Channel::read_from(&result[..]);
                    let mut app = self.app.lock().await;
                    let feed = Feed::new(feed.name.clone(), feed.url.clone());
                    app.set_feed(channel.unwrap());
                    app.selected_feed = Some(feed);
                }
                Err(_e) => {}
            },
            Err(_e) => {}
        }
    }
}
