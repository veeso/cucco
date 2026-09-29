use std::io::{self, Write};

use crossterm::cursor::{Hide, MoveToColumn, MoveUp, RestorePosition, SavePosition, Show};
use crossterm::event::{
    self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyEvent, KeyEventKind,
    KeyModifiers,
};
use crossterm::style::{Attribute, Print, SetAttribute};
use crossterm::terminal::{self, Clear, ClearType};
use crossterm::{ExecutableCommand, queue};
use inquire::error::{InquireError, InquireResult};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[derive(Debug)]
pub(crate) struct MultilineText<'a> {
    message: &'a str,
    help_message: &'a str,
}

impl<'a> MultilineText<'a> {
    pub(crate) fn new(message: &'a str, help_message: &'a str) -> Self {
        Self {
            message,
            help_message,
        }
    }

    pub(crate) fn prompt_skippable(self) -> InquireResult<Option<String>> {
        let _terminal_session = TerminalSession::enable()?;
        let mut prompt = PromptState::new(self.message, self.help_message);
        prompt.render()?;

        loop {
            match event::read()? {
                Event::Key(key) if key.kind != KeyEventKind::Release => {
                    if let Some(result) = prompt.handle_key(key)? {
                        return Ok(result);
                    }
                }
                Event::Resize(_, _) => prompt.render_after_resize()?,
                Event::Paste(value) => {
                    prompt.insert_pasted_text(&value);
                    prompt.render()?;
                }
                _ => {}
            }
        }
    }
}

#[derive(Debug)]
struct TerminalSession {
    bracketed_paste: bool,
}

impl TerminalSession {
    fn enable() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        let mut session = Self {
            bracketed_paste: false,
        };
        session.bracketed_paste =
            optional_capability(io::stderr().execute(EnableBracketedPaste).map(|_| ()))?;
        Ok(session)
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let mut stderr = io::stderr();
        if self.bracketed_paste {
            _ = queue!(stderr, DisableBracketedPaste);
        }
        _ = queue!(stderr, SetAttribute(Attribute::Reset), Show);
        _ = stderr.flush();
        _ = terminal::disable_raw_mode();
    }
}

fn optional_capability(result: io::Result<()>) -> io::Result<bool> {
    match result {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::Unsupported => Ok(false),
        Err(error) => Err(error),
    }
}

#[derive(Debug)]
struct PromptState<'a> {
    content: String,
    cursor: usize,
    cursor_row: u16,
    help_message: &'a str,
    message: &'a str,
}

impl<'a> PromptState<'a> {
    fn new(message: &'a str, help_message: &'a str) -> Self {
        Self {
            content: String::new(),
            cursor: 0,
            cursor_row: 0,
            help_message,
            message,
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> InquireResult<Option<Option<String>>> {
        match (key.code, key.modifiers) {
            (KeyCode::Char('c'), modifiers) if modifiers.contains(KeyModifiers::CONTROL) => {
                self.finish(None)?;
                return Err(InquireError::OperationInterrupted);
            }
            (KeyCode::Esc, _) => {
                self.finish(None)?;
                return Ok(Some(None));
            }
            (KeyCode::Char('g' | 'd'), modifiers) if modifiers.contains(KeyModifiers::CONTROL) => {
                self.finish(None)?;
                return Ok(Some(None));
            }
            (KeyCode::Enter | KeyCode::Char('\n' | '\r'), modifiers)
                if modifiers.contains(KeyModifiers::ALT) =>
            {
                self.insert_char('\n');
            }
            (KeyCode::Enter | KeyCode::Char('\n' | '\r'), _) => {
                let answer = self.content.clone();
                self.finish(Some(&answer))?;
                return Ok(Some(Some(answer)));
            }
            (KeyCode::Char('j'), modifiers) if modifiers.contains(KeyModifiers::CONTROL) => {
                let answer = self.content.clone();
                self.finish(Some(&answer))?;
                return Ok(Some(Some(answer)));
            }
            (KeyCode::Backspace, _) => self.delete_before_cursor(),
            (KeyCode::Delete, modifiers) if modifiers.contains(KeyModifiers::CONTROL) => {
                self.delete_word_after_cursor();
            }
            (KeyCode::Delete, _) => self.delete_after_cursor(),
            (KeyCode::Left, modifiers) if modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_to_previous_word();
            }
            (KeyCode::Left, _) => self.move_left(),
            (KeyCode::Right, modifiers) if modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_to_next_word();
            }
            (KeyCode::Right, _) => self.move_right(),
            (KeyCode::Home, _) => self.cursor = 0,
            (KeyCode::End, _) => self.cursor = self.content.len(),
            (KeyCode::Char('a'), modifiers) if modifiers.contains(KeyModifiers::CONTROL) => {
                self.cursor = 0;
            }
            (KeyCode::Char('e'), modifiers) if modifiers.contains(KeyModifiers::CONTROL) => {
                self.cursor = self.content.len();
            }
            (KeyCode::Char('h'), modifiers) if modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(None);
            }
            (KeyCode::Char(character), _) => self.insert_char(character),
            _ => return Ok(None),
        }

        self.render()?;
        Ok(None)
    }

    fn insert_char(&mut self, character: char) {
        if character.is_control() && character != '\n' {
            return;
        }

        self.content.insert(self.cursor, character);
        self.cursor += character.len_utf8();
        self.normalize_cursor();
    }

    fn insert_str(&mut self, value: &str) {
        self.content.insert_str(self.cursor, value);
        self.cursor += value.len();
        self.normalize_cursor();
    }

    fn insert_pasted_text(&mut self, value: &str) {
        let normalized: String = value
            .replace("\r\n", "\n")
            .replace('\r', "\n")
            .chars()
            .filter(|character| !character.is_control() || *character == '\n')
            .collect();
        self.insert_str(&normalized);
    }

    fn normalize_cursor(&mut self) {
        self.cursor = self
            .content
            .grapheme_indices(true)
            .map(|(index, _)| index)
            .find(|index| *index >= self.cursor)
            .unwrap_or(self.content.len());
    }

    fn move_left(&mut self) {
        self.cursor = self.content[..self.cursor]
            .grapheme_indices(true)
            .next_back()
            .map_or(0, |(index, _)| index);
    }

    fn move_right(&mut self) {
        if let Some(grapheme) = self.content[self.cursor..].graphemes(true).next() {
            self.cursor += grapheme.len();
        }
    }

    fn move_to_previous_word(&mut self) {
        self.cursor = previous_word_index(&self.content, self.cursor);
    }

    fn move_to_next_word(&mut self) {
        self.cursor = next_word_index(&self.content, self.cursor);
    }

    fn delete_before_cursor(&mut self) {
        let previous = self.content[..self.cursor]
            .grapheme_indices(true)
            .next_back()
            .map_or(0, |(index, _)| index);
        self.content.drain(previous..self.cursor);
        self.cursor = previous;
        self.normalize_cursor();
    }

    fn delete_after_cursor(&mut self) {
        if let Some(grapheme) = self.content[self.cursor..].graphemes(true).next() {
            self.content
                .drain(self.cursor..self.cursor + grapheme.len());
            self.normalize_cursor();
        }
    }

    fn delete_word_after_cursor(&mut self) {
        self.content
            .drain(self.cursor..next_word_index(&self.content, self.cursor));
        self.normalize_cursor();
    }

    fn render(&mut self) -> io::Result<()> {
        let mut stderr = io::stderr();
        self.clear(&mut stderr)?;

        queue!(
            stderr,
            Hide,
            Print("? "),
            SetAttribute(Attribute::Bold),
            Print(self.message),
            SetAttribute(Attribute::Reset),
            Print(" ")
        )?;

        write_multiline(&mut stderr, &self.content[..self.cursor])?;
        stderr.execute(SavePosition)?;
        write_multiline(&mut stderr, &self.content[self.cursor..])?;
        if self.cursor == self.content.len() {
            stderr.execute(Print(' '))?;
        }
        queue!(
            stderr,
            Print("\r\n["),
            Print(self.help_message),
            Print(']'),
            RestorePosition,
            Show
        )?;
        stderr.flush()?;

        self.cursor_row = self.calculate_cursor_row();
        Ok(())
    }

    fn render_after_resize(&mut self) -> io::Result<()> {
        self.cursor_row = self.calculate_cursor_row();
        self.render()
    }

    fn finish(&mut self, answer: Option<&str>) -> io::Result<()> {
        let mut stderr = io::stderr();
        self.clear(&mut stderr)?;
        queue!(
            stderr,
            Show,
            Print("> "),
            SetAttribute(Attribute::Bold),
            Print(self.message),
            SetAttribute(Attribute::Reset)
        )?;
        if let Some(answer) = answer {
            stderr.execute(Print(' '))?;
            write_multiline(&mut stderr, answer)?;
        }
        stderr.execute(Print("\r\n"))?;
        stderr.flush()
    }

    fn clear(&self, stderr: &mut io::Stderr) -> io::Result<()> {
        queue!(stderr, Hide)?;
        if self.cursor_row > 0 {
            queue!(stderr, MoveUp(self.cursor_row))?;
        }
        queue!(stderr, MoveToColumn(0), Clear(ClearType::FromCursorDown))
    }

    fn calculate_cursor_row(&self) -> u16 {
        let terminal_width = match terminal::size() {
            Ok((0, _)) | Err(_) => u16::MAX,
            Ok((width, _)) => width,
        };
        self.calculate_cursor_row_for_width(terminal_width)
    }

    fn calculate_cursor_row_for_width(&self, terminal_width: u16) -> u16 {
        let width = usize::from(terminal_width.max(1));
        let mut position = DisplayPosition::default();

        for value in ["? ", self.message, " ", &self.content[..self.cursor]] {
            for grapheme in value.graphemes(true) {
                position.advance(grapheme, width);
            }
        }

        position.row
    }
}

#[derive(Debug, Default)]
struct DisplayPosition {
    column: usize,
    row: u16,
}

impl DisplayPosition {
    fn advance(&mut self, grapheme: &str, terminal_width: usize) {
        if grapheme == "\n" {
            self.row = self.row.saturating_add(1);
            self.column = 0;
            return;
        }

        let grapheme_width = UnicodeWidthStr::width(grapheme);
        if grapheme_width == 0 {
            return;
        }

        if self.column == terminal_width
            || (self.column > 0 && grapheme_width > terminal_width - self.column)
        {
            self.row = self.row.saturating_add(1);
            self.column = 0;
        }

        let occupied = self.column.saturating_add(grapheme_width);
        self.row = self.row.saturating_add(
            u16::try_from(occupied.saturating_sub(1) / terminal_width).unwrap_or(u16::MAX),
        );
        self.column = match occupied % terminal_width {
            0 => terminal_width,
            column => column,
        };
    }
}

fn next_word_index(content: &str, cursor: usize) -> usize {
    let mut seen_word = false;

    for (offset, grapheme) in content[cursor..].grapheme_indices(true) {
        if is_alphanumeric(grapheme) {
            seen_word = true;
        } else if seen_word {
            return cursor + offset;
        }
    }

    content.len()
}

fn previous_word_index(content: &str, cursor: usize) -> usize {
    let mut seen_word = false;

    for (index, grapheme) in content[..cursor].grapheme_indices(true).rev() {
        if is_alphanumeric(grapheme) {
            seen_word = true;
        } else if seen_word {
            return index + grapheme.len();
        }
    }

    0
}

fn is_alphanumeric(grapheme: &str) -> bool {
    grapheme.unicode_words().next().is_some()
}

fn write_multiline(writer: &mut impl Write, value: &str) -> io::Result<()> {
    for segment in value.split_inclusive('\n') {
        if let Some(content) = segment.strip_suffix('\n') {
            queue!(writer, Print(content), Print("\r\n"))?;
        } else {
            writer.execute(Print(segment))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_terminal_capability_is_optional() {
        let unsupported = io::Error::new(io::ErrorKind::Unsupported, "unsupported");
        assert!(matches!(optional_capability(Err(unsupported)), Ok(false)));
        assert!(matches!(optional_capability(Ok(())), Ok(true)));
    }

    #[test]
    fn terminal_capability_propagates_other_errors() {
        let error = io::Error::new(io::ErrorKind::BrokenPipe, "broken pipe");
        assert_eq!(
            optional_capability(Err(error)).map_err(|error| error.kind()),
            Err(io::ErrorKind::BrokenPipe)
        );
    }

    #[test]
    fn cursor_row_uses_delayed_terminal_wrapping() {
        let mut prompt = PromptState::new("", "");
        prompt.insert_str("ab");
        assert_eq!(prompt.calculate_cursor_row_for_width(5), 0);

        prompt.insert_char('c');
        assert_eq!(prompt.calculate_cursor_row_for_width(5), 1);
    }

    #[test]
    fn cursor_row_tracks_explicit_newlines_and_wide_graphemes() {
        let mut newline_prompt = PromptState::new("", "");
        newline_prompt.insert_str("ab\n");
        assert_eq!(newline_prompt.calculate_cursor_row_for_width(5), 1);

        let mut wide_prompt = PromptState::new("", "");
        wide_prompt.insert_char('界');
        assert_eq!(wide_prompt.calculate_cursor_row_for_width(4), 1);
    }

    #[test]
    fn word_navigation_uses_unicode_word_boundaries() {
        let content = "one, 世界 three";
        assert_eq!(next_word_index(content, 0), 3);
        assert_eq!(next_word_index(content, 3), 11);
        assert_eq!(previous_word_index(content, content.len()), 12);
        assert_eq!(previous_word_index(content, 11), 5);
    }

    #[test]
    fn pasted_newlines_are_normalized() {
        let mut prompt = PromptState::new("", "");
        prompt.insert_pasted_text("one\t\r\ntwo\x1b[31m\u{7}\rthree");
        assert_eq!(prompt.content, "one\ntwo[31m\nthree");
        assert_eq!(prompt.cursor, prompt.content.len());
    }

    #[test]
    fn insertion_keeps_cursor_on_a_whole_string_grapheme_boundary() {
        let mut prompt = PromptState::new("", "");
        prompt.insert_str("👩👩");
        prompt.cursor = "👩".len();

        prompt.insert_char('\u{200d}');

        assert_eq!(prompt.content, "👩‍👩");
        assert_eq!(prompt.cursor, prompt.content.len());
        assert_eq!(prompt.content.graphemes(true).count(), 1);
    }
}
