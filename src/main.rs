mod app;
mod network;

extern crate crossterm;
extern crate rss;
extern crate serde;
extern crate tui;

use app::{App, Feed};
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use network::{IoEvent, Network};

use std::{
    io,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let feeds = vec![
        Feed::new(
            "SVT".to_string(),
            "https://www.svt.se/nyheter/rss.xml".to_string(),
        ),
        Feed::new(
            "HN Frontpage".to_string(),
            "https://hnrss.org/frontpage".to_string(),
        ),
    ];
    // create app and run it
    let tick_rate = Duration::from_millis(250);
    let (sync_io_tx, sync_io_rx) = std::sync::mpsc::channel::<IoEvent>();
    let app = Arc::new(Mutex::new(App::new(feeds, sync_io_tx)));
    let cloned_app = Arc::clone(&app);
    std::thread::spawn(move || {
        let mut network = Network::new(&app);
        start_tokio(sync_io_rx, &mut network);
    });
    let _res = run_app(&mut terminal, &cloned_app, tick_rate).await?;

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

#[tokio::main]
async fn start_tokio<'a>(io_rx: std::sync::mpsc::Receiver<IoEvent>, network: &mut Network) {
    while let Ok(io_event) = io_rx.recv() {
        network.handle_network_event(io_event).await;
    }
}

async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &Arc<Mutex<App>>,
    tick_rate: Duration,
) -> io::Result<()> {
    let mut last_tick = Instant::now();
    loop {
        let mut app = app.lock().await;
        terminal.draw(|mut f| ui(&mut f, &app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            let event = event::read()?;
            match event {
                Event::Key(KeyEvent {
                    modifiers: KeyModifiers::CONTROL,
                    code: KeyCode::Char('w'),
                }) => app.switch_view(),
                Event::Key(KeyEvent {
                    modifiers: KeyModifiers::NONE,
                    code: KeyCode::Char('q'),
                }) => {
                    if app.stacking <=0 {
                        return Ok(())
                    } else {
                        app.back();
                    }
                },
                Event::Key(KeyEvent {
                    modifiers: KeyModifiers::NONE,
                    code: KeyCode::Char('h'),
                }) => app.feed_data.unselect(),
                Event::Key(KeyEvent {
                    modifiers: KeyModifiers::NONE,
                    code: KeyCode::Char('j'),
                }) => { 
                    match app.selected_view {
                        app::SelectedView::FeedView => app.feed_data.next(),
                        app::SelectedView::NewsView => app.news_data.as_mut().unwrap().next(),
                    }
                },
                Event::Key(KeyEvent {
                    modifiers: KeyModifiers::NONE,
                    code: KeyCode::Char('k'),
                }) => { 
                    match app.selected_view {
                        app::SelectedView::FeedView => app.feed_data.next(),
                        app::SelectedView::NewsView => app.news_data.as_mut().unwrap().previous(),
                    }
                },
                Event::Key(KeyEvent {
                    modifiers: KeyModifiers::NONE,
                    code: KeyCode::Enter,
                }) => 
                    match app.selected_view {
                        app::SelectedView::FeedView => app.view_feed_under_cursor(),
                        app::SelectedView::NewsView => app.view_news_under_cursor(),
                    },
                _ => {}
            }
        }
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &App) {
    // Create two chunks with equal horizontal screen space
    let channel_picker_screen = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(f.size());

    // Iterate through all elements in the `items` app and append some debug text to it.
    let items: Vec<ListItem> = app
        .feed_data
        .items
        .iter()
        .map(|i| {
            let lines = vec![Spans::from(i.name.clone())];
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
    match &app.news_data {
        None => (),
        Some(item) => {
            for news in item.items.iter() {
                let mut description = String::from("...");
                match &news.description {
                    Some(n) => description = n.to_string().clone(),
                    None => (),
                };
                let text = vec![
                    Spans::from(news.title().unwrap().to_string()),
                    Spans::from(vec![Span::styled(description, Style::default().fg(Color::Yellow))]),
                ];
                news_items.push(ListItem::new(text).style(Style::default().fg(Color::White)));
            }
        },
    }

    let news_list = List::new(news_items)
        .block(Block::default().borders(Borders::ALL).title("News"))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");


    // let news_list =
    //     List::new(news_items).block(Block::default().borders(Borders::ALL).title("News"));

    f.render_stateful_widget(items, channel_picker_screen[0], &mut app.feed_data.state.clone());
    match app.news_data {
        Some(_) => f.render_stateful_widget(news_list, channel_picker_screen[1], &mut app.news_data.as_ref().unwrap().state.clone()),
        None => {},
    }
}
