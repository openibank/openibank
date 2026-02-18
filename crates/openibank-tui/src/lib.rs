use std::path::{Path, PathBuf};
use std::time::Duration;

use crossterm::event::{self, Event as CEvent, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use openibank_demo::{DemoEngine, DemoError};
use openibank_domain::Receipt;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::{Frame, Terminal};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TuiError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("demo error: {0}")]
    Demo(#[from] DemoError),
}

pub struct TuiRunResult {
    pub run_id: String,
}

struct TuiState {
    status_line: String,
    selected_receipt: usize,
}

pub async fn run_demo_tui(
    seed: u64,
    export_dir: Option<PathBuf>,
) -> Result<TuiRunResult, TuiError> {
    let engine = DemoEngine::new(seed).await?;
    engine.start().await?;

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = TuiState {
        status_line: "demo started".to_string(),
        selected_receipt: 0,
    };

    let result = loop {
        let runtime = engine.runtime();
        let agents = runtime.list_agents().await;
        let worldline_log = runtime.tail_worldline(64).await;
        let receipts = runtime.latest_receipts(32).await;

        if !receipts.is_empty() {
            state.selected_receipt = state.selected_receipt.min(receipts.len() - 1);
        } else {
            state.selected_receipt = 0;
        }

        terminal.draw(|frame| {
            draw_ui(
                frame,
                &runtime.worldline_id(),
                runtime.maple_version(),
                runtime.run_id(),
                &agents,
                &worldline_log,
                &receipts,
                state.selected_receipt,
                &state.status_line,
                engine.is_running(),
            );
        })?;

        if event::poll(Duration::from_millis(150))? {
            if let CEvent::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => {
                        if engine.is_running() {
                            let _ = engine.stop().await;
                        }
                        if let Some(dir) = export_dir.as_ref() {
                            if let Ok(path) = engine.export_bundle(dir).await {
                                state.status_line = format!("bundle exported: {}", path.display());
                            }
                        }
                        break Ok(TuiRunResult {
                            run_id: runtime.run_id().to_string(),
                        });
                    }
                    KeyCode::Char('s') | KeyCode::Char('S') => {
                        if engine.is_running() {
                            engine.stop().await?;
                            state.status_line = "demo stopped".to_string();
                        } else {
                            engine.start().await?;
                            state.status_line = "demo started".to_string();
                        }
                    }
                    KeyCode::Char('r') | KeyCode::Char('R') => {
                        let out = export_dir
                            .clone()
                            .unwrap_or_else(|| Path::new("out").to_path_buf())
                            .join(runtime.run_id());
                        let files = engine.generate_latest_cards(&out).await?;
                        if files.is_empty() {
                            state.status_line = "no receipt available yet".to_string();
                        } else {
                            state.status_line =
                                format!("receipt card generated: {}", out.display());
                        }
                    }
                    KeyCode::Char('v') | KeyCode::Char('V') => {
                        match engine.verify_latest_receipt().await {
                            Ok(true) => {
                                state.status_line = "receipt verification: VERIFIED".to_string()
                            }
                            Ok(false) => {
                                state.status_line = "receipt verification: no receipt".to_string()
                            }
                            Err(err) => state.status_line = format!("verify failed: {}", err),
                        }
                    }
                    KeyCode::Char('e') | KeyCode::Char('E') => {
                        let out = export_dir
                            .clone()
                            .unwrap_or_else(|| Path::new("out").to_path_buf());
                        let path = engine.export_bundle(&out).await?;
                        state.status_line = format!("bundle exported: {}", path.display());
                    }
                    KeyCode::Up => {
                        if state.selected_receipt > 0 {
                            state.selected_receipt -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if state.selected_receipt + 1 < receipts.len() {
                            state.selected_receipt += 1;
                        }
                    }
                    _ => {}
                }
            }
        }
    };

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

#[allow(clippy::too_many_arguments)]
fn draw_ui(
    frame: &mut Frame<'_>,
    worldline_id: &str,
    maple_version: &str,
    run_id: &str,
    agents: &[openibank_maple::AgentSnapshot],
    worldline_log: &[openibank_maple::WorldlineEventRecord],
    receipts: &[Receipt],
    selected_receipt: usize,
    status_line: &str,
    running: bool,
) {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(2),
        ])
        .split(frame.area());

    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " OpenIBank v0.1.0 ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(
            " Maple v{} | mode=local-sim | run_id={} | worldline={}",
            maple_version, run_id, worldline_id
        )),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(header, vertical[0]);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(24),
            Constraint::Percentage(46),
            Constraint::Percentage(30),
        ])
        .split(vertical[1]);

    render_agents(frame, body[0], agents);
    render_worldline_log(frame, body[1], worldline_log);
    render_receipts(frame, body[2], receipts, selected_receipt);

    let footer = Paragraph::new(format!(
        "S start/stop | R generate receipt card | V verify receipt | E export bundle | Q quit   [{}] {}",
        if running { "running" } else { "stopped" },
        status_line
    ))
    .block(Block::default().borders(Borders::ALL).title("Hotkeys"));
    frame.render_widget(footer, vertical[2]);
}

fn render_agents(frame: &mut Frame<'_>, area: Rect, agents: &[openibank_maple::AgentSnapshot]) {
    let items: Vec<ListItem<'_>> = agents
        .iter()
        .map(|agent| {
            let line = format!(
                "{} | bal={} | permits={} | last={}",
                agent.agent_id,
                agent.balance,
                agent.permits_count,
                agent
                    .last_worldline_event
                    .as_deref()
                    .unwrap_or("-")
                    .split(':')
                    .next_back()
                    .unwrap_or("-")
            );
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Agents"));
    frame.render_widget(list, area);
}

fn render_worldline_log(
    frame: &mut Frame<'_>,
    area: Rect,
    worldline_log: &[openibank_maple::WorldlineEventRecord],
) {
    let items: Vec<ListItem<'_>> = worldline_log
        .iter()
        .map(|event| {
            ListItem::new(format!(
                "{} | {} | {} | {}",
                event.event_id,
                event.agent_id,
                event.event_type,
                short_hash(&event.hash)
            ))
        })
        .collect();
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("WorldLine Log"),
    );
    frame.render_widget(list, area);
}

fn render_receipts(frame: &mut Frame<'_>, area: Rect, receipts: &[Receipt], selected: usize) {
    let split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
        .split(area);

    let items: Vec<ListItem<'_>> = receipts
        .iter()
        .enumerate()
        .map(|(idx, receipt)| {
            let marker = if idx == selected { ">" } else { " " };
            ListItem::new(format!(
                "{} {} | {} | {} -> {}",
                marker, receipt.tx_id, receipt.amount, receipt.from, receipt.to
            ))
        })
        .collect();
    frame.render_widget(
        List::new(items).block(Block::default().borders(Borders::ALL).title("Receipts")),
        split[0],
    );

    let details = receipts
        .get(selected)
        .map(receipt_details)
        .unwrap_or_else(|| "No receipts yet".to_string());
    frame.render_widget(
        Paragraph::new(details).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Receipt Detail"),
        ),
        split[1],
    );
}

fn receipt_details(receipt: &Receipt) -> String {
    format!(
        "tx_id: {}\nverified: {}\namount: {}\nfrom->to: {} -> {}\npermit: {}\ncommitment: {}\nwll: {}\nsig: {}",
        receipt.tx_id,
        if receipt.verify().is_ok() { "yes" } else { "no" },
        receipt.amount,
        receipt.from,
        receipt.to,
        receipt.permit_id,
        receipt.commitment_id,
        receipt.worldline_pointer(),
        short_hash(&receipt.receipt_sig),
    )
}

fn short_hash(hash: &str) -> String {
    if hash.len() > 16 {
        format!("{}...", &hash[..16])
    } else {
        hash.to_string()
    }
}
