use std::path::Path;
use std::time::{Duration, Instant};

use ratatui::widgets::ListState;
use tosumu_core::inspect::{
    inspect_page_from_pager,
    inspect_pages_from_pager,
    HeaderInfo,
    PageInspectState,
    PageSummary,
    TreeSummary,
    VerifyReport,
    WalSummary,
};
use tosumu_core::pager::Pager;

use crate::error_boundary::CliError;

use super::watch::{capture_watch_fingerprint, watch_refresh_needed, WatchFingerprint};

pub(super) const PANEL_SCROLL_PAGE: u16 = 8;
pub(super) const PAGE_LIST_JUMP: usize = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FocusPane {
    Pages,
    Panel,
}

impl FocusPane {
    pub(super) fn toggle(self) -> Self {
        match self {
            FocusPane::Pages => FocusPane::Panel,
            FocusPane::Panel => FocusPane::Pages,
        }
    }

    pub(super) fn label(self) -> &'static str {
        match self {
            FocusPane::Pages => "pages",
            FocusPane::Panel => "panel",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ViewMode {
    Header,
    Detail,
    Verify,
    Tree,
    Wal,
    Protectors,
}

impl ViewMode {
    pub(super) const ALL: [ViewMode; 6] = [
        ViewMode::Header,
        ViewMode::Detail,
        ViewMode::Verify,
        ViewMode::Tree,
        ViewMode::Wal,
        ViewMode::Protectors,
    ];

    pub(super) fn from_key(code: crossterm::event::KeyCode) -> Option<Self> {
        match code {
            crossterm::event::KeyCode::Char('1') | crossterm::event::KeyCode::Char('h') => Some(ViewMode::Header),
            crossterm::event::KeyCode::Char('2') | crossterm::event::KeyCode::Char('d') => Some(ViewMode::Detail),
            crossterm::event::KeyCode::Char('3') | crossterm::event::KeyCode::Char('v') => Some(ViewMode::Verify),
            crossterm::event::KeyCode::Char('4') | crossterm::event::KeyCode::Char('t') => Some(ViewMode::Tree),
            crossterm::event::KeyCode::Char('5') | crossterm::event::KeyCode::Char('l') => Some(ViewMode::Wal),
            crossterm::event::KeyCode::Char('6') | crossterm::event::KeyCode::Char('p') => Some(ViewMode::Protectors),
            _ => None,
        }
    }

    pub(super) fn label(self) -> &'static str {
        match self {
            ViewMode::Header => "Header",
            ViewMode::Detail => "Detail",
            ViewMode::Verify => "Verify",
            ViewMode::Tree => "Tree",
            ViewMode::Wal => "WAL",
            ViewMode::Protectors => "Protectors",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PageStatus {
    Ok,
    AuthFailed,
    Corrupt,
    Io,
}

#[derive(Clone)]
pub(super) struct PageRow {
    pub(super) pgno: u64,
    pub(super) page_type: Option<u8>,
    pub(super) page_version: Option<u64>,
    pub(super) slot_count: Option<u16>,
    pub(super) status: PageStatus,
    pub(super) issue: Option<String>,
}

pub(super) enum SelectedPageDetail {
    Decoded(PageSummary),
    Unavailable {
        pgno: u64,
        status: PageStatus,
        issue: Option<String>,
    },
}

pub(super) struct ViewApp<'a> {
    pub(super) path: &'a Path,
    pub(super) header: HeaderInfo,
    pub(super) verify: VerifyReport,
    pub(super) pages: Vec<PageRow>,
    pub(super) mode: ViewMode,
    pub(super) focus: FocusPane,
    pub(super) watch_enabled: bool,
    pub(super) panel_scroll: u16,
    pub(super) last_refresh: Instant,
    pub(super) last_watch_fingerprint: Option<WatchFingerprint>,
    pub(super) status_message: Option<String>,
    pub(super) tree: Result<TreeSummary, String>,
    pub(super) wal: Result<WalSummary, String>,
    pub(super) keyslots: Result<Vec<(u16, u8)>, String>,
    pub(super) selected: Option<usize>,
    pub(super) selected_detail: Option<SelectedPageDetail>,
}

impl<'a> ViewApp<'a> {
    pub(super) fn new(
        path: &'a Path,
        header: HeaderInfo,
        verify: VerifyReport,
        pages: Vec<PageRow>,
        tree: Result<TreeSummary, String>,
        wal: Result<WalSummary, String>,
        keyslots: Result<Vec<(u16, u8)>, String>,
        watch_enabled: bool,
    ) -> Self {
        Self {
            path,
            header,
            verify,
            pages,
            mode: ViewMode::Detail,
            focus: FocusPane::Pages,
            watch_enabled,
            panel_scroll: 0,
            last_refresh: Instant::now(),
            last_watch_fingerprint: capture_watch_fingerprint(path).ok(),
            status_message: None,
            tree,
            wal,
            keyslots,
            selected: None,
            selected_detail: None,
        }
    }

    pub(super) fn list_state(&self) -> ListState {
        let mut state = ListState::default();
        state.select(self.selected);
        state
    }

    pub(super) fn select_first(&mut self, pager: &Pager) -> Result<(), CliError> {
        if self.pages.is_empty() {
            return Ok(());
        }
        self.selected = Some(0);
        self.panel_scroll = 0;
        self.refresh_selected_detail(pager)
    }

    pub(super) fn select_last(&mut self, pager: &Pager) -> Result<(), CliError> {
        if self.pages.is_empty() {
            return Ok(());
        }
        self.selected = Some(self.pages.len() - 1);
        self.panel_scroll = 0;
        self.refresh_selected_detail(pager)
    }

    pub(super) fn select_next(&mut self, pager: &Pager) -> Result<(), CliError> {
        if self.pages.is_empty() {
            return Ok(());
        }
        self.selected = Some(match self.selected {
            Some(index) if index + 1 < self.pages.len() => index + 1,
            _ => self.pages.len() - 1,
        });
        self.panel_scroll = 0;
        self.refresh_selected_detail(pager)
    }

    pub(super) fn select_previous(&mut self, pager: &Pager) -> Result<(), CliError> {
        if self.pages.is_empty() {
            return Ok(());
        }
        self.selected = Some(match self.selected {
            Some(index) if index > 0 => index - 1,
            _ => 0,
        });
        self.panel_scroll = 0;
        self.refresh_selected_detail(pager)
    }

    pub(super) fn select_index(&mut self, pager: &Pager, index: usize) -> Result<(), CliError> {
        if self.pages.is_empty() {
            return Ok(());
        }

        self.selected = Some(index.min(self.pages.len() - 1));
        self.panel_scroll = 0;
        self.refresh_selected_detail(pager)
    }

    pub(super) fn move_down(&mut self, pager: &Pager) -> Result<(), CliError> {
        match self.focus {
            FocusPane::Pages => self.select_next(pager),
            FocusPane::Panel => {
                self.scroll_panel_down(1);
                Ok(())
            }
        }
    }

    pub(super) fn move_up(&mut self, pager: &Pager) -> Result<(), CliError> {
        match self.focus {
            FocusPane::Pages => self.select_previous(pager),
            FocusPane::Panel => {
                self.scroll_panel_up(1);
                Ok(())
            }
        }
    }

    pub(super) fn move_home(&mut self, pager: &Pager) -> Result<(), CliError> {
        match self.focus {
            FocusPane::Pages => self.select_first(pager),
            FocusPane::Panel => {
                self.panel_scroll = 0;
                Ok(())
            }
        }
    }

    pub(super) fn move_end(&mut self, pager: &Pager) -> Result<(), CliError> {
        match self.focus {
            FocusPane::Pages => self.select_last(pager),
            FocusPane::Panel => {
                self.panel_scroll = u16::MAX;
                Ok(())
            }
        }
    }

    pub(super) fn move_page_down(&mut self, pager: &Pager) -> Result<(), CliError> {
        match self.focus {
            FocusPane::Pages => {
                let next_index = self.selected.unwrap_or(0).saturating_add(PAGE_LIST_JUMP);
                self.select_index(pager, next_index)
            }
            FocusPane::Panel => {
                self.scroll_panel_down(PANEL_SCROLL_PAGE);
                Ok(())
            }
        }
    }

    pub(super) fn move_page_up(&mut self, pager: &Pager) -> Result<(), CliError> {
        match self.focus {
            FocusPane::Pages => {
                let next_index = self.selected.unwrap_or(0).saturating_sub(PAGE_LIST_JUMP);
                self.select_index(pager, next_index)
            }
            FocusPane::Panel => {
                self.scroll_panel_up(PANEL_SCROLL_PAGE);
                Ok(())
            }
        }
    }

    pub(super) fn toggle_focus(&mut self) {
        self.focus = self.focus.toggle();
    }

    pub(super) fn set_mode(&mut self, mode: ViewMode) {
        if self.mode != mode {
            self.mode = mode;
            self.panel_scroll = 0;
        }
    }

    pub(super) fn scroll_panel_down(&mut self, amount: u16) {
        self.panel_scroll = self.panel_scroll.saturating_add(amount);
    }

    pub(super) fn scroll_panel_up(&mut self, amount: u16) {
        self.panel_scroll = self.panel_scroll.saturating_sub(amount);
    }

    pub(super) fn refresh_selected_detail(&mut self, pager: &Pager) -> Result<(), CliError> {
        self.selected_detail = match self.selected.and_then(|index| self.pages.get(index).cloned()) {
            Some(page) if matches!(page.status, PageStatus::Ok) => {
                Some(SelectedPageDetail::Decoded(inspect_page_from_pager(pager, page.pgno)?))
            }
            Some(page) => Some(SelectedPageDetail::Unavailable {
                pgno: page.pgno,
                status: page.status,
                issue: page.issue,
            }),
            None => None,
        };
        Ok(())
    }

    pub(super) fn restore_selection(&mut self, selected_pgno: Option<u64>, pager: &Pager) -> Result<(), CliError> {
        if self.pages.is_empty() {
            self.selected = None;
            self.selected_detail = None;
            return Ok(());
        }

        self.selected = selected_pgno
            .and_then(|pgno| self.pages.iter().position(|page| page.pgno == pgno))
            .or(Some(0));
        self.panel_scroll = 0;
        self.refresh_selected_detail(pager)
    }

    pub(super) fn should_refresh(&self) -> bool {
        self.watch_enabled && self.last_refresh.elapsed() >= Duration::from_secs(1)
    }

    pub(super) fn watch_refresh_needed(&self, path: &Path) -> std::io::Result<bool> {
        watch_refresh_needed(path, self.last_watch_fingerprint.as_ref())
    }

    pub(super) fn note_watch_check(&mut self) {
        self.last_refresh = Instant::now();
    }

    pub(super) fn toggle_watch(&mut self) {
        self.watch_enabled = !self.watch_enabled;
        self.last_refresh = Instant::now();
        self.status_message = Some(if self.watch_enabled {
            "watch enabled".to_string()
        } else {
            "watch paused".to_string()
        });
    }

    pub(super) fn status_suffix(&self) -> String {
        self.status_message
            .as_ref()
            .map(|message| format!(" • {message}"))
            .unwrap_or_default()
    }
}

pub(super) fn load_page_rows(pager: &Pager) -> Result<Vec<PageRow>, CliError> {
    let pages = inspect_pages_from_pager(pager)?;
    Ok(pages
        .pages
        .into_iter()
        .map(|page| PageRow {
            pgno: page.pgno,
            page_type: page.page_type,
            page_version: page.page_version,
            slot_count: page.slot_count,
            status: match page.state {
                PageInspectState::Ok => PageStatus::Ok,
                PageInspectState::AuthFailed => PageStatus::AuthFailed,
                PageInspectState::Corrupt => PageStatus::Corrupt,
                PageInspectState::Io => PageStatus::Io,
            },
            issue: page.issue,
        })
        .collect())
}