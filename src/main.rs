use clap::Parser;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    config: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Item {
    pub command: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    #[serde(skip)]
    pub key: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(flatten)]
    pub items: HashMap<String, Item>,
}

impl Default for Config {
    fn default() -> Self {
        let mut items = HashMap::new();
        items.insert(
            "example-command".to_string(),
            Item {
                command: "echo 'Hello World'".to_string(),
                name: Some("Example Command".to_string()),
                description: Some("\n    This is some multiline string.\n    The indentation will be removed on each line.\n        Since this line has an additional indentation level, this additional\n        one will be displayed.".to_string()),
                tags: Some(vec!["example".to_string(), "test".to_string()]),
                key: "example-command".to_string(),
            },
        );
        items.insert(
            "ls-home".to_string(),
            Item {
                command: "ls -la ~".to_string(),
                name: None,
                description: Some("List home directory content.".to_string()),
                tags: None,
                key: "ls-home".to_string(),
            },
        );
        Self { items }
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
    let mut config: Config = toml::from_str(&content)?;

    // Populate the key field from the map keys and process descriptions
    for (key, item) in config.items.iter_mut() {
        item.key = key.clone();
        if let Some(desc) = &mut item.description {
            *desc = process_description(desc);
        }
    }

    Ok(config)
}

fn process_description(s: &str) -> String {
    if !s.contains('\n') && !s.contains('\r') {
        return s.to_string();
    }

    let lines: Vec<&str> = s.lines().collect();
    if lines.is_empty() {
        return s.to_string();
    }

    // Skip the first line if it's empty (common in TOML multiline strings)
    let start_idx = if lines[0].is_empty() && lines.len() > 1 {
        1
    } else {
        0
    };

    let first_content_line = lines[start_idx];
    let ws_count = first_content_line
        .chars()
        .take_while(|c| c.is_whitespace())
        .count();

    if ws_count > 0 {
        let ws_to_remove = &first_content_line[..ws_count];
        lines[start_idx..]
            .iter()
            .map(|line| {
                if line.starts_with(ws_to_remove) {
                    &line[ws_to_remove.len()..]
                } else if line.trim().is_empty() {
                    ""
                } else {
                    line
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        lines[start_idx..].join("\n")
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Mode {
    Insert,
    Normal,
}

struct App {
    input: String,
    config: Config,
    filtered_items: Vec<Item>,
    list_state: ListState,
    mode: Mode,
}

impl App {
    fn new(config: Config) -> App {
        let mut items: Vec<Item> = config.items.values().cloned().collect();
        items.sort_by(|a, b| a.key.cmp(&b.key));
        let mut list_state = ListState::default();
        if !items.is_empty() {
            list_state.select(Some(0));
        }
        App {
            input: String::new(),
            config,
            filtered_items: items,
            list_state,
            mode: Mode::Insert,
        }
    }

    fn filter_items(&mut self) {
        let all_items: Vec<Item> = self.config.items.values().cloned().collect();
        if self.input.is_empty() {
            self.filtered_items = all_items;
            self.filtered_items.sort_by(|a, b| a.key.cmp(&b.key));
        } else {
            // Using fzf in the background
            let mut input_data = String::new();
            for (idx, item) in all_items.iter().enumerate() {
                let name = item.name.as_deref().unwrap_or(&item.key);
                let description = item.description.as_deref().unwrap_or("");
                let tags = item.tags.as_ref().map(|t| t.join(" ")).unwrap_or_default();
                input_data.push_str(&format!(
                    "{} | {} | {} | {} | {}\n",
                    idx, name, item.command, description, tags
                ));
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
                stdin
                    .write_all(input_data.as_bytes())
                    .expect("Failed to write to stdin");
            }

            let output = child.wait_with_output().expect("Failed to read stdout");
            let stdout = String::from_utf8_lossy(&output.stdout);

            let mut new_filtered = Vec::new();
            for line in stdout.lines() {
                if let Some(idx_str) = line.split('|').next()
                    && let Ok(idx) = idx_str.trim().parse::<usize>()
                        && idx < all_items.len() {
                            new_filtered.push(all_items[idx].clone());
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

    match res {
        Ok(Some(command)) => {
            // Execute the command
            let mut child = Command::new("sh").arg("-c").arg(command).spawn()?;
            child.wait()?;
        }
        Ok(None) => {}
        Err(err) => {
            println!("{:?}", err);
        }
    }

    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
) -> Result<Option<String>, Box<dyn std::error::Error>>
where
    <B as Backend>::Error: std::error::Error + 'static,
{
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            match app.mode {
                Mode::Insert => match key.code {
                    KeyCode::Esc => {
                        app.mode = Mode::Normal;
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Ok(None);
                    }
                    KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.next();
                    }
                    KeyCode::Char('y') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.previous();
                    }
                    KeyCode::Down => {
                        app.next();
                    }
                    KeyCode::Up => {
                        app.previous();
                    }
                    KeyCode::Char(c) => {
                        app.input.push(c);
                        app.filter_items();
                    }
                    KeyCode::Backspace => {
                        app.input.pop();
                        app.filter_items();
                    }
                    KeyCode::Enter => {
                        if let Some(i) = app.list_state.selected()
                            && i < app.filtered_items.len() {
                                return Ok(Some(app.filtered_items[i].command.clone()));
                            }
                    }
                    _ => {}
                },
                Mode::Normal => match key.code {
                    KeyCode::Char('i') => {
                        app.mode = Mode::Insert;
                    }
                    KeyCode::Char('j') => {
                        app.next();
                    }
                    KeyCode::Char('k') => {
                        app.previous();
                    }
                    KeyCode::Esc => {
                        return Ok(None);
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Ok(None);
                    }
                    KeyCode::Enter => {
                        if let Some(i) = app.list_state.selected()
                            && i < app.filtered_items.len() {
                                return Ok(Some(app.filtered_items[i].command.clone()));
                            }
                    }
                    _ => {}
                },
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([Constraint::Length(3), Constraint::Min(1)].as_ref())
        .split(f.area());

    let title = format!("Search [{:?}]", app.mode);
    let input = Paragraph::new(app.input.as_str())
        .block(Block::default().borders(Borders::ALL).title(title));
    f.render_widget(input, chunks[0]);

    let selected_index = app.list_state.selected();
    // width of the list area minus borders
    let list_width = chunks[1].width.saturating_sub(2);

    let items: Vec<ListItem> = app
        .filtered_items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_selected = Some(i) == selected_index;
            let display_name = item.name.as_deref().unwrap_or(&item.key);

            let mut content_lines = vec![Line::from(vec![Span::styled(
                display_name,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )])];

            if let Some(desc) = &item.description {
                for line in desc.lines() {
                    content_lines.push(Line::from(vec![Span::raw(line)]));
                }
            }

            if let Some(tags) = &item.tags
                && !tags.is_empty() {
                    content_lines.push(Line::from(vec![Span::styled(
                        tags.join(", "),
                        Style::default().fg(Color::Cyan),
                    )]));
                }

            if is_selected {
                let border_style = Style::default().fg(Color::Yellow);
                let inner_width = (list_width as usize).saturating_sub(2);
                let top_border = format!("┌{}┐", "─".repeat(inner_width));
                let bottom_border = format!("└{}┘", "─".repeat(inner_width));

                let mut final_lines = vec![Line::from(Span::styled(top_border, border_style))];

                for line in content_lines {
                    let mut spans = vec![Span::styled("│ ", border_style)];
                    spans.extend(line.spans);
                    final_lines.push(Line::from(spans));
                }

                final_lines.push(Line::from(Span::styled(bottom_border, border_style)));
                ListItem::new(final_lines)
            } else {
                let mut final_lines = vec![Line::from("")]; // Top spacer (to match top border)
                for line in content_lines {
                    let mut spans = vec![Span::raw("  ")]; // Match "│ " width
                    spans.extend(line.spans);
                    final_lines.push(Line::from(spans));
                }
                final_lines.push(Line::from("")); // Bottom spacer (to match bottom border)
                ListItem::new(final_lines)
            }
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Matches"))
        .highlight_style(Style::default())
        .highlight_symbol("");

    f.render_stateful_widget(list, chunks[1], &mut app.list_state);
}
