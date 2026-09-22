use ratatui::{
    buffer::Buffer,
    layout::Rect,
    symbols::{self, scrollbar},
    text::Line,
    widgets::{
        Clear, Padding, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget,
        Widget, Wrap,
    },
};

use crate::{
    canvas::{components::scroll_bar::dialog_scroll_bar_area, drawing_utils::dialog_block},
    collection::processes::Pid,
    options::config::style::Styles,
};

pub struct CommandDialog<'a> {
    styles: &'a Styles,
}

impl<'a> CommandDialog<'a> {
    pub fn new(styles: &'a Styles) -> Self {
        Self { styles }
    }
}

impl StatefulWidget for CommandDialog<'_> {
    type State = CommandDialogState;

    fn render(self, area: Rect, buffer: &mut Buffer, state: &mut Self::State) {
        if !state.is_open() {
            return;
        }

        Clear.render(area, buffer);
        buffer.set_style(area, self.styles.general_widget_style);

        let block = dialog_block(self.styles.border_type, self.styles.border_style)
            .title_top(Line::styled(state.title(), self.styles.widget_title_style))
            .title_top(
                Line::styled(" Esc to close ", self.styles.widget_title_style).right_aligned(),
            )
            .padding(Padding::right(1));
        let inner = block.inner(area);

        let line_count = Paragraph::new(state.content())
            .wrap(Wrap { trim: false })
            .line_count(inner.width);
        state.max_scroll = line_count
            .saturating_sub(inner.height.into())
            .min(u16::MAX.into()) as u16;
        state.viewport_height = inner.height;
        state.scroll = state.scroll.min(state.max_scroll);

        Paragraph::new(state.content())
            .block(block)
            .style(self.styles.text_style)
            .wrap(Wrap { trim: false })
            .scroll((state.scroll, 0))
            .render(area, buffer);

        if state.max_scroll > 0 {
            const SYMBOLS: scrollbar::Set<'_> = scrollbar::Set {
                track: "",
                thumb: symbols::block::FULL,
                begin: "▲",
                end: "▼",
            };
            let scrollbar_area = dialog_scroll_bar_area(area);
            let scrollbar = {
                let base = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .style(self.styles.text_style);

                if scrollbar_area.height > 2 {
                    base.symbols(SYMBOLS)
                } else {
                    base.track_symbol(Some(SYMBOLS.track))
                        .thumb_symbol(SYMBOLS.thumb)
                        .begin_symbol(None)
                        .end_symbol(None)
                }
            };
            let mut scrollbar_state = ScrollbarState::new(line_count)
                .position(state.scroll.into())
                .viewport_content_length(inner.height.into());

            scrollbar.render(scrollbar_area, buffer, &mut scrollbar_state);
        }
    }
}

const GROUPED_MESSAGE: &str = "This row represents multiple processes. Press Esc, then Tab to ungroup them before viewing a command.";

#[derive(Default)]
pub struct CommandDialogState {
    content: Option<CommandDialogContent>,
    scroll: u16,
    max_scroll: u16,
    viewport_height: u16,
}

enum CommandDialogContent {
    Command { pid: Pid, command: String },
    Grouped,
}

impl CommandDialogState {
    pub fn command(pid: Pid, command: String) -> Self {
        Self {
            content: Some(CommandDialogContent::Command { pid, command }),
            scroll: 0,
            max_scroll: 0,
            viewport_height: 0,
        }
    }

    pub fn grouped() -> Self {
        Self {
            content: Some(CommandDialogContent::Grouped),
            ..Self::default()
        }
    }

    pub fn is_open(&self) -> bool {
        self.content.is_some()
    }

    pub fn title(&self) -> String {
        match &self.content {
            Some(CommandDialogContent::Command { pid, .. }) => {
                format!(" Command for PID {pid} ")
            }
            Some(CommandDialogContent::Grouped) => " Command unavailable ".into(),
            None => String::new(),
        }
    }

    pub fn content(&self) -> &str {
        match &self.content {
            Some(CommandDialogContent::Command { command, .. }) => command,
            Some(CommandDialogContent::Grouped) => GROUPED_MESSAGE,
            None => "",
        }
    }

    pub fn close(&mut self) {
        self.content = None;
        self.scroll = 0;
        self.max_scroll = 0;
        self.viewport_height = 0;
    }

    pub fn scroll_down(&mut self) {
        self.scroll_by(self.scroll.saturating_add(1));
    }

    pub fn scroll_up(&mut self) {
        self.scroll_by(self.scroll.saturating_sub(1));
    }

    pub fn page_down(&mut self) {
        self.scroll_by(self.scroll.saturating_add(self.viewport_height));
    }

    pub fn page_up(&mut self) {
        self.scroll_by(self.scroll.saturating_sub(self.viewport_height));
    }

    pub fn half_page_down(&mut self) {
        self.scroll_by(self.scroll.saturating_add(self.viewport_height / 2));
    }

    pub fn half_page_up(&mut self) {
        self.scroll_by(self.scroll.saturating_sub(self.viewport_height / 2));
    }

    pub fn scroll_to_start(&mut self) {
        self.scroll = 0;
    }

    pub fn scroll_to_end(&mut self) {
        self.scroll = self.max_scroll;
    }

    fn scroll_by(&mut self, position: u16) {
        self.scroll = position.min(self.max_scroll);
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{buffer::Buffer, layout::Rect, widgets::StatefulWidget};

    use crate::options::config::style::Styles;

    use super::*;

    fn line(buffer: &Buffer, y: u16) -> String {
        (0..buffer.area.width)
            .map(|x| buffer[(x, y)].symbol())
            .collect()
    }

    fn line_without_scrollbar(buffer: &Buffer, y: u16) -> String {
        let mut chars: Vec<_> = line(buffer, y).chars().collect();
        chars[12] = ' ';
        chars.into_iter().collect()
    }

    #[test]
    fn command_view_snapshots_and_closes() {
        let original = String::from("python worker.py --queue critical");
        let mut view = CommandDialogState::command(42, original.clone());

        assert!(view.is_open());
        assert_eq!(view.title(), " Command for PID 42 ");
        assert_eq!(view.content(), original);

        view.close();
        assert!(!view.is_open());
    }

    #[test]
    fn command_view_preserves_configured_background() {
        let area = Rect::new(0, 0, 40, 5);
        let mut buffer = Buffer::empty(area);
        let mut view = CommandDialogState::command(42, "python worker.py".into());
        let background = ratatui::style::Color::Blue;
        let styles = Styles {
            general_widget_style: ratatui::style::Style::default().bg(background),
            ..Styles::default()
        };

        CommandDialog::new(&styles).render(area, &mut buffer, &mut view);

        for y in 0..area.height {
            for x in 0..area.width {
                assert_eq!(buffer[(x, y)].bg, background, "cell ({x}, {y})");
            }
        }
    }

    #[test]
    fn command_view_wraps_without_trimming_whitespace() {
        let area = Rect::new(0, 0, 14, 5);
        let mut buffer = Buffer::empty(area);
        let mut view = CommandDialogState::command(42, "  alpha beta gamma".into());

        CommandDialog::new(&Styles::default()).render(area, &mut buffer, &mut view);

        assert_eq!(line(&buffer, 1), "│  alpha     │");
        assert_eq!(line(&buffer, 2), "│beta gamma  │");
    }

    #[test]
    fn command_view_scrolls_through_every_wrapped_line() {
        let area = Rect::new(0, 0, 14, 4);
        let mut buffer = Buffer::empty(area);
        let mut view =
            CommandDialogState::command(42, "one two three four five six seven eight".into());
        let styles = Styles::default();
        let dialog = CommandDialog::new(&styles);

        dialog.render(area, &mut buffer, &mut view);
        assert_eq!(line_without_scrollbar(&buffer, 1), "│one two     │");
        assert_eq!(line_without_scrollbar(&buffer, 2), "│three four  │");

        view.scroll_down();
        CommandDialog::new(&Styles::default()).render(area, &mut buffer, &mut view);
        assert_eq!(line_without_scrollbar(&buffer, 1), "│three four  │");
        assert_eq!(line_without_scrollbar(&buffer, 2), "│five six    │");

        view.scroll_down();
        view.scroll_down();
        CommandDialog::new(&Styles::default()).render(area, &mut buffer, &mut view);
        assert_eq!(line_without_scrollbar(&buffer, 1), "│five six    │");
        assert_eq!(line_without_scrollbar(&buffer, 2), "│seven eight │");
    }

    #[test]
    fn grouped_processes_explain_how_to_view_a_command() {
        let view = CommandDialogState::grouped();

        assert!(view.is_open());
        assert_eq!(view.title(), " Command unavailable ");
        assert_eq!(
            view.content(),
            "This row represents multiple processes. Press Esc, then Tab to ungroup them before viewing a command."
        );
    }

    #[test]
    fn overflowing_command_shows_a_scrollbar() {
        let area = Rect::new(0, 0, 14, 6);
        let mut buffer = Buffer::empty(area);
        let mut view = CommandDialogState::command(
            42,
            "one two three four five six seven eight nine ten eleven twelve".into(),
        );

        CommandDialog::new(&Styles::default()).render(area, &mut buffer, &mut view);

        assert_eq!(buffer[(12, 1)].symbol(), "▲");
        assert_eq!(buffer[(12, 4)].symbol(), "▼");
    }

    #[test]
    fn overflowing_command_keeps_scrollbar_in_short_view() {
        let area = Rect::new(0, 0, 14, 4);
        let mut buffer = Buffer::empty(area);
        let mut view = CommandDialogState::command(
            42,
            "one two three four five six seven eight nine ten".into(),
        );

        CommandDialog::new(&Styles::default()).render(area, &mut buffer, &mut view);

        assert_eq!(buffer[(12, 1)].symbol(), "█");
        assert_eq!(buffer[(12, 2)].symbol(), " ");
    }

    #[test]
    fn overflowing_command_keeps_scrollbar_in_one_row_view() {
        let area = Rect::new(0, 0, 14, 3);
        let mut buffer = Buffer::empty(area);
        let mut view = CommandDialogState::command(42, "one two three four five".into());

        CommandDialog::new(&Styles::default()).render(area, &mut buffer, &mut view);

        assert_eq!(buffer[(12, 1)].symbol(), "█");
    }

    #[test]
    fn command_view_supports_page_and_endpoint_navigation() {
        let area = Rect::new(0, 0, 14, 4);
        let mut buffer = Buffer::empty(area);
        let mut view = CommandDialogState::command(
            42,
            "one two three four five six seven eight nine ten".into(),
        );
        let styles = Styles::default();

        CommandDialog::new(&styles).render(area, &mut buffer, &mut view);
        view.page_down();
        CommandDialog::new(&styles).render(area, &mut buffer, &mut view);
        assert_eq!(line_without_scrollbar(&buffer, 1), "│five six    │");

        view.scroll_to_end();
        CommandDialog::new(&styles).render(area, &mut buffer, &mut view);
        assert_eq!(line_without_scrollbar(&buffer, 2), "│nine ten    │");

        view.scroll_to_start();
        CommandDialog::new(&styles).render(area, &mut buffer, &mut view);
        assert_eq!(line_without_scrollbar(&buffer, 1), "│one two     │");
    }

    #[test]
    fn command_view_rewraps_and_clamps_scroll_after_resize() {
        let small = Rect::new(0, 0, 14, 4);
        let large = Rect::new(0, 0, 28, 6);
        let mut buffer = Buffer::empty(small);
        let mut view = CommandDialogState::command(42, "one two three four five six".into());
        let styles = Styles::default();

        CommandDialog::new(&styles).render(small, &mut buffer, &mut view);
        view.scroll_to_end();

        let mut buffer = Buffer::empty(large);
        CommandDialog::new(&styles).render(large, &mut buffer, &mut view);
        assert_eq!(line(&buffer, 1), "│one two three four five   │");
        assert_eq!(line(&buffer, 2), "│six                       │");
    }

    #[test]
    fn command_view_hard_wraps_long_tokens() {
        let area = Rect::new(0, 0, 14, 5);
        let mut buffer = Buffer::empty(area);
        let mut view = CommandDialogState::command(42, "abcdefghijklmnop".into());

        CommandDialog::new(&Styles::default()).render(area, &mut buffer, &mut view);

        assert_eq!(line(&buffer, 1), "│abcdefghijk │");
        assert_eq!(line(&buffer, 2), "│lmnop       │");
    }

    #[test]
    fn command_view_wraps_using_unicode_display_width() {
        let area = Rect::new(0, 0, 14, 5);
        let mut buffer = Buffer::empty(area);
        let mut view = CommandDialogState::command(42, "界界界界界界界".into());

        CommandDialog::new(&Styles::default()).render(area, &mut buffer, &mut view);

        assert_eq!(line(&buffer, 1), "│界 界 界 界 界   │");
        assert_eq!(line(&buffer, 2), "│界 界         │");
    }
}
