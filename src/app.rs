use crate::clear::Clear;
use crate::git_status::StatusLists;
use crate::git_utils;
use crate::git_utils::Diff;
use crate::git_utils::DiffLine;
use crossterm::event::{Event, KeyCode, MouseEvent};
use git_utils::DiffLineType;
use std::cmp;
use std::path::Path;
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, SelectableList, Text, Widget},
    Frame,
};

#[derive(Default)]
pub struct App {
    status: StatusLists,
    status_select: Option<usize>,
    diff: Diff,
    offset: u16,
    do_quit: bool,
    show_popup: bool,
}

impl App {
    ///
    pub fn is_quit(&self) -> bool {
        self.do_quit
    }
}

impl App {
    ///
    fn fetch_status(&mut self) {
        let new_status = StatusLists::new();

        if self.status != new_status {
            self.status = new_status;

            self.status_select = if self.status.wt_items.len() > 0 {
                Some(0)
            } else {
                None
            };
        }

        self.update_diff();
    }

    ///
    fn update_diff(&mut self) {
        let new_diff = match self.status_select {
            Some(i) => git_utils::get_diff(Path::new(self.status.wt_items[i].path.as_str())),
            None => Diff::default(),
        };

        if new_diff != self.diff {
            self.diff = new_diff;
            self.offset = 0;
        }
    }

    ///
    pub fn draw<B: Backend>(&self, f: &mut Frame<B>) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(f.size());

        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(chunks[0]);

        draw_list(
            f,
            left_chunks[0],
            "Status [S]".to_string(),
            self.status.wt_items_pathlist().as_slice(),
            self.status_select,
            true,
        );

        draw_list(
            f,
            left_chunks[1],
            "Index [I]".to_string(),
            self.status.index_items_pathlist().as_slice(),
            None,
            false,
        );

        let txt = self
            .diff
            .0
            .iter()
            .map(|e: &DiffLine| {
                let content = e.content.clone();
                match e.line_type {
                    DiffLineType::Delete => Text::Styled(
                        content.into(),
                        Style::default().fg(Color::White).bg(Color::Red),
                    ),
                    DiffLineType::Add => Text::Styled(
                        content.into(),
                        Style::default().fg(Color::White).bg(Color::Green),
                    ),
                    DiffLineType::Header => Text::Styled(
                        content.into(),
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Gray)
                            .modifier(Modifier::BOLD),
                    ),
                    _ => Text::Raw(content.into()),
                }
            })
            .collect::<Vec<_>>();

        Paragraph::new(txt.iter())
            .block(Block::default().title("Diff [D]").borders(Borders::ALL))
            .alignment(Alignment::Left)
            .scroll(self.offset)
            .render(f, chunks[1]);

        if self.show_popup {
            let txt = [Text::Raw(
                "test lorem ipsum dolorem test lorem ipsum dolorem"
                    .to_string()
                    .into(),
            )];

            Clear::new(
                Paragraph::new(txt.iter())
                    .block(Block::default().title("Popup").borders(Borders::ALL))
                    .alignment(Alignment::Center),
            )
            .render(f, Rect::new(20, 0, 100, 10));
        }
    }

    ///
    pub fn event(&mut self, ev: Event) {
        if ev == Event::Key(KeyCode::Esc.into()) || ev == Event::Key(KeyCode::Char('q').into()) {
            self.do_quit = true;
        }

        if ev == Event::Key(KeyCode::Char('d').into()) {
            self.show_popup = !self.show_popup;
        }

        if ev == Event::Key(KeyCode::Up.into()) {
            self.input(-1);
        }
        if ev == Event::Key(KeyCode::Down.into()) {
            self.input(1);
        }

        if ev == Event::Key(KeyCode::PageDown.into()) {
            self.scroll(true);
        }
        if ev == Event::Key(KeyCode::PageUp.into()) {
            self.scroll(false);
        }
        if let Event::Mouse(MouseEvent::ScrollDown(_, _, _)) = ev {
            self.scroll(true);
        }
        if let Event::Mouse(MouseEvent::ScrollUp(_, _, _)) = ev {
            self.scroll(false);
        }

        if ev == Event::Key(KeyCode::Enter.into()) {
            self.index_add();
        }
    }

    pub fn update(&mut self) {
        self.fetch_status();
    }

    fn index_add(&mut self) {
        if let Some(i) = self.status_select {
            let repo = git_utils::repo();

            let mut index = repo.index().unwrap();

            let path = Path::new(self.status.wt_items[i].path.as_str());
            index.update_all(path, None).unwrap();
            index.write().unwrap();

            self.update();
        }
    }

    fn scroll(&mut self, inc: bool) {
        if inc {
            self.offset = self.offset.checked_add(1).unwrap_or(self.offset);
        } else {
            self.offset = self.offset.checked_sub(1).unwrap_or(0);
        }
    }

    fn input(&mut self, delta: i32) {
        let items_len = self.status.wt_items.len();
        if items_len > 0 {
            if let Some(i) = self.status_select {
                let mut i = i as i32;

                i = cmp::min(i + delta, (items_len - 1) as i32);
                i = cmp::max(i, 0);

                self.status_select = Some(i as usize);
            }
        }

        self.update_diff();
    }
}

fn draw_list<B: Backend, T: AsRef<str>>(
    f: &mut Frame<B>,
    r: Rect,
    title: String,
    items: &[T],
    select: Option<usize>,
    selected: bool,
) {
    let mut style_border = Style::default();
    let mut style_title = Style::default();
    if selected {
        style_border = style_border.fg(Color::Red);
        style_title = style_title.modifier(Modifier::BOLD);
    }
    SelectableList::default()
        .block(
            Block::default()
                .title(title.as_str())
                .borders(Borders::ALL)
                .title_style(style_title)
                .border_style(style_border),
        )
        .items(items)
        .select(select)
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().modifier(Modifier::BOLD))
        .highlight_symbol(">")
        .render(f, r);
}
