use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};

use tosumu_core::format::{PAGE_TYPE_FREE, PAGE_TYPE_INTERNAL, PAGE_TYPE_LEAF, PAGE_TYPE_OVERFLOW};
use tosumu_core::inspect::RecordInfo;
#[cfg(test)]
use tosumu_core::inspect::{
    PageVerifyResult, VerifyIssueKind, VerifyReport, WalRecordSummary, WalRecordSummaryKind,
};
use tosumu_core::inspection_session::{
    InspectionKeyslots, InspectionSection, InspectionTree, InspectionWal, InspectionWalRecordKind,
};

use super::state::{FocusPane, PageStatus, SelectedPageDetail, ViewApp, ViewMode};

pub(super) fn draw(frame: &mut ratatui::Frame<'_>, app: &ViewApp) {
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
        .constraints([Constraint::Length(44), Constraint::Min(40)])
        .split(root[1]);

    frame.render_widget(title_widget(app), root[0]);
    frame.render_stateful_widget(page_list_widget(app), body[0], &mut app.list_state());
    frame.render_widget(panel_widget(app), body[1]);
    frame.render_widget(help_widget(app), root[2]);
}

fn title_widget(app: &ViewApp) -> Paragraph<'static> {
    let mut spans = vec![
        Span::styled(
            "tosumu view",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::raw(app.path.display().to_string()),
        Span::raw("  "),
    ];
    for (index, mode) in ViewMode::ALL.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw(" "));
        }
        let label = format!("{}:{}", index + 1, mode.label());
        let style = if *mode == app.mode {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };
        spans.push(Span::styled(label, style));
    }
    Paragraph::new(Line::from(spans)).block(Block::default().borders(Borders::ALL))
}

fn page_list_widget(app: &ViewApp) -> List<'static> {
    let items = app
        .page_list_window()
        .into_iter()
        .map(page_list_item)
        .collect::<Vec<_>>();
    let title = app.page_list_title();

    List::new(items)
        .block(focus_block(&title, app.focus == FocusPane::Pages))
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ")
}

fn panel_widget(app: &ViewApp) -> Paragraph<'static> {
    match app.mode {
        ViewMode::Header => header_widget(app),
        ViewMode::Detail => detail_widget(app),
        ViewMode::Verify => verify_widget(app),
        ViewMode::Tree => tree_widget(app),
        ViewMode::Wal => wal_widget(app),
        ViewMode::Protectors => protectors_widget(app),
    }
}

fn header_widget(app: &ViewApp) -> Paragraph<'static> {
    panel_paragraph("Header", header_lines(app), app)
}

fn verify_widget(app: &ViewApp) -> Paragraph<'static> {
    panel_paragraph("Verify", verify_lines(app), app)
}

fn detail_widget(app: &ViewApp) -> Paragraph<'static> {
    panel_paragraph("Page Detail", detail_lines(app), app)
}

fn tree_widget(app: &ViewApp) -> Paragraph<'static> {
    panel_paragraph("B+ Tree", tree_lines(app), app)
}

fn wal_widget(app: &ViewApp) -> Paragraph<'static> {
    panel_paragraph("WAL", wal_lines(app), app)
}

fn protectors_widget(app: &ViewApp) -> Paragraph<'static> {
    panel_paragraph("Protectors", protectors_lines(app), app)
}

fn help_widget(app: &ViewApp) -> Paragraph<'static> {
    let watch = if app.watch_enabled { "on" } else { "off" };
    let text = format!(
        "Tab or Left/Right switches focus • j/k and arrows act on active pane • PgUp/PgDn jumps pages or scrolls panel • / starts filter • n/N move between matches • : starts goto-page • K/J scroll panel • 1-6 or h/d/v/t/l/p for panels • current={} • focus={} • scroll={} • watch={}{}",
        app.mode.label(),
        app.focus.label(),
        app.panel_scroll,
        watch,
        app.footer_status(),
    );
    Paragraph::new(text).block(Block::default().borders(Borders::ALL))
}

fn panel_paragraph(
    title: &'static str,
    lines: Vec<Line<'static>>,
    app: &ViewApp,
) -> Paragraph<'static> {
    Paragraph::new(Text::from(lines))
        .block(focus_block(title, app.focus == FocusPane::Panel))
        .scroll((app.panel_scroll, 0))
        .wrap(Wrap { trim: false })
}

fn header_lines(app: &ViewApp) -> Vec<Line<'static>> {
    let header = &app.observation.header;
    vec![
        Line::from(format!("format_version:       {}", header.format_version)),
        Line::from(format!("page_size:            {}", header.page_size)),
        Line::from(format!("flags:                0x{:04x}", header.flags)),
        Line::from(format!("page_count:           {}", header.page_count)),
        Line::from(format!("root_page:            {}", header.root_page)),
        Line::from(format!("keyslot_count:        {}", header.keyslot_count)),
        Line::from(format!(
            "page_list_total:      {}",
            app.observation.pages.total
        )),
        Line::from(format!(
            "page_list_retained:   {}",
            app.observation.pages.entries.len()
        )),
        Line::from(format!(
            "page_list_truncated:  {}",
            app.observation.pages.truncated
        )),
        Line::from(""),
        Line::from("Header facts come from the shared inspection observation."),
    ]
}

fn verify_lines(app: &ViewApp) -> Vec<Line<'static>> {
    let verification = &app.observation.verification;
    let mut lines = vec![
        Line::from(format!("pages_checked: {}", verification.pages_checked)),
        Line::from(format!("pages_ok:      {}", verification.pages_ok)),
        Line::from(format!("issues:        {}", verification.issues.len())),
        Line::from(format!(
            "issues_truncated: {}",
            verification.issues_truncated
        )),
        Line::from(format!("btree_checked: {}", verification.btree_checked)),
        Line::from(format!("btree_ok:      {}", verification.btree_ok)),
        Line::from(if verification.issues.is_empty() {
            "status:        clean".to_string()
        } else {
            format!("status:        {} issue(s)", verification.issues.len())
        }),
    ];

    if verification.issues.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from("No verification anomalies detected."));
    } else {
        lines.push(Line::from(""));
        lines.push(Line::from("Issues:"));
        for issue in verification.issues.iter().take(10) {
            let page = issue
                .page_number
                .map(|page| format!("pg {page:>4}: "))
                .unwrap_or_default();
            lines.push(Line::from(format!(
                "{page}[{}] {}",
                issue.code, issue.message
            )));
        }
        if verification.issues.len() > 10 {
            lines.push(Line::from(format!(
                "... {} more issue(s)",
                verification.issues.len() - 10
            )));
        }
    }

    lines
}

fn detail_lines(app: &ViewApp) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    match &app.selected_detail {
        Some(SelectedPageDetail::Decoded(detail)) => {
            lines.push(Line::from(format!("page:         {}", detail.pgno)));
            lines.push(Line::from(format!(
                "type:         {}",
                page_type_label(detail.page_type)
            )));
            lines.push(Line::from(format!("page_version: {}", detail.page_version)));
            lines.push(Line::from(format!("slot_count:   {}", detail.slot_count)));
            lines.push(Line::from(format!("free_start:   {}", detail.free_start)));
            lines.push(Line::from(format!("free_end:     {}", detail.free_end)));
            lines.push(Line::from(""));

            if detail.records.is_empty() {
                lines.push(Line::from("(no decoded records)"));
            } else {
                for (index, record) in detail.records.iter().enumerate().take(12) {
                    lines.push(Line::from(format!(
                        "slot {index:>2}: {}",
                        record_summary(record)
                    )));
                }
                if detail.records.len() > 12 {
                    lines.push(Line::from(format!(
                        "... {} more record(s)",
                        detail.records.len() - 12
                    )));
                }
            }
        }
        Some(SelectedPageDetail::Unavailable {
            pgno,
            status,
            issue,
        }) => {
            lines.push(Line::from(format!("page:    {pgno}")));
            lines.push(Line::from(format!(
                "status:  {}",
                page_status_label(*status)
            )));
            lines.push(Line::from(""));
            if let Some(issue) = issue {
                lines.push(Line::from(issue.clone()));
            } else {
                lines.push(Line::from("No decoded detail is available for this page."));
            }
        }
        None => lines.push(Line::from(
            "page 0 is the file header; no data pages to inspect yet",
        )),
    }

    lines
}

fn tree_lines(app: &ViewApp) -> Vec<Line<'static>> {
    inspection_tree_lines(&app.observation.tree)
}

fn wal_lines(app: &ViewApp) -> Vec<Line<'static>> {
    inspection_wal_lines(&app.observation.wal)
}

fn protectors_lines(app: &ViewApp) -> Vec<Line<'static>> {
    let verification = &app.observation.verification;
    let mut lines = vec![
        Line::from(format!(
            "header keyslot_count: {}",
            app.observation.header.keyslot_count
        )),
        Line::from(format!(
            "pages_checked:        {}",
            verification.pages_checked
        )),
        Line::from(format!("pages_ok:             {}", verification.pages_ok)),
        Line::from(""),
        Line::from("Configured keyslots:"),
    ];

    push_keyslot_lines(&app.observation.keyslots, &mut lines);

    lines
}

fn inspection_tree_lines(section: &InspectionSection<InspectionTree>) -> Vec<Line<'static>> {
    match section {
        InspectionSection::Available(tree) => {
            let mut lines = vec![
                Line::from(format!("root_pgno: {}", tree.root_page)),
                Line::from(format!("nodes:     {}", tree.nodes.len())),
                Line::from(format!("truncated: {}", tree.truncated)),
                Line::from(""),
            ];
            for node in &tree.nodes {
                let indent = "  ".repeat(node.depth);
                lines.push(Line::from(format!(
                    "{indent}pg={} {} v{} slots={} children={}",
                    node.page_number,
                    page_type_label(node.page_type),
                    node.page_version,
                    node.slot_count,
                    node.child_count,
                )));
            }
            lines
        }
        InspectionSection::Unavailable(unavailable) => vec![Line::from(format!(
            "[{}] {}",
            unavailable.code, unavailable.message
        ))],
    }
}

fn inspection_wal_lines(section: &InspectionSection<InspectionWal>) -> Vec<Line<'static>> {
    match section {
        InspectionSection::Available(wal) => {
            let mut lines = vec![
                Line::from(format!("wal_exists: {}", wal.exists)),
                Line::from(format!("records:    {}", wal.record_count)),
                Line::from(format!("truncated:  {}", wal.truncated)),
                Line::from(""),
            ];
            if wal.records.is_empty() {
                lines.push(Line::from("No WAL records found."));
            } else {
                for record in &wal.records {
                    lines.push(Line::from(format_inspection_wal_record(record)));
                }
            }
            lines
        }
        InspectionSection::Unavailable(unavailable) => vec![Line::from(format!(
            "[{}] {}",
            unavailable.code, unavailable.message
        ))],
    }
}

fn push_keyslot_lines(
    section: &InspectionSection<InspectionKeyslots>,
    lines: &mut Vec<Line<'static>>,
) {
    match section {
        InspectionSection::Available(keyslots) if keyslots.slots.is_empty() => {
            lines.push(Line::from("No active keyslots found."));
        }
        InspectionSection::Available(keyslots) => {
            for keyslot in &keyslots.slots {
                lines.push(Line::from(format!(
                    "slot {:>2}: {}",
                    keyslot.slot,
                    keyslot_kind_label(keyslot.kind)
                )));
            }
            if keyslots.truncated > 0 {
                lines.push(Line::from(format!(
                    "... {} more keyslot(s)",
                    keyslots.truncated
                )));
            }
        }
        InspectionSection::Unavailable(unavailable) => {
            lines.push(Line::from(format!(
                "[{}] {}",
                unavailable.code, unavailable.message
            )));
        }
    }
}

fn format_inspection_wal_record(
    record: &tosumu_core::inspection_session::InspectionWalRecord,
) -> String {
    match &record.kind {
        InspectionWalRecordKind::Begin { transaction_id } => {
            format!("lsn {:>4}: begin txn={transaction_id}", record.lsn)
        }
        InspectionWalRecordKind::PageWrite {
            page_number,
            page_version,
        } => format!(
            "lsn {:>4}: page_write pg={} v{}",
            record.lsn, page_number, page_version
        ),
        InspectionWalRecordKind::Commit { transaction_id } => {
            format!("lsn {:>4}: commit txn={transaction_id}", record.lsn)
        }
        InspectionWalRecordKind::Checkpoint { up_to_lsn } => {
            format!("lsn {:>4}: checkpoint up_to={up_to_lsn}", record.lsn)
        }
    }
}

fn focus_block(title: &str, focused: bool) -> Block<'static> {
    let title = if focused {
        format!("{title} [active]")
    } else {
        title.to_string()
    };
    let mut block = Block::default().title(title).borders(Borders::ALL);
    if focused {
        block = block.border_style(Style::default().fg(Color::Cyan));
    }
    block
}

pub(super) fn page_type_label(page_type: u8) -> &'static str {
    match page_type {
        PAGE_TYPE_LEAF => "Leaf",
        PAGE_TYPE_INTERNAL => "Internal",
        PAGE_TYPE_OVERFLOW => "Overflow",
        PAGE_TYPE_FREE => "Free",
        _ => "Unknown",
    }
}

pub(super) fn keyslot_kind_label(kind: u8) -> &'static str {
    match kind {
        0 => "Empty",
        1 => "Sentinel",
        2 => "Passphrase",
        3 => "RecoveryKey",
        4 => "Keyfile",
        _ => "Unknown",
    }
}

#[cfg(test)]
pub(super) fn selected_page_auth_summary(report: &VerifyReport, pgno: u64) -> String {
    report
        .page_results
        .iter()
        .find(|result| result.pgno == pgno)
        .map(|result| {
            if result.auth_ok {
                match result.page_version {
                    Some(version) => format!("pg {pgno} ok (v{version})"),
                    None => format!("pg {pgno} ok"),
                }
            } else {
                format!("pg {pgno} {}", page_verify_state_label(result))
            }
        })
        .unwrap_or_else(|| format!("pg {pgno} not in verify report"))
}

#[cfg(test)]
fn page_verify_state_label(result: &PageVerifyResult) -> &'static str {
    if result.auth_ok {
        "ok"
    } else {
        match result.issue_kind {
            Some(VerifyIssueKind::AuthFailed) => "auth_failed",
            Some(VerifyIssueKind::Corrupt) => "corrupt",
            Some(VerifyIssueKind::Io) => "io",
            None => "unknown",
        }
    }
}

#[cfg(test)]
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct PageAuthSummary {
    pub(super) ok: u64,
    pub(super) auth_failed: usize,
    pub(super) corrupt: usize,
    pub(super) io: usize,
}

#[cfg(test)]
pub(super) fn summarize_page_auth(report: &VerifyReport) -> PageAuthSummary {
    let mut summary = PageAuthSummary::default();
    for result in &report.page_results {
        if result.auth_ok {
            summary.ok += 1;
            continue;
        }

        match result.issue_kind {
            Some(VerifyIssueKind::AuthFailed) => summary.auth_failed += 1,
            Some(VerifyIssueKind::Corrupt) => summary.corrupt += 1,
            Some(VerifyIssueKind::Io) => summary.io += 1,
            None => summary.io += 1,
        }
    }
    summary
}

fn page_status_label(status: PageStatus) -> &'static str {
    match status {
        PageStatus::Ok => "ok",
        PageStatus::AuthFailed => "auth",
        PageStatus::Corrupt => "corrupt",
        PageStatus::Io => "io",
    }
}

fn page_status_style(status: PageStatus) -> Style {
    match status {
        PageStatus::Ok => Style::default().fg(Color::Green),
        PageStatus::AuthFailed => Style::default().fg(Color::Red),
        PageStatus::Corrupt => Style::default().fg(Color::Yellow),
        PageStatus::Io => Style::default().fg(Color::Magenta),
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

pub(super) fn preview_bytes(bytes: &[u8]) -> String {
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
            let hex = bytes
                .iter()
                .take(16)
                .map(|b| format!("{b:02x}"))
                .collect::<String>();
            if bytes.len() > 16 {
                format!("0x{hex}...")
            } else {
                format!("0x{hex}")
            }
        }
    }
}

#[cfg(test)]
pub(super) fn format_wal_record(record: &WalRecordSummary) -> String {
    match &record.kind {
        WalRecordSummaryKind::Begin { txn_id } => {
            format!("lsn {:>4}: begin txn={txn_id}", record.lsn)
        }
        WalRecordSummaryKind::PageWrite { pgno, page_version } => {
            format!(
                "lsn {:>4}: page_write pg={} v{}",
                record.lsn, pgno, page_version
            )
        }
        WalRecordSummaryKind::Commit { txn_id } => {
            format!("lsn {:>4}: commit txn={txn_id}", record.lsn)
        }
        WalRecordSummaryKind::Checkpoint { up_to_lsn } => {
            format!("lsn {:>4}: checkpoint up_to={up_to_lsn}", record.lsn)
        }
    }
}

fn page_list_item(page: &super::state::PageRow) -> ListItem<'static> {
    ListItem::new(Line::from(vec![
        Span::styled("■", page_status_style(page.status)),
        Span::raw(page_list_summary(page)),
    ]))
}

pub(super) fn page_list_summary(page: &super::state::PageRow) -> String {
    let page_version = page
        .page_version
        .map(|value| value.to_string())
        .unwrap_or_else(|| "--".to_string());
    let slot_count = page
        .slot_count
        .map(|value| value.to_string())
        .unwrap_or_else(|| "--".to_string());
    format!(
        " {:>4}  {:<8}  {:<7}  v{:>3}  slots {:>3}",
        page.pgno,
        page.page_type.map(page_type_label).unwrap_or("?"),
        page_status_label(page.status),
        page_version,
        slot_count,
    )
}
