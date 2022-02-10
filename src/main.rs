extern crate crossterm;
extern crate tui;

use reqwest;
use rss::Channel;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    error::Error,
    io,
    time::{Duration, Instant},
};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{ Span, Spans },
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame, Terminal,
};

struct StatefulList<T> {
    state: ListState,
    items: Vec<T>,
}

impl<T> StatefulList<T> {
    fn with_items(items: Vec<T>) -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items,
        }
    }

    fn next(&mut self) {
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

    fn previous(&mut self) {
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

    fn unselect(&mut self) {
        self.state.select(None);
    }
}

/// This struct holds the current state of the app. In particular, it has the `items` field which is a wrapper
/// around `ListState`. Keeping track of the items state let us render the associated widget with its state
/// and have access to features such as natural scrolling.
///
/// Check the event handling at the bottom to see how to change the state on incoming events.
/// Check the drawing logic for items on how to specify the highlighting style for selected items.
struct App {
    data: StatefulList<rss::Channel>,
    selected_feed: Option<rss::Channel>,
}

impl App {
    fn new(data: Vec<rss::Channel> ) -> App {
        App {
            data: StatefulList::with_items(data),
            selected_feed: None,
        }
    }

    fn view_feed_under_cursor(&mut self) {
        let index = self.data.state.selected();
        match index {
            None => panic!(),
            Some(i) =>  self.selected_feed = Some(self.data.items.clone()[i].clone()),
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let urls = vec![
        "https://hnrss.org/frontpage",
        "http://www.sweclockers.com/feeds/nyheter",
        "https://rss.aftonbladet.se/rss2/small/pages/sections/senastenytt/",
        "https://www.svt.se/nyheter/rss.xml",
    ];
    let mut channels = Vec::new();
    for url in urls.iter() {
        let content = reqwest::blocking::get(*url)?.bytes()?;
        let channel = Channel::read_from(&content[..])?;
        channels.push(channel);
    }
    // create app and run it
    let tick_rate = Duration::from_millis(250);
    let app = App::new(channels);
    let res = run_app(&mut terminal, app, tick_rate);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    tick_rate: Duration,
) -> io::Result<()> {
    let mut last_tick = Instant::now();
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('h') => app.data.unselect(),
                    KeyCode::Char('j') => app.data.next(),
                    KeyCode::Char('k') => app.data.previous(),
                    KeyCode::Enter => app.view_feed_under_cursor(),
                    _ => {}
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    // Create two chunks with equal horizontal screen space
    let channel_picker_screen = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(f.size());

    // Iterate through all elements in the `items` app and append some debug text to it.
    let items: Vec<ListItem> = app
        .data
        .items
        .iter()
        .map(|i| {
            let lines = vec![Spans::from(i.title())];
            ListItem::new(lines).style(Style::default().fg(Color::White))
        })
        .collect();

    // Create a List from all list items and highlight the currently selected one
    let items = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("fred_rss"))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    // We can now render the item list
    let mut news_items = Vec::<ListItem>::new();
    match app.selected_feed.clone() {
        None => (),
        Some(item) => {
            for news in item.items() {
                let lines = Span::from(news.title().unwrap().to_string());
                news_items.push(ListItem::new(lines).style(Style::default().fg(Color::White)));
            }
        }
    }

    let news_list = List::new(news_items).block(Block::default().borders(Borders::ALL).title("News"));
    f.render_stateful_widget(items, channel_picker_screen[0], &mut app.data.state);
    f.render_widget(news_list, channel_picker_screen[1]);
}
