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
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Gauge};
use ratatui::{Frame, Terminal};

// ── Bloomberg-dark palette ──────────────────────────────────────────────────
const BLOOMBERG_CYAN: Color    = Color::Rgb(0, 212, 255);   // #00d4ff — headers/IDs
const BLOOMBERG_GREEN: Color   = Color::Rgb(0, 230, 118);   // #00e676 — positive/verified
const BLOOMBERG_AMBER: Color   = Color::Rgb(255, 171, 64);  // #ffab40 — warnings/pending
const BLOOMBERG_DIM: Color     = Color::Rgb(74, 85, 104);   // #4a5568 — dimmed text
const BLOOMBERG_TEXT: Color    = Color::Rgb(200, 208, 224); // #c8d0e0 — normal text
const BLOOMBERG_BG: Color      = Color::Rgb(10, 14, 26);    // #0a0e1a — background
const BLOOMBERG_HEADER_BG: Color = Color::Rgb(15, 23, 41); // #0f1729 — header bg
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
            Constraint::Length(3),   // header
            Constraint::Min(10),     // body
            Constraint::Length(3),   // footer
        ])
        .split(frame.area());

    // ── Bloomberg-dark header bar ────────────────────────────────────────────
    let status_color = if running { BLOOMBERG_GREEN } else { BLOOMBERG_AMBER };
    let status_tag = if running { "● LIVE" } else { "○ PAUSED" };
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " ◈ OpeniBank ",
            Style::default().fg(BLOOMBERG_CYAN).bg(BLOOMBERG_HEADER_BG).add_modifier(Modifier::BOLD),
        ),
        Span::styled("v2.0", Style::default().fg(BLOOMBERG_DIM).bg(BLOOMBERG_HEADER_BG)),
        Span::styled(
            format!("  {}  ", status_tag),
            Style::default().fg(status_color).bg(BLOOMBERG_HEADER_BG).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("│ Maple {} │ sim:{} │ wl:{}", maple_version, run_id, short_hash(worldline_id)),
            Style::default().fg(BLOOMBERG_DIM).bg(BLOOMBERG_HEADER_BG),
        ),
    ]))
    .style(Style::default().bg(BLOOMBERG_HEADER_BG))
    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(BLOOMBERG_DIM)));
    frame.render_widget(header, vertical[0]);

    // ── Body: 3-column layout ───────────────────────────────────────────────
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(22),  // agents
            Constraint::Percentage(44),  // worldline log
            Constraint::Percentage(34),  // receipts
        ])
        .split(vertical[1]);

    render_agents(frame, body[0], agents);
    render_worldline_log(frame, body[1], worldline_log);
    render_receipts(frame, body[2], receipts, selected_receipt);

    // ── Footer: status bar ──────────────────────────────────────────────────
    let hotkeys = Line::from(vec![
        Span::styled("[S]", Style::default().fg(BLOOMBERG_CYAN).add_modifier(Modifier::BOLD)),
        Span::styled(" start/stop  ", Style::default().fg(BLOOMBERG_TEXT)),
        Span::styled("[R]", Style::default().fg(BLOOMBERG_CYAN).add_modifier(Modifier::BOLD)),
        Span::styled(" gen card  ", Style::default().fg(BLOOMBERG_TEXT)),
        Span::styled("[V]", Style::default().fg(BLOOMBERG_CYAN).add_modifier(Modifier::BOLD)),
        Span::styled(" verify  ", Style::default().fg(BLOOMBERG_TEXT)),
        Span::styled("[E]", Style::default().fg(BLOOMBERG_CYAN).add_modifier(Modifier::BOLD)),
        Span::styled(" export  ", Style::default().fg(BLOOMBERG_TEXT)),
        Span::styled("[Q]", Style::default().fg(BLOOMBERG_AMBER).add_modifier(Modifier::BOLD)),
        Span::styled(" quit   ", Style::default().fg(BLOOMBERG_TEXT)),
        Span::styled("▶ ", Style::default().fg(BLOOMBERG_DIM)),
        Span::styled(status_line, Style::default().fg(BLOOMBERG_GREEN)),
    ]);
    let footer = Paragraph::new(hotkeys)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(Span::styled("Controls", Style::default().fg(BLOOMBERG_DIM)))
            .border_style(Style::default().fg(BLOOMBERG_DIM)));
    frame.render_widget(footer, vertical[2]);
}

fn render_agents(frame: &mut Frame<'_>, area: Rect, agents: &[openibank_maple::AgentSnapshot]) {
    let items: Vec<ListItem<'_>> = agents
        .iter()
        .map(|agent| {
            let last = agent.last_worldline_event.as_deref().unwrap_or("-");
            let last_short = last.split(':').next_back().unwrap_or("-");
            let line = Line::from(vec![
                Span::styled(
                    format!("{:<14}", &agent.agent_id[..agent.agent_id.len().min(14)]),
                    Style::default().fg(BLOOMBERG_CYAN),
                ),
                Span::styled(
                    format!(" ${:<10}", agent.balance),
                    Style::default().fg(BLOOMBERG_GREEN),
                ),
                Span::styled(
                    format!(" p:{} ", agent.permits_count),
                    Style::default().fg(BLOOMBERG_TEXT),
                ),
                Span::styled(
                    short_hash(last_short),
                    Style::default().fg(BLOOMBERG_DIM),
                ),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(Span::styled("◈ Agents", Style::default().fg(BLOOMBERG_CYAN).add_modifier(Modifier::BOLD)))
            .border_style(Style::default().fg(BLOOMBERG_DIM)))
        .highlight_style(Style::default().bg(BLOOMBERG_HEADER_BG).add_modifier(Modifier::BOLD));
    frame.render_widget(list, area);
}

fn render_worldline_log(
    frame: &mut Frame<'_>,
    area: Rect,
    worldline_log: &[openibank_maple::WorldlineEventRecord],
) {
    let items: Vec<ListItem<'_>> = worldline_log
        .iter()
        .enumerate()
        .map(|(idx, event)| {
            // Alternate row shading with event type color
            let type_color = match event.event_type.as_str() {
                s if s.contains("Consequence") => BLOOMBERG_GREEN,
                s if s.contains("Commitment") => BLOOMBERG_AMBER,
                s if s.contains("Intent")     => BLOOMBERG_CYAN,
                _                             => BLOOMBERG_TEXT,
            };
            let line = Line::from(vec![
                Span::styled(
                    format!("{:04} ", idx),
                    Style::default().fg(BLOOMBERG_DIM),
                ),
                Span::styled(
                    format!("{:<14} ", &event.agent_id[..event.agent_id.len().min(14)]),
                    Style::default().fg(BLOOMBERG_CYAN),
                ),
                Span::styled(
                    format!("{:<18} ", &event.event_type[..event.event_type.len().min(18)]),
                    Style::default().fg(type_color),
                ),
                Span::styled(
                    short_hash(&event.hash),
                    Style::default().fg(BLOOMBERG_DIM),
                ),
            ]);
            ListItem::new(line)
        })
        .collect();
    let list = List::new(items)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(Span::styled("◈ WorldLine", Style::default().fg(BLOOMBERG_AMBER).add_modifier(Modifier::BOLD)))
            .border_style(Style::default().fg(BLOOMBERG_DIM)));
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
            let is_sel = idx == selected;
            let verified = receipt.verify().is_ok();
            let marker_color = if is_sel { BLOOMBERG_AMBER } else { BLOOMBERG_DIM };
            let sig_color = if verified { BLOOMBERG_GREEN } else { BLOOMBERG_AMBER };
            let line = Line::from(vec![
                Span::styled(if is_sel { "▶ " } else { "  " }, Style::default().fg(marker_color)),
                Span::styled(
                    format!("{:.16}", receipt.tx_id),
                    Style::default().fg(BLOOMBERG_CYAN),
                ),
                Span::styled(
                    format!(" {}", receipt.amount),
                    Style::default().fg(BLOOMBERG_GREEN).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    if verified { " ✓" } else { " ?" },
                    Style::default().fg(sig_color),
                ),
            ]);
            ListItem::new(line)
        })
        .collect();
    frame.render_widget(
        List::new(items).block(Block::default()
            .borders(Borders::ALL)
            .title(Span::styled("◈ Receipts", Style::default().fg(BLOOMBERG_GREEN).add_modifier(Modifier::BOLD)))
            .border_style(Style::default().fg(BLOOMBERG_DIM))),
        split[0],
    );

    let details = receipts
        .get(selected)
        .map(receipt_details)
        .unwrap_or_else(|| "No receipts yet.\n\nPress S to start the demo\nor wait for transactions.".to_string());
    frame.render_widget(
        Paragraph::new(details)
            .style(Style::default().fg(BLOOMBERG_TEXT))
            .block(Block::default()
                .borders(Borders::ALL)
                .title(Span::styled("Receipt Detail", Style::default().fg(BLOOMBERG_DIM)))
                .border_style(Style::default().fg(BLOOMBERG_DIM))),
        split[1],
    );
}

fn receipt_details(receipt: &Receipt) -> String {
    let verified = receipt.verify().is_ok();
    format!(
        "ID:     {}\nSTATUS: {}\nAMOUNT: {}\nFROM:   {}\nTO:     {}\nPERMIT: {}\nCOMMIT: {}\nWLL:    {}\nSIG:    {}\nTIME:   {}",
        receipt.tx_id,
        if verified { "✓ VERIFIED" } else { "✗ UNSIGNED" },
        receipt.amount,
        receipt.from,
        receipt.to,
        short_hash(&receipt.permit_id),
        short_hash(&receipt.commitment_id),
        receipt.worldline_pointer(),
        short_hash(&receipt.receipt_sig),
        receipt.timestamp.format("%Y-%m-%dT%H:%M:%SZ"),
    )
}

fn short_hash(hash: &str) -> String {
    if hash.len() > 16 {
        format!("{}...", &hash[..16])
    } else {
        hash.to_string()
    }
}
