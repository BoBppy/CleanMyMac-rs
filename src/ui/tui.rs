//! Modern TUI interface using ratatui

use crate::cleaner::Cleaner;
use crate::rules::{CleanItem, RiskLevel, get_all_rules};
use crate::scanner::FileScanner;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Clear, List, ListItem, ListState, Padding, Paragraph,
        Scrollbar, ScrollbarOrientation, ScrollbarState, Tabs, Wrap,
    },
};
use std::io;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

/// Messages for communication between scanner thread and UI
enum ScanMessage {
    /// Found a batch of items
    FoundItems(Vec<CleanItem>),
    /// Scan completed
    Finished,
    /// Scan failed with error
    Error(String),
}

/// App state for the TUI
pub struct App {
    /// Current tab index
    current_tab: usize,
    /// List of scanned items
    items: Vec<CleanItem>,
    /// Selected items for cleaning
    selected: Vec<bool>,
    /// List state for navigation
    list_state: ListState,
    /// Should quit the app
    should_quit: bool,
    /// Is scanning in progress
    is_scanning: bool,
    /// Is cleaning in progress
    is_cleaning: bool,
    /// Status message
    status_message: String,
    /// Total size of selected items
    selected_size: u64,
    /// Scrollbar state
    scrollbar_state: ScrollbarState,
    /// Show help popup
    show_help: bool,
    /// Animation frame
    animation_frame: usize,
    /// Last tick time
    last_tick: Instant,
    /// Channel receiver for scan results
    scan_rx: Option<Receiver<ScanMessage>>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            current_tab: 0,
            items: Vec::new(),
            selected: Vec::new(),
            list_state: ListState::default(),
            should_quit: false,
            is_scanning: false,
            is_cleaning: false,
            status_message: String::from("Press 's' to scan, 'q' to quit"),
            selected_size: 0,
            scrollbar_state: ScrollbarState::default(),
            show_help: false,
            animation_frame: 0,
            last_tick: Instant::now(),
            scan_rx: None,
        }
    }
}

impl App {
    /// Create a new App
    pub fn new() -> Self {
        Self::default()
    }

    /// Run the main TUI loop
    pub fn run(&mut self) -> anyhow::Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Main loop
        let tick_rate = Duration::from_millis(100);

        loop {
            terminal.draw(|f| self.ui(f))?;

            // Check for scan results
            let mut scan_finished = false;
            if let Some(rx) = &self.scan_rx {
                while let Ok(msg) = rx.try_recv() {
                    match msg {
                        ScanMessage::FoundItems(items) => {
                            self.items = items;
                        }
                        ScanMessage::Finished => {
                            self.is_scanning = false;
                            scan_finished = true;

                            // Post-scan updates
                            self.selected = vec![false; self.items.len()];
                            self.scrollbar_state =
                                ScrollbarState::default().content_length(self.items.len());
                            if !self.items.is_empty() {
                                self.list_state.select(Some(0));
                            }
                            let total_size = self.items.iter().map(|i| i.size).sum::<u64>();
                            self.status_message = format!(
                                "✅ Found {} items ({}). Press Space to select, 'c' to clean",
                                self.items.len(),
                                format_bytes(total_size)
                            );
                        }
                        ScanMessage::Error(e) => {
                            self.is_scanning = false;
                            scan_finished = true;
                            self.status_message = format!("❌ Scan failed: {}", e);
                        }
                    }
                }
            }

            if scan_finished {
                self.scan_rx = None;
            }

            // Handle events with timeout
            if event::poll(tick_rate)? {
                if let Event::Key(key) = event::read()? {
                    self.handle_key(key.code, key.modifiers);
                }
            }

            // Update animation
            if self.last_tick.elapsed() >= Duration::from_millis(100) {
                self.animation_frame = (self.animation_frame + 1) % 8;
                self.last_tick = Instant::now();

                // Update status message if scanning
                if self.is_scanning {
                    let spinner = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
                    self.status_message = format!(
                        "{} Scanning...",
                        spinner[self.animation_frame % spinner.len()]
                    );
                }
            }

            if self.should_quit {
                break;
            }
        }

        // Restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        Ok(())
    }

    /// Handle key events
    fn handle_key(&mut self, key: KeyCode, modifiers: KeyModifiers) {
        if self.show_help {
            self.show_help = false;
            return;
        }

        match key {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
            }
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            KeyCode::Char('s') if !self.is_scanning => {
                self.scan();
            }
            KeyCode::Char('c') if !self.is_cleaning && !self.items.is_empty() => {
                self.clean();
            }
            KeyCode::Char('a') if !self.items.is_empty() => {
                // Select all
                let all_selected = self.selected.iter().all(|&s| s);
                self.selected.iter_mut().for_each(|s| *s = !all_selected);
                self.update_selected_size();
            }
            KeyCode::Tab => {
                self.current_tab = (self.current_tab + 1) % 3;
            }
            KeyCode::BackTab => {
                self.current_tab = if self.current_tab == 0 {
                    2
                } else {
                    self.current_tab - 1
                };
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.previous_item();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.next_item();
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                self.toggle_selection();
            }
            KeyCode::Char('?') => {
                self.show_help = true;
            }
            _ => {}
        }
    }

    /// Move to previous item
    fn previous_item(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
        self.scrollbar_state = self.scrollbar_state.position(i);
    }

    /// Move to next item
    fn next_item(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
        self.scrollbar_state = self.scrollbar_state.position(i);
    }

    /// Toggle selection of current item
    fn toggle_selection(&mut self) {
        if let Some(i) = self.list_state.selected() {
            if i < self.selected.len() {
                self.selected[i] = !self.selected[i];
                self.update_selected_size();
            }
        }
    }

    /// Update total selected size
    fn update_selected_size(&mut self) {
        self.selected_size = self
            .items
            .iter()
            .zip(self.selected.iter())
            .filter(|(_, s)| **s)
            .map(|(item, _)| item.size)
            .sum();
    }

    /// Scan for cleanable items
    fn scan(&mut self) {
        if self.is_scanning {
            return;
        }

        self.is_scanning = true;
        self.status_message = String::from("🔍 Scanning...");
        self.items.clear();
        self.selected.clear();

        let (tx, rx) = mpsc::channel();
        self.scan_rx = Some(rx);

        thread::spawn(move || {
            let rules = get_all_rules();
            let scanner = FileScanner::new(rules);
            match scanner.scan_quiet() {
                Ok(items) => {
                    let _ = tx.send(ScanMessage::FoundItems(items));
                    let _ = tx.send(ScanMessage::Finished);
                }
                Err(e) => {
                    let _ = tx.send(ScanMessage::Error(e.to_string()));
                }
            }
        });
    }

    /// Clean selected items
    fn clean(&mut self) {
        let selected_items: Vec<_> = self
            .items
            .iter()
            .zip(self.selected.iter())
            .filter(|(_, s)| **s)
            .map(|(item, _)| item.clone())
            .collect();

        if selected_items.is_empty() {
            self.status_message =
                String::from("⚠️ No items selected. Press Space to select items.");
            return;
        }

        self.is_cleaning = true;
        self.status_message = String::from("🧹 Cleaning...");

        let cleaner = Cleaner::new().use_trash(true).confirm_high_risk(false);

        match cleaner.clean(&selected_items) {
            Ok(result) => {
                self.status_message = format!(
                    "✅ Cleaned {} items, freed {}",
                    result.cleaned_count,
                    format_bytes(result.bytes_freed)
                );
                // Remove cleaned items
                let mut new_items = Vec::new();
                let mut new_selected = Vec::new();
                for (i, item) in self.items.iter().enumerate() {
                    if !self.selected[i] {
                        new_items.push(item.clone());
                        new_selected.push(false);
                    }
                }
                self.items = new_items;
                self.selected = new_selected;
                self.scrollbar_state = ScrollbarState::default().content_length(self.items.len());
                self.selected_size = 0;
                if !self.items.is_empty() {
                    self.list_state.select(Some(0));
                } else {
                    self.list_state.select(None);
                }
            }
            Err(e) => {
                self.status_message = format!("❌ Clean failed: {}", e);
            }
        }

        self.is_cleaning = false;
    }

    /// Render the UI
    fn ui(&mut self, frame: &mut Frame) {
        let size = frame.area();

        // Create main layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3), // Title bar
                Constraint::Length(3), // Tabs
                Constraint::Min(10),   // Main content
                Constraint::Length(3), // Status bar
            ])
            .split(size);

        // Render title bar with gradient effect
        self.render_title(frame, chunks[0]);

        // Render tabs
        self.render_tabs(frame, chunks[1]);

        // Render main content based on current tab
        match self.current_tab {
            0 => self.render_scan_tab(frame, chunks[2]),
            1 => self.render_stats_tab(frame, chunks[2]),
            2 => self.render_settings_tab(frame, chunks[2]),
            _ => {}
        }

        // Render status bar
        self.render_status_bar(frame, chunks[3]);

        // Render help popup if needed
        if self.show_help {
            self.render_help_popup(frame, size);
        }
    }

    /// Render title bar
    fn render_title(&self, frame: &mut Frame, area: Rect) {
        let title_text = vec![
            Span::styled("🧹 ", Style::default().fg(Color::Cyan)),
            Span::styled("Clean", Style::default().fg(Color::Cyan).bold()),
            Span::styled("My", Style::default().fg(Color::Blue).bold()),
            Span::styled("Mac", Style::default().fg(Color::Magenta).bold()),
            Span::styled("-rs", Style::default().fg(Color::Yellow).bold()),
            Span::styled(" • Modern System Cleaner", Style::default().fg(Color::Gray)),
        ];

        let title = Paragraph::new(Line::from(title_text))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Cyan))
                    .padding(Padding::horizontal(1)),
            )
            .style(Style::default());

        frame.render_widget(title, area);
    }

    /// Render tabs
    fn render_tabs(&self, frame: &mut Frame, area: Rect) {
        let titles = vec!["📂 Scan", "📊 Stats", "⚙️  Settings"];
        let tabs = Tabs::new(titles)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .select(self.current_tab)
            .style(Style::default().fg(Color::Gray))
            .highlight_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .divider("│");

        frame.render_widget(tabs, area);
    }

    /// Render scan tab
    fn render_scan_tab(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(area);

        // Left panel - Item list
        self.render_item_list(frame, chunks[0]);

        // Right panel - Details
        self.render_details_panel(frame, chunks[1]);
    }

    /// Render item list
    fn render_item_list(&mut self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let checkbox = if self.selected.get(i).copied().unwrap_or(false) {
                    "☑ "
                } else {
                    "☐ "
                };

                let risk_color = match item.risk_level {
                    RiskLevel::Low => Color::Green,
                    RiskLevel::Medium => Color::Yellow,
                    RiskLevel::High => Color::Red,
                };

                let size_str = format_bytes(item.size);
                let path_str = item.path.display().to_string();
                let path_short = if path_str.len() > 50 {
                    format!("...{}", &path_str[path_str.len() - 47..])
                } else {
                    path_str
                };

                let content = Line::from(vec![
                    Span::styled(checkbox, Style::default().fg(Color::Cyan)),
                    Span::styled("● ", Style::default().fg(risk_color)),
                    Span::styled(
                        format!("{:>10} ", size_str),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::styled(path_short, Style::default().fg(Color::White)),
                ]);

                ListItem::new(content)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title(format!(" 📁 Items ({}) ", self.items.len()))
                    .title_style(Style::default().fg(Color::Cyan).bold())
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .padding(Padding::horizontal(1)),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(list, area, &mut self.list_state);

        // Render scrollbar
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            area.inner(ratatui::layout::Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut self.scrollbar_state,
        );
    }

    /// Render details panel
    fn render_details_panel(&self, frame: &mut Frame, area: Rect) {
        let selected_count = self.selected.iter().filter(|&&s| s).count();

        let details = if let Some(i) = self.list_state.selected() {
            if let Some(item) = self.items.get(i) {
                vec![
                    Line::from(vec![Span::styled(
                        "Path: ",
                        Style::default().fg(Color::Gray),
                    )]),
                    Line::from(vec![Span::styled(
                        format!("  {}", item.path.display()),
                        Style::default().fg(Color::White),
                    )]),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("Size: ", Style::default().fg(Color::Gray)),
                        Span::styled(
                            format_bytes(item.size),
                            Style::default().fg(Color::Yellow).bold(),
                        ),
                    ]),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("Category: ", Style::default().fg(Color::Gray)),
                        Span::styled(item.category.to_string(), Style::default().fg(Color::Cyan)),
                    ]),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("Risk: ", Style::default().fg(Color::Gray)),
                        Span::styled(
                            item.risk_level.to_string(),
                            Style::default().fg(match item.risk_level {
                                RiskLevel::Low => Color::Green,
                                RiskLevel::Medium => Color::Yellow,
                                RiskLevel::High => Color::Red,
                            }),
                        ),
                    ]),
                    Line::from(""),
                    Line::from(vec![Span::styled(
                        "Description: ",
                        Style::default().fg(Color::Gray),
                    )]),
                    Line::from(vec![Span::styled(
                        format!("  {}", item.description),
                        Style::default().fg(Color::White),
                    )]),
                ]
            } else {
                vec![Line::from("No item selected")]
            }
        } else {
            vec![Line::from("No item selected")]
        };

        let mut content = details;
        content.push(Line::from(""));
        content.push(Line::from("─".repeat(25)));
        content.push(Line::from(""));
        content.push(Line::from(vec![
            Span::styled("Selected: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{} items", selected_count),
                Style::default().fg(Color::Cyan).bold(),
            ),
        ]));
        content.push(Line::from(vec![
            Span::styled("Total: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format_bytes(self.selected_size),
                Style::default().fg(Color::Green).bold(),
            ),
        ]));

        let paragraph = Paragraph::new(content)
            .block(
                Block::default()
                    .title(" 📋 Details ")
                    .title_style(Style::default().fg(Color::Cyan).bold())
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .padding(Padding::new(1, 1, 1, 1)),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }

    /// Render stats tab with visual bar chart
    fn render_stats_tab(&self, frame: &mut Frame, area: Rect) {
        use std::collections::HashMap;

        // Split area into two columns
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Left panel: Category breakdown with bar chart
        let mut by_category: HashMap<String, u64> = HashMap::new();
        for item in &self.items {
            *by_category.entry(item.category.to_string()).or_insert(0) += item.size;
        }

        let total_size: u64 = self.items.iter().map(|i| i.size).sum();

        let mut content = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("📊 ", Style::default()),
                Span::styled(
                    "Storage by Category",
                    Style::default().fg(Color::Cyan).bold(),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Total: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format_bytes(total_size),
                    Style::default().fg(Color::Green).bold(),
                ),
                Span::styled(
                    format!(" ({} items)", self.items.len()),
                    Style::default().fg(Color::Gray),
                ),
            ]),
            Line::from(""),
        ];

        let mut categories: Vec<_> = by_category.iter().collect();
        categories.sort_by(|a, b| b.1.cmp(a.1));

        let max_size = categories.first().map(|(_, s)| **s).unwrap_or(1);
        let bar_width = 20;

        // Color palette for categories
        let colors = [
            Color::Cyan,
            Color::Green,
            Color::Yellow,
            Color::Magenta,
            Color::Blue,
            Color::Red,
            Color::LightCyan,
            Color::LightGreen,
        ];

        for (i, (category, size)) in categories.iter().enumerate() {
            let percentage = if total_size > 0 {
                (**size as f64 / total_size as f64 * 100.0) as u16
            } else {
                0
            };

            // Calculate bar length
            let bar_len = if max_size > 0 {
                ((**size as f64 / max_size as f64) * bar_width as f64) as usize
            } else {
                0
            };

            let bar = "█".repeat(bar_len);
            let empty = "░".repeat(bar_width - bar_len);
            let color = colors[i % colors.len()];

            content.push(Line::from(vec![Span::styled(
                format!("{:<12} ", category),
                Style::default().fg(Color::White),
            )]));
            content.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(bar, Style::default().fg(color)),
                Span::styled(empty, Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!(" {:>8} ", format_bytes(**size)),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(
                    format!("{:>3}%", percentage),
                    Style::default().fg(Color::Gray),
                ),
            ]));
        }

        let left_panel = Paragraph::new(content)
            .block(
                Block::default()
                    .title(" 📊 Category Analysis ")
                    .title_style(Style::default().fg(Color::Cyan).bold())
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .padding(Padding::new(1, 1, 0, 0)),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(left_panel, chunks[0]);

        // Right panel: Top items and summary
        let mut right_content = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("📁 ", Style::default()),
                Span::styled("Top Items by Size", Style::default().fg(Color::Cyan).bold()),
            ]),
            Line::from(""),
        ];

        // Show top 10 items
        let mut sorted_items: Vec<_> = self.items.iter().collect();
        sorted_items.sort_by(|a, b| b.size.cmp(&a.size));

        for (i, item) in sorted_items.iter().take(10).enumerate() {
            let name = item
                .path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string());

            let name_short = if name.len() > 20 {
                format!("{}...", &name[..17])
            } else {
                name
            };

            let percentage = if total_size > 0 {
                (item.size as f64 / total_size as f64 * 100.0) as u16
            } else {
                0
            };

            let color = match i {
                0 => Color::Red,
                1 => Color::Yellow,
                2 => Color::Green,
                _ => Color::White,
            };

            right_content.push(Line::from(vec![
                Span::styled(format!("{:>2}. ", i + 1), Style::default().fg(Color::Gray)),
                Span::styled(format!("{:<20} ", name_short), Style::default().fg(color)),
                Span::styled(
                    format!("{:>8}", format_bytes(item.size)),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(
                    format!(" {:>2}%", percentage),
                    Style::default().fg(Color::Gray),
                ),
            ]));
        }

        // Add disk usage visualization
        right_content.push(Line::from(""));
        right_content.push(Line::from("─".repeat(35)));
        right_content.push(Line::from(""));
        right_content.push(Line::from(vec![
            Span::styled("💾 ", Style::default()),
            Span::styled(
                "Disk Space Reclaimable",
                Style::default().fg(Color::Cyan).bold(),
            ),
        ]));
        right_content.push(Line::from(""));

        // Create a simple visualization of space
        let segments: usize = 30;
        let mut usage_bar = String::new();
        let category_segments: Vec<_> = categories
            .iter()
            .take(5)
            .map(|(_, size)| {
                if total_size > 0 {
                    ((**size as f64 / total_size as f64) * segments as f64) as usize
                } else {
                    0
                }
            })
            .collect();

        let mut used = 0;
        for (i, &seg) in category_segments.iter().enumerate() {
            let symbol = match i {
                0 => "█",
                1 => "▓",
                2 => "▒",
                3 => "░",
                _ => "·",
            };
            usage_bar.push_str(&symbol.repeat(seg));
            used += seg;
        }
        usage_bar.push_str(&"·".repeat(segments.saturating_sub(used)));

        right_content.push(Line::from(vec![
            Span::styled("[", Style::default().fg(Color::Gray)),
            Span::styled(
                &usage_bar[..usage_bar.len().min(30)],
                Style::default().fg(Color::Cyan),
            ),
            Span::styled("]", Style::default().fg(Color::Gray)),
        ]));

        let right_panel = Paragraph::new(right_content)
            .block(
                Block::default()
                    .title(" � Size Analysis ")
                    .title_style(Style::default().fg(Color::Cyan).bold())
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .padding(Padding::new(1, 1, 0, 0)),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(right_panel, chunks[1]);
    }

    /// Render settings tab
    fn render_settings_tab(&self, frame: &mut Frame, area: Rect) {
        let content = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("⚙️  ", Style::default()),
                Span::styled("Settings", Style::default().fg(Color::Cyan).bold()),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  [", Style::default().fg(Color::Gray)),
                Span::styled("✓", Style::default().fg(Color::Green)),
                Span::styled("] Move to Trash", Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("  [", Style::default().fg(Color::Gray)),
                Span::styled("✓", Style::default().fg(Color::Green)),
                Span::styled(
                    "] Confirm High-Risk Operations",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  [", Style::default().fg(Color::Gray)),
                Span::styled("✓", Style::default().fg(Color::Green)),
                Span::styled("] Scan Hidden Files", Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("  [", Style::default().fg(Color::Gray)),
                Span::styled("✓", Style::default().fg(Color::Green)),
                Span::styled("] Heuristic Detection", Style::default().fg(Color::White)),
            ]),
            Line::from(""),
            Line::from("─".repeat(40)),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Config file: ",
                Style::default().fg(Color::Gray),
            )]),
            Line::from(vec![Span::styled(
                "  ~/.config/cleanmymac-rs/config.toml",
                Style::default().fg(Color::Yellow),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Tip: Edit the config file to customize behavior",
                Style::default().fg(Color::Gray).italic(),
            )]),
        ];

        let paragraph = Paragraph::new(content)
            .block(
                Block::default()
                    .title(" ⚙️  Settings ")
                    .title_style(Style::default().fg(Color::Cyan).bold())
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .padding(Padding::new(2, 2, 1, 1)),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }

    /// Render status bar
    fn render_status_bar(&self, frame: &mut Frame, area: Rect) {
        let spinner = if self.is_scanning || self.is_cleaning {
            let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];
            frames[self.animation_frame]
        } else {
            ""
        };

        let status = Line::from(vec![
            Span::styled(spinner, Style::default().fg(Color::Cyan)),
            Span::styled(" ", Style::default()),
            Span::styled(&self.status_message, Style::default().fg(Color::White)),
        ]);

        let help = Line::from(vec![
            Span::styled(" s", Style::default().fg(Color::Cyan).bold()),
            Span::styled(":scan ", Style::default().fg(Color::Gray)),
            Span::styled("c", Style::default().fg(Color::Cyan).bold()),
            Span::styled(":clean ", Style::default().fg(Color::Gray)),
            Span::styled("a", Style::default().fg(Color::Cyan).bold()),
            Span::styled(":all ", Style::default().fg(Color::Gray)),
            Span::styled("?", Style::default().fg(Color::Cyan).bold()),
            Span::styled(":help ", Style::default().fg(Color::Gray)),
            Span::styled("q", Style::default().fg(Color::Cyan).bold()),
            Span::styled(":quit", Style::default().fg(Color::Gray)),
        ]);

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);

        let left = Paragraph::new(status).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::DarkGray)),
        );

        let right = Paragraph::new(help)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .alignment(ratatui::layout::Alignment::Right);

        frame.render_widget(left, chunks[0]);
        frame.render_widget(right, chunks[1]);
    }

    /// Render help popup
    fn render_help_popup(&self, frame: &mut Frame, area: Rect) {
        let popup_area = centered_rect(60, 70, area);

        frame.render_widget(Clear, popup_area);

        let help_text = vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "  Keyboard Shortcuts",
                Style::default().fg(Color::Cyan).bold(),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  s        ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    "Scan for cleanable files",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  c        ", Style::default().fg(Color::Yellow)),
                Span::styled("Clean selected items", Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("  a        ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    "Select/Deselect all items",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  Space    ", Style::default().fg(Color::Yellow)),
                Span::styled("Toggle selection", Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("  ↑/k      ", Style::default().fg(Color::Yellow)),
                Span::styled("Move up", Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("  ↓/j      ", Style::default().fg(Color::Yellow)),
                Span::styled("Move down", Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("  Tab      ", Style::default().fg(Color::Yellow)),
                Span::styled("Switch tabs", Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("  ?        ", Style::default().fg(Color::Yellow)),
                Span::styled("Show this help", Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("  q/Esc    ", Style::default().fg(Color::Yellow)),
                Span::styled("Quit", Style::default().fg(Color::White)),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "  Press any key to close",
                Style::default().fg(Color::Gray).italic(),
            )]),
        ];

        let help = Paragraph::new(help_text)
            .block(
                Block::default()
                    .title(" ❓ Help ")
                    .title_style(Style::default().fg(Color::Cyan).bold())
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Cyan))
                    .padding(Padding::new(1, 1, 0, 0)),
            )
            .style(Style::default().bg(Color::Black));

        frame.render_widget(help, popup_area);
    }
}

/// Format bytes to human-readable string
fn format_bytes(bytes: u64) -> String {
    bytesize::ByteSize::b(bytes).to_string()
}

/// Create a centered rect
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
