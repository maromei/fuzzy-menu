use clap::Parser;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::io::Write;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    config: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Item {
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub items: Vec<Item>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            items: vec![
                Item {
                    name: "Example Command".to_string(),
                    description: "This is an example command description.".to_string(),
                    tags: vec!["example".to_string(), "test".to_string()],
                },
                Item {
                    name: "Another Command".to_string(),
                    description: "This is another command description with more details.".to_string(),
                    tags: vec!["other".to_string(), "demo".to_string()],
                },
            ],
        }
    }
}

fn get_default_config_path() -> Option<PathBuf> {
    if let Some(home_dir) = directories::BaseDirs::new() {
        let mut home_path = home_dir.home_dir().to_path_buf();
        home_path.push(".config");
        home_path.push("fuzzy-menu");
        return Some(home_path.join("config.toml"));
    }
    None
}

fn load_config(path: &Path) -> Result<Config, Box<dyn std::error::Error>> {
    if !path.exists() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let config = Config::default();
        let toml = toml::to_string_pretty(&config)?;
        fs::write(path, toml)?;
        return Ok(config);
    }

    let content = fs::read_to_string(path)?;
    let config: Config = toml::from_str(&content)?;
    Ok(config)
}

struct App {
    input: String,
    config: Config,
    filtered_items: Vec<Item>,
    list_state: ListState,
}

impl App {
    fn new(config: Config) -> App {
        let items = config.items.clone();
        let mut list_state = ListState::default();
        if !items.is_empty() {
            list_state.select(Some(0));
        }
        App {
            input: String::new(),
            config,
            filtered_items: items,
            list_state,
        }
    }

    fn filter_items(&mut self) {
        if self.input.is_empty() {
            self.filtered_items = self.config.items.clone();
        } else {
            // Using fzf in the background
            let mut input_data = String::new();
            for (idx, item) in self.config.items.iter().enumerate() {
                // Use a delimiter that is unlikely to be in the name/description
                input_data.push_str(&format!("{} | {} | {} | {}\n", idx, item.name, item.description, item.tags.join(" ")));
            }

            let mut child = Command::new("fzf")
                .arg("--filter")
                .arg(&self.input)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .expect("Failed to spawn fzf");

            {
                let stdin = child.stdin.as_mut().expect("Failed to open stdin");
                stdin.write_all(input_data.as_bytes()).expect("Failed to write to stdin");
            }

            let output = child.wait_with_output().expect("Failed to read stdout");
            let stdout = String::from_utf8_lossy(&output.stdout);

            let mut new_filtered = Vec::new();
            for line in stdout.lines() {
                if let Some(idx_str) = line.split('|').next() {
                    if let Ok(idx) = idx_str.trim().parse::<usize>() {
                        if idx < self.config.items.len() {
                            new_filtered.push(self.config.items[idx].clone());
                        }
                    }
                }
            }
            self.filtered_items = new_filtered;
        }
        
        if self.filtered_items.is_empty() {
            self.list_state.select(None);
        } else {
            self.list_state.select(Some(0));
        }
    }

    fn next(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.filtered_items.len().saturating_sub(1) {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        if !self.filtered_items.is_empty() {
            self.list_state.select(Some(i));
        }
    }

    fn previous(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.filtered_items.len().saturating_sub(1)
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        if !self.filtered_items.is_empty() {
            self.list_state.select(Some(i));
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let config_path = if let Some(path) = args.config {
        PathBuf::from(path)
    } else {
        get_default_config_path().expect("Could not determine config path")
    };

    let config = load_config(&config_path)?;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let app = App::new(config);
    let res = run_app(&mut terminal, app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> Result<(), Box<dyn std::error::Error>> 
where 
    <B as Backend>::Error: std::error::Error + 'static
{
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Esc => return Ok(()),
                KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => return Ok(()),
                KeyCode::Char(c) => {
                    app.input.push(c);
                    app.filter_items();
                }
                KeyCode::Backspace => {
                    app.input.pop();
                    app.filter_items();
                }
                KeyCode::Down => {
                    app.next();
                }
                KeyCode::Up => {
                    app.previous();
                }
                KeyCode::Enter => {
                    // Currently no action specified for Enter in requirements
                    // but we could exit or execute the command.
                    // For now, let's just exit.
                    return Ok(());
                }
                _ => {}
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(1),
            ]
            .as_ref(),
        )
        .split(f.area());

    let input = Paragraph::new(app.input.as_str())
        .block(Block::default().borders(Borders::ALL).title("Search"));
    f.render_widget(input, chunks[0]);

    let items: Vec<ListItem> = app
        .filtered_items
        .iter()
        .map(|item| {
            let lines = vec![
                Line::from(vec![
                    Span::styled(&item.name, Style::default().add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![
                    Span::raw(&item.description),
                ]),
                Line::from(vec![
                    Span::styled(item.tags.join(", "), Style::default().fg(Color::Cyan)),
                ]),
                Line::from(""), // Spacer
            ];
            ListItem::new(lines)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Matches"))
        .highlight_style(
            Style::default()
                .bg(Color::LightBlue)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, chunks[1], &mut app.list_state);
}
