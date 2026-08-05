//! Headless Ratatui evidence for the public inspection boundary.
//!
//! This module is intentionally an adapter. It consumes the already resolved
//! Tosumu observation/failure DTO and has no page decoding, host-path, or
//! terminal-backend authority of its own.

use ratatui::{
    backend::TestBackend,
    buffer::Buffer,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
};
use serde::Serialize;
use std::cell::RefCell;
use tosumu_inspection_boundary::InspectionBytesResponse;

thread_local! {
    static ACTIVE_SESSION: RefCell<Option<ProjectionSession>> = const { RefCell::new(None) };
}

#[derive(Clone)]
struct ProjectionSession {
    response: InspectionBytesResponse,
    scroll_offset: u16,
}

#[derive(Debug, Serialize)]
pub(crate) struct RatatuiSnapshot {
    pub schema_version: u16,
    pub width: u16,
    pub height: u16,
    pub cells: Vec<CellEvidence>,
}

#[derive(Debug, Serialize)]
pub(crate) struct CellEvidence {
    pub x: u16,
    pub y: u16,
    pub symbol: String,
    pub foreground: String,
    pub background: String,
    pub modifiers: Vec<String>,
}

pub(crate) fn render_inspection(
    response: &InspectionBytesResponse,
    width: u16,
    height: u16,
) -> Result<RatatuiSnapshot, String> {
    render_inspection_at(response, width, height, 0)
}

/// Starts a provider-local terminal session for an already resolved response.
/// The response never changes here; interaction only changes the viewport.
pub(crate) fn start_session(response: InspectionBytesResponse) {
    ACTIVE_SESSION.with(|session| {
        *session.borrow_mut() = Some(ProjectionSession {
            response,
            scroll_offset: 0,
        });
    });
}

/// Applies normalized browser navigation to the headless Ratatui viewport.
pub(crate) fn interact(event: &str, width: u16, height: u16) -> Result<RatatuiSnapshot, String> {
    ACTIVE_SESSION.with(|session| {
        let mut session = session.borrow_mut();
        let active = session
            .as_mut()
            .ok_or_else(|| "no Ratatui inspection session is active".to_owned())?;
        let line_count = response_lines(&active.response).1.len() as u16;
        let viewport_height = height.saturating_sub(5).max(1);
        let maximum = line_count.saturating_sub(viewport_height);
        match event {
            "scroll-up" => active.scroll_offset = active.scroll_offset.saturating_sub(1),
            "scroll-down" => active.scroll_offset = (active.scroll_offset + 1).min(maximum),
            "page-up" => {
                active.scroll_offset = active.scroll_offset.saturating_sub(viewport_height)
            }
            "page-down" => {
                active.scroll_offset = (active.scroll_offset + viewport_height).min(maximum)
            }
            "home" => active.scroll_offset = 0,
            "end" => active.scroll_offset = maximum,
            _ => return Err(format!("unsupported Ratatui navigation event: {event}")),
        }
        render_inspection_at(&active.response, width, height, active.scroll_offset)
    })
}

fn render_inspection_at(
    response: &InspectionBytesResponse,
    width: u16,
    height: u16,
    scroll_offset: u16,
) -> Result<RatatuiSnapshot, String> {
    if width < 32 || height < 10 {
        return Err(format!(
            "Ratatui inspection projection requires at least 32x10 cells, received {width}x{height}"
        ));
    }

    let backend = TestBackend::new(width, height);
    let mut terminal =
        Terminal::new(backend).map_err(|error| format!("create Ratatui TestBackend: {error}"))?;
    let (summary, details, accent) = response_lines(response);

    terminal
        .draw(|frame| {
            let regions = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Min(6),
                    Constraint::Length(2),
                ])
                .split(frame.area());
            frame.render_widget(
                Paragraph::new("TOSUMU INSPECTION / RATATUI TESTBACKEND").style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                regions[0],
            );
            frame.render_widget(
                Paragraph::new(details.into_iter().map(Line::from).collect::<Vec<_>>())
                    .block(Block::default().borders(Borders::ALL).title(summary))
                    .style(Style::default().fg(accent))
                    .scroll((scroll_offset, 0))
                    .wrap(Wrap { trim: false }),
                regions[1],
            );
            frame.render_widget(
                Paragraph::new(
                    "Arrows/wheel scroll terminal view. Semantic DTO remains authoritative.",
                )
                .style(Style::default().fg(Color::DarkGray)),
                regions[2],
            );
        })
        .map_err(|error| format!("draw Ratatui inspection projection: {error}"))?;

    Ok(snapshot_from_buffer(
        terminal.backend().buffer(),
        width,
        height,
    ))
}

pub(crate) fn snapshot_from_buffer(buffer: &Buffer, width: u16, height: u16) -> RatatuiSnapshot {
    let cells = buffer
        .content
        .iter()
        .enumerate()
        .map(|(index, cell)| CellEvidence {
            x: (index % width as usize) as u16,
            y: (index / width as usize) as u16,
            symbol: cell.symbol().to_owned(),
            foreground: format!("{:?}", cell.fg),
            background: format!("{:?}", cell.bg),
            modifiers: modifier_names(cell.modifier),
        })
        .collect();

    RatatuiSnapshot {
        schema_version: 1,
        width,
        height,
        cells,
    }
}

pub(crate) fn response_lines(response: &InspectionBytesResponse) -> (String, Vec<String>, Color) {
    match response {
        InspectionBytesResponse::Observation {
            schema_version,
            observation,
        } => (
            "HEADER ONLY / OBSERVATION".to_owned(),
            vec![
                format!("boundary schema: {schema_version}"),
                format!("input bytes: {}", observation.source.byte_count),
                format!("format version: {}", observation.header.format_version),
                format!("page size: {}", observation.header.page_size),
                format!("declared pages: {}", observation.header.page_count),
                format!("root page: {}", observation.header.root_page),
                format!("retained page rows: {}", observation.pages.retained),
                format!("source kind: {}", observation.source.kind),
                format!("unlock state: {}", observation.source.unlock_state),
                format!("format flags: {}", observation.header.flags),
                format!("declared keyslots: {}", observation.header.keyslot_count),
                "tree / WAL / keyslots: unavailable by raw-byte policy".to_owned(),
                "viewport controls: arrows, page up/down, home/end, wheel".to_owned(),
            ],
            Color::Green,
        ),
        InspectionBytesResponse::Failure {
            schema_version,
            error,
        } => (
            "EXPLICIT BOUNDARY REJECTION".to_owned(),
            vec![
                format!("boundary schema: {schema_version}"),
                format!("code: {}", error.code),
                format!("status: {}", error.status),
                format!("message: {}", error.message),
                "no storage facts were inferred from rejected bytes".to_owned(),
            ],
            Color::Red,
        ),
    }
}

fn modifier_names(modifiers: Modifier) -> Vec<String> {
    [(Modifier::BOLD, "BOLD"), (Modifier::DIM, "DIM")]
        .into_iter()
        .filter(|(flag, _)| modifiers.contains(*flag))
        .map(|(_, name)| name.to_owned())
        .collect()
}
