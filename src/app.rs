extern crate rss;
extern crate tui;

use crate::network::IoEvent;
use rss::Channel;
use tui::widgets::ListState;

use std::sync::mpsc::Sender;

#[derive(Clone)]
pub struct StatefulList<T> {
    pub state: ListState,
    pub items: Vec<T>,
}

#[derive(Clone)]
pub struct Feed {
    pub channel: Option<Channel>,
    pub name: String,
    pub url: String,
}

impl Feed {
    pub fn new(name: String, url: String) -> Feed {
        Feed {
            name,
            url,
            channel: None,
        }
    }

    pub fn set_channel(&mut self, channel: Channel) {
        self.channel = Some(channel);
    }
}

impl<T> StatefulList<T> {
    pub fn with_items(items: Vec<T>) -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items,
        }
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn unselect(&mut self) {
        self.state.select(None);
    }
}

pub struct App {
    pub data: StatefulList<Feed>,
    pub selected_feed: Option<Feed>,
    io_tx: Option<Sender<IoEvent>>,
    pub is_loading: bool,
}

impl App {
    pub fn new(data: Vec<Feed>, io_tx: Sender<IoEvent>) -> App {
        App {
            data: StatefulList::with_items(data),
            selected_feed: None,
            io_tx: Some(io_tx),
            is_loading: false,
        }
    }

    pub fn dispatch(&mut self, action: IoEvent) {
        self.is_loading = true;
        if let Some(io_tx) = &self.io_tx {
            if let Err(e) = io_tx.send(action) {
                self.is_loading = false;
                println!("Error from dispatch {}", e);
            };
        }
    }

    pub fn get_channel(&mut self, feed: Feed) {
        self.dispatch(IoEvent::GetChannel(feed));
    }

    pub fn view_feed_under_cursor(&mut self) {
        let index = self.data.state.selected();
        match index {
            None => panic!(),
            Some(i) => self.get_channel(self.data.items[i].clone()),
        }
    }
}
