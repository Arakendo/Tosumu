//! Bounded Rust/WASM command-window evidence for the raw-byte inspection lab.
//!
//! This is deliberately not a browser terminal emulator and it does not run
//! Tosumu Command Language. The session owns only a small local command profile
//! over facts that the raw-byte inspection boundary has already disclosed.

use crate::ratatui_projection::{response_lines, snapshot_from_buffer, RatatuiSnapshot};
use ratatui::{
    backend::TestBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
};
use std::cell::RefCell;
use tosumu_inspection_boundary::InspectionBytesResponse;

const MAX_PROMPT_BYTES: usize = 512;
const MAX_TRANSCRIPT_LINES: usize = 96;
const MAX_HISTORY_ENTRIES: usize = 32;

thread_local! {
    static ACTIVE_TERMINAL: RefCell<Option<CommandSession>> = const { RefCell::new(None) };
}

#[derive(Clone)]
struct CommandSession {
    response: InspectionBytesResponse,
    transcript: Vec<String>,
    prompt: String,
    history: Vec<String>,
    history_index: Option<usize>,
    scroll_offset: u16,
}

pub(crate) fn start(response: InspectionBytesResponse) {
    ACTIVE_TERMINAL.with(|active| {
        *active.borrow_mut() = Some(CommandSession {
            response,
            transcript: vec![
                "[system] browser-safe inspection terminal ready".to_owned(),
                "[hint] HELP, STATUS, and CLEAR are available in this profile".to_owned(),
                "[boundary] TQL and native storage commands are unavailable here".to_owned(),
            ],
            prompt: String::new(),
            history: Vec::new(),
            history_index: None,
            scroll_offset: 0,
        });
    });
}

pub(crate) fn interact(event: &str, width: u16, height: u16) -> Result<RatatuiSnapshot, String> {
    ACTIVE_TERMINAL.with(|active| {
        let mut active = active.borrow_mut();
        let session = active
            .as_mut()
            .ok_or_else(|| "no Ratatui command session is active".to_owned())?;
        apply_event(session, event)?;
        render(session, width, height)
    })
}

fn apply_event(session: &mut CommandSession, event: &str) -> Result<(), String> {
    if let Some(text) = event.strip_prefix("text:") {
        append_text(session, text)?;
        return Ok(());
    }

    match event {
        "key:backspace" => {
            session.prompt.pop();
            session.history_index = None;
        }
        "key:enter" => submit(session),
        "key:arrow-up" => recall_previous(session),
        "key:arrow-down" => recall_next(session),
        "key:scroll-up" => session.scroll_offset = session.scroll_offset.saturating_sub(1),
        "key:scroll-down" => session.scroll_offset = session.scroll_offset.saturating_add(1),
        "key:page-up" => session.scroll_offset = session.scroll_offset.saturating_sub(4),
        "key:page-down" => session.scroll_offset = session.scroll_offset.saturating_add(4),
        "key:home" => session.scroll_offset = 0,
        "key:end" => session.scroll_offset = u16::MAX,
        "key:escape" => {
            session.prompt.clear();
            session.history_index = None;
        }
        _ => return Err(format!("unsupported Ratatui terminal event: {event}")),
    }
    Ok(())
}

fn append_text(session: &mut CommandSession, text: &str) -> Result<(), String> {
    if text.chars().any(|character| character.is_control()) {
        return Err("terminal text event contains a control character".to_owned());
    }
    if session.prompt.len().saturating_add(text.len()) > MAX_PROMPT_BYTES {
        return Err(format!("terminal prompt exceeds {MAX_PROMPT_BYTES} bytes"));
    }
    session.prompt.push_str(text);
    session.history_index = None;
    Ok(())
}

fn submit(session: &mut CommandSession) {
    let command = session.prompt.trim().to_owned();
    session.prompt.clear();
    session.history_index = None;
    if command.is_empty() {
        return;
    }

    if session.history.last() != Some(&command) {
        session.history.push(command.clone());
        if session.history.len() > MAX_HISTORY_ENTRIES {
            session.history.remove(0);
        }
    }
    push_line(session, format!("> {command}"));

    match command.to_ascii_uppercase().as_str() {
        "HELP" => {
            push_line(
                session,
                "local profile commands: HELP | STATUS | CLEAR".to_owned(),
            );
            push_line(session, "TQL is unavailable: raw uploaded bytes do not provide a reviewed native store session.".to_owned());
        }
        "STATUS" => {
            let (title, lines, _) = response_lines(&session.response);
            push_line(session, format!("[{title}]"));
            for line in lines {
                push_line(session, line);
            }
        }
        "CLEAR" => {
            session.transcript.clear();
            push_line(
                session,
                "[system] transcript cleared; inspection response remains active".to_owned(),
            );
        }
        _ => push_line(
            session,
            format!(
                "[unsupported] {command}; browser-safe profile supports HELP, STATUS, and CLEAR"
            ),
        ),
    }
    session.scroll_offset = u16::MAX;
}

fn push_line(session: &mut CommandSession, line: String) {
    session.transcript.push(line);
    if session.transcript.len() > MAX_TRANSCRIPT_LINES {
        session.transcript.remove(0);
    }
}

fn recall_previous(session: &mut CommandSession) {
    if session.history.is_empty() {
        return;
    }
    let next = session
        .history_index
        .unwrap_or(session.history.len())
        .saturating_sub(1);
    session.history_index = Some(next);
    session.prompt.clone_from(&session.history[next]);
}

fn recall_next(session: &mut CommandSession) {
    let Some(index) = session.history_index else {
        return;
    };
    if index + 1 >= session.history.len() {
        session.history_index = None;
        session.prompt.clear();
    } else {
        let next = index + 1;
        session.history_index = Some(next);
        session.prompt.clone_from(&session.history[next]);
    }
}

fn render(
    session: &mut CommandSession,
    width: u16,
    height: u16,
) -> Result<RatatuiSnapshot, String> {
    if width < 40 || height < 12 {
        return Err(format!(
            "Ratatui command projection requires at least 40x12 cells, received {width}x{height}"
        ));
    }

    let backend = TestBackend::new(width, height);
    let mut terminal =
        Terminal::new(backend).map_err(|error| format!("create Ratatui TestBackend: {error}"))?;
    // The prompt is a bordered paragraph and needs an interior row. A
    // two-cell allocation only draws its top and bottom borders, which hides
    // accepted input even though the session state updated correctly.
    let transcript_height = height.saturating_sub(7).max(1);
    let maximum_scroll = session
        .transcript
        .len()
        .saturating_sub(transcript_height as usize) as u16;
    if session.scroll_offset == u16::MAX {
        session.scroll_offset = maximum_scroll;
    } else {
        session.scroll_offset = session.scroll_offset.min(maximum_scroll);
    }
    let transcript = session.transcript.clone();
    let prompt = session.prompt.clone();
    let scroll = session.scroll_offset;

    terminal
        .draw(|frame| {
            let regions = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Min(6),
                    Constraint::Length(3),
                    Constraint::Length(1),
                ])
                .split(frame.area());
            frame.render_widget(
                Paragraph::new("TOSUMU INSPECTION / BROWSER-SAFE RATATUI SESSION").style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                regions[0],
            );
            frame.render_widget(
                Paragraph::new(transcript.into_iter().map(Line::from).collect::<Vec<_>>())
                    .block(Block::default().borders(Borders::ALL).title("TRANSCRIPT"))
                    .style(Style::default().fg(Color::Green))
                    .scroll((scroll, 0))
                    .wrap(Wrap { trim: false }),
                regions[1],
            );
            frame.render_widget(
                Paragraph::new(format!("> {prompt}_"))
                    .block(Block::default().borders(Borders::ALL).title("PROMPT"))
                    .style(Style::default().fg(Color::Cyan)),
                regions[2],
            );
            frame.render_widget(
                Paragraph::new(
                    "Enter submits | Up/Down history | Wheel/Page keys review | Esc clears prompt",
                )
                .style(Style::default().fg(Color::DarkGray)),
                regions[3],
            );
        })
        .map_err(|error| format!("draw Ratatui command projection: {error}"))?;

    Ok(snapshot_from_buffer(
        terminal.backend().buffer(),
        width,
        height,
    ))
}

#[cfg(test)]
pub(crate) fn rendered_text(snapshot: &RatatuiSnapshot) -> String {
    snapshot
        .cells
        .iter()
        .map(|cell| cell.symbol.as_str())
        .collect()
}

#[cfg(test)]
pub(crate) fn rendered_lines(snapshot: &RatatuiSnapshot) -> Vec<String> {
    (0..snapshot.height)
        .map(|y| {
            snapshot
                .cells
                .iter()
                .filter(|cell| cell.y == y)
                .map(|cell| cell.symbol.as_str())
                .collect::<String>()
        })
        .collect()
}
