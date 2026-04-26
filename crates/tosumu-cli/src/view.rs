use std::path::Path;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Terminal;
use tosumu_core::error::TosumuError;
use tosumu_core::format::{PAGE_TYPE_FREE, PAGE_TYPE_INTERNAL, PAGE_TYPE_LEAF, PAGE_TYPE_OVERFLOW};
use tosumu_core::inspect::{inspect_page_from_pager, read_header_info, verify_pager, HeaderInfo, PageSummary, RecordInfo, VerifyReport};
use tosumu_core::pager::Pager;

use crate::error_boundary::CliError;
use crate::unlock::open_pager;

pub fn run(path: &Path) -> Result<(), CliError> {
    let header = read_header_info(path)?;
    let (pager, _) = open_pager(path)?;
    let verify = verify_pager(&pager)?;
    let pages = load_page_rows(&pager)?;
    let mut app = ViewApp::new(path, header, verify, pages);
    app.select_first(&pager)?;

    enable_raw_mode().map_err(TosumuError::Io)?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen).map_err(TosumuError::Io)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(TosumuError::Io)?;

    let run_result = run_loop(&mut terminal, &pager, &mut app);

    let mut restore_error = None;
    if let Err(error) = disable_raw_mode().map_err(TosumuError::Io) {
        restore_error = Some(error);
    }
    if let Err(error) = execute!(terminal.backend_mut(), LeaveAlternateScreen).map_err(TosumuError::Io) {
        restore_error = restore_error.or(Some(error));
    }
    if let Err(error) = terminal.show_cursor().map_err(TosumuError::Io) {
        restore_error = restore_error.or(Some(error));
    }

    match (run_result, restore_error) {
        (Err(error), _) => Err(error),
        (Ok(()), Some(error)) => Err(error.into()),
        (Ok(()), None) => Ok(()),
    }
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    pager: &Pager,
    app: &mut ViewApp,
) -> Result<(), CliError> {
    loop {
        terminal.draw(|frame| draw(frame, app)).map_err(TosumuError::Io)?;

        if !event::poll(Duration::from_millis(200)).map_err(TosumuError::Io)? {
            continue;
        }

        let Event::Key(key) = event::read().map_err(TosumuError::Io)? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
            KeyCode::Down | KeyCode::Char('j') => app.select_next(pager)?,
            KeyCode::Up | KeyCode::Char('k') => app.select_previous(pager)?,
            KeyCode::Home | KeyCode::Char('g') => app.select_first(pager)?,
            KeyCode::End | KeyCode::Char('G') => app.select_last(pager)?,
            _ => {}
        }
    }
}

fn draw(frame: &mut ratatui::Frame<'_>, app: &ViewApp) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(12),
            Constraint::Length(2),
        ])
        .split(frame.area());

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(40), Constraint::Min(40)])
        .split(root[1]);

    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(12),
            Constraint::Length(6),
            Constraint::Min(8),
        ])
        .split(body[1]);

    frame.render_widget(title_widget(app), root[0]);
    frame.render_stateful_widget(page_list_widget(app), body[0], &mut app.list_state());
    frame.render_widget(header_widget(app), right[0]);
    frame.render_widget(verify_widget(app), right[1]);
    frame.render_widget(detail_widget(app), right[2]);
    frame.render_widget(help_widget(), root[2]);
}

fn title_widget(app: &ViewApp) -> Paragraph<'static> {
    let text = Line::from(vec![
        Span::styled("tosumu view", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::raw(app.path.display().to_string()),
    ]);
    Paragraph::new(text).block(Block::default().borders(Borders::ALL))
}

fn page_list_widget(app: &ViewApp) -> List<'static> {
    let items = app
        .pages
        .iter()
        .map(|page| {
            let summary = format!(
                "{:>4}  {:<8}  v{:>3}  slots {:>3}",
                page.pgno,
                page_type_label(page.page_type),
                page.page_version,
                page.slot_count,
            );
            ListItem::new(Line::from(summary))
        })
        .collect::<Vec<_>>();

    List::new(items)
        .block(Block::default().title("Pages").borders(Borders::ALL))
        .highlight_style(Style::default().bg(Color::Blue).fg(Color::Black).add_modifier(Modifier::BOLD))
        .highlight_symbol("> ")
}

fn header_widget(app: &ViewApp) -> Paragraph<'static> {
    let header = &app.header;
    let lines = vec![
        Line::from(format!("format_version:       {}", header.format_version)),
        Line::from(format!("page_size:            {}", header.page_size)),
        Line::from(format!("page_count:           {}", header.page_count)),
        Line::from(format!("root_page:            {}", header.root_page)),
        Line::from(format!("freelist_head:        {}", header.freelist_head)),
        Line::from(format!("wal_checkpoint_lsn:   {}", header.wal_checkpoint_lsn)),
        Line::from(format!("dek_id:               {}", header.dek_id)),
        Line::from(format!("keyslot_count:        {}", header.keyslot_count)),
        Line::from(format!("keyslot_region_pages: {}", header.keyslot_region_pages)),
        Line::from(format!("slot0_kind:           {}", keyslot_kind_label(header.ks0_kind))),
    ];

    Paragraph::new(Text::from(lines))
        .block(Block::default().title("Header").borders(Borders::ALL))
        .wrap(Wrap { trim: false })
}

fn verify_widget(app: &ViewApp) -> Paragraph<'static> {
    let lines = vec![
        Line::from(format!("pages_checked: {}", app.verify.pages_checked)),
        Line::from(format!("pages_ok:      {}", app.verify.pages_ok)),
        Line::from(format!("issues:        {}", app.verify.issues.len())),
        Line::from(if app.verify.issues.is_empty() {
            "status:        clean".to_string()
        } else {
            format!("status:        {} issue(s)", app.verify.issues.len())
        }),
    ];

    Paragraph::new(Text::from(lines))
        .block(Block::default().title("Verify").borders(Borders::ALL))
        .wrap(Wrap { trim: false })
}

fn detail_widget(app: &ViewApp) -> Paragraph<'static> {
    let mut lines = Vec::new();
    match &app.selected_detail {
        Some(detail) => {
            lines.push(Line::from(format!("page:         {}", detail.pgno)));
            lines.push(Line::from(format!("type:         {}", page_type_label(detail.page_type))));
            lines.push(Line::from(format!("page_version: {}", detail.page_version)));
            lines.push(Line::from(format!("slot_count:   {}", detail.slot_count)));
            lines.push(Line::from(format!("free_start:   {}", detail.free_start)));
            lines.push(Line::from(format!("free_end:     {}", detail.free_end)));
            lines.push(Line::from(""));

            if detail.records.is_empty() {
                lines.push(Line::from("(no decoded records)"));
            } else {
                for (index, record) in detail.records.iter().enumerate().take(12) {
                    lines.push(Line::from(format!("slot {index:>2}: {}", record_summary(record))));
                }
                if detail.records.len() > 12 {
                    lines.push(Line::from(format!("... {} more record(s)", detail.records.len() - 12)));
                }
            }
        }
        None => lines.push(Line::from("page 0 is the file header; no data pages to inspect yet")),
    }

    Paragraph::new(Text::from(lines))
        .block(Block::default().title("Page Detail").borders(Borders::ALL))
        .wrap(Wrap { trim: false })
}

fn help_widget() -> Paragraph<'static> {
    Paragraph::new("Up/Down or j/k to move • g/G for first/last • q or Esc to quit")
        .block(Block::default().borders(Borders::ALL))
}

fn load_page_rows(pager: &Pager) -> Result<Vec<PageRow>, CliError> {
    let mut rows = Vec::new();
    for pgno in 1..pager.page_count() {
        let summary = inspect_page_from_pager(pager, pgno)?;
        rows.push(PageRow {
            pgno,
            page_type: summary.page_type,
            page_version: summary.page_version,
            slot_count: summary.slot_count,
        });
    }
    Ok(rows)
}

fn page_type_label(page_type: u8) -> &'static str {
    match page_type {
        PAGE_TYPE_LEAF => "Leaf",
        PAGE_TYPE_INTERNAL => "Internal",
        PAGE_TYPE_OVERFLOW => "Overflow",
        PAGE_TYPE_FREE => "Free",
        _ => "Unknown",
    }
}

fn keyslot_kind_label(kind: u8) -> &'static str {
    match kind {
        0 => "Empty",
        1 => "Sentinel",
        2 => "Passphrase",
        3 => "RecoveryKey",
        4 => "Keyfile",
        _ => "Unknown",
    }
}

fn record_summary(record: &RecordInfo) -> String {
    match record {
        RecordInfo::Live { key, value } => format!(
            "live key={} value={}",
            preview_bytes(key),
            preview_bytes(value)
        ),
        RecordInfo::Tombstone { key } => format!("tombstone key={}", preview_bytes(key)),
        RecordInfo::Unknown { slot, record_type } => {
            format!("unknown slot={slot} record_type=0x{record_type:02x}")
        }
    }
}

fn preview_bytes(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(text) => {
            let shortened = text.chars().take(24).collect::<String>();
            if text.chars().count() > 24 {
                format!("{shortened:?}...")
            } else {
                format!("{shortened:?}")
            }
        }
        Err(_) => {
            let hex = bytes.iter().take(16).map(|b| format!("{b:02x}")).collect::<String>();
            if bytes.len() > 16 {
                format!("0x{hex}...")
            } else {
                format!("0x{hex}")
            }
        }
    }
}

#[derive(Clone, Copy)]
struct PageRow {
    pgno: u64,
    page_type: u8,
    page_version: u64,
    slot_count: u16,
}

struct ViewApp<'a> {
    path: &'a Path,
    header: HeaderInfo,
    verify: VerifyReport,
    pages: Vec<PageRow>,
    selected: Option<usize>,
    selected_detail: Option<PageSummary>,
}

impl<'a> ViewApp<'a> {
    fn new(path: &'a Path, header: HeaderInfo, verify: VerifyReport, pages: Vec<PageRow>) -> Self {
        Self {
            path,
            header,
            verify,
            pages,
            selected: None,
            selected_detail: None,
        }
    }

    fn list_state(&self) -> ListState {
        let mut state = ListState::default();
        state.select(self.selected);
        state
    }

    fn select_first(&mut self, pager: &Pager) -> Result<(), CliError> {
        if self.pages.is_empty() {
            return Ok(());
        }
        self.selected = Some(0);
        self.refresh_selected_detail(pager)
    }

    fn select_last(&mut self, pager: &Pager) -> Result<(), CliError> {
        if self.pages.is_empty() {
            return Ok(());
        }
        self.selected = Some(self.pages.len() - 1);
        self.refresh_selected_detail(pager)
    }

    fn select_next(&mut self, pager: &Pager) -> Result<(), CliError> {
        if self.pages.is_empty() {
            return Ok(());
        }
        self.selected = Some(match self.selected {
            Some(index) if index + 1 < self.pages.len() => index + 1,
            _ => self.pages.len() - 1,
        });
        self.refresh_selected_detail(pager)
    }

    fn select_previous(&mut self, pager: &Pager) -> Result<(), CliError> {
        if self.pages.is_empty() {
            return Ok(());
        }
        self.selected = Some(match self.selected {
            Some(index) if index > 0 => index - 1,
            _ => 0,
        });
        self.refresh_selected_detail(pager)
    }

    fn refresh_selected_detail(&mut self, pager: &Pager) -> Result<(), CliError> {
        self.selected_detail = match self.selected.and_then(|index| self.pages.get(index).copied()) {
            Some(page) => Some(inspect_page_from_pager(pager, page.pgno)?),
            None => None,
        };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_type_labels_cover_known_types() {
        assert_eq!(page_type_label(PAGE_TYPE_LEAF), "Leaf");
        assert_eq!(page_type_label(PAGE_TYPE_INTERNAL), "Internal");
        assert_eq!(page_type_label(PAGE_TYPE_OVERFLOW), "Overflow");
        assert_eq!(page_type_label(PAGE_TYPE_FREE), "Free");
        assert_eq!(page_type_label(0xff), "Unknown");
    }

    #[test]
    fn keyslot_kind_labels_cover_known_types() {
        assert_eq!(keyslot_kind_label(0), "Empty");
        assert_eq!(keyslot_kind_label(1), "Sentinel");
        assert_eq!(keyslot_kind_label(2), "Passphrase");
        assert_eq!(keyslot_kind_label(3), "RecoveryKey");
        assert_eq!(keyslot_kind_label(4), "Keyfile");
        assert_eq!(keyslot_kind_label(9), "Unknown");
    }

    #[test]
    fn preview_bytes_formats_utf8_and_binary() {
        assert_eq!(preview_bytes(b"alpha"), "\"alpha\"");
        assert_eq!(preview_bytes(&[0xde, 0xad, 0xbe, 0xef]), "0xdeadbeef");
    }
}