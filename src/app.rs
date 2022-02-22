extern crate rss;
extern crate tui;

use crate::network::IoEvent;
use serde::{Deserialize, Serialize};
use tui::widgets::ListState;

use std::sync::mpsc::Sender;

#[derive(Clone)]
pub struct StatefulList<T> {
    pub state: ListState,
    pub items: Vec<T>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Config {
    pub feeds: Vec<ConfigFeed>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ConfigFeed {
    pub name: String,
    pub url: String,
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

pub enum SelectedView {
    FeedView,
    NewsView,
}

pub struct App {
    pub feeds: StatefulList<ConfigFeed>,
    pub news_data: Option<StatefulList<rss::Item>>,
    io_tx: Option<Sender<IoEvent>>,
    pub is_loading: bool,
    pub selected_view: SelectedView,
    pub news_index: usize,
    pub stacking: usize,
    pub config: Config,
}

impl App {
    pub fn new(config: Config, io_tx: Sender<IoEvent>) -> App {
        App {
            config: config.clone(),
            feeds: StatefulList::with_items(config.feeds.clone()),
            news_data: None,
            io_tx: Some(io_tx),
            is_loading: false,
            selected_view: SelectedView::FeedView,
            news_index: 0,
            stacking: 0,
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

    pub fn switch_view(&mut self) {
        match self.selected_view {
            SelectedView::FeedView => self.selected_view = SelectedView::NewsView,
            SelectedView::NewsView => self.selected_view = SelectedView::FeedView,
        }
    }

    pub fn get_channel(&mut self, url: String) {
        self.dispatch(IoEvent::GetChannel(url));
    }

    pub fn set_feed(&mut self, channel: rss::Channel) {
        self.news_data = Some(StatefulList::with_items(channel.items().to_vec()));
    }

    pub fn view_feed_under_cursor(&mut self) {
        if let Some(index) = self.feeds.state.selected() {
            self.get_channel(self.feeds.items[index].url.clone());
        }
    }

    pub fn back(&mut self) {
        self.news_index = 0;
        self.stacking -= 1;
    }

    pub fn view_news_under_cursor(&mut self) {
        self.stacking += 1;
        match &self.news_data {
            Some(data) => match data.state.selected() {
                Some(i) => self.news_index = i,
                None => self.news_index = 0,
            },
            None => {}
        }
    }
}
