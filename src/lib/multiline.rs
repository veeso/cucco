use std::io::{self, Write};

use crossterm::cursor::{Hide, MoveToColumn, MoveUp, RestorePosition, SavePosition, Show};
use crossterm::event::{
    self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyEvent, KeyEventKind,
    KeyModifiers,
};
use crossterm::style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor};
use crossterm::terminal::{self, Clear, ClearType};
use crossterm::{ExecutableCommand, queue};
use inquire::CustomUserError;
use inquire::error::{InquireError, InquireResult};
use inquire::validator::{ErrorMessage, Validation};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Checks submitted content and returns the message to show when it is rejected.
type Validator = fn(&str) -> Result<Validation, CustomUserError>;

/// Help line shown while a suggestion can be filled in.
const SUGGESTION_HELP: &str = "<tab> or <right> to use the suggestion";

/// Multi-line text prompt that can be skipped.
///
/// Alt+enter inserts a newline, enter submits.
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

    /// Runs the prompt until it is submitted or skipped.
    ///
    /// # Errors
    ///
    /// Returns [`InquireError::OperationInterrupted`] on ctrl+c, and any I/O
    /// error raised by the terminal.
    pub(crate) fn prompt_skippable(self) -> InquireResult<Option<String>> {
        run(PromptState::new(self.message, self.help_message))
    }
}

/// Single-line text prompt with a greyed-out suggestion.
///
/// While the input is empty, tab or the right arrow copy the suggestion into
/// the input, where it can be edited like typed text. The look follows the
/// inquire defaults: bold message, dark grey suggestion, red `# ` error.
#[derive(Debug)]
pub(crate) struct SuggestedText<'a> {
    message: &'a str,
    suggestion: &'a str,
    validator: Validator,
}

impl<'a> SuggestedText<'a> {
    /// Creates the prompt. An empty `suggestion` shows nothing and no help line.
    pub(crate) fn new(message: &'a str, suggestion: &'a str, validator: Validator) -> Self {
        Self {
            message,
            suggestion,
            validator,
        }
    }

    /// Runs the prompt until the validator accepts the submitted content.
    ///
    /// # Errors
    ///
    /// Returns [`InquireError::OperationCanceled`] when the prompt is skipped
    /// with esc, and [`InquireError::OperationInterrupted`] on ctrl+c.
    pub(crate) fn prompt(self) -> InquireResult<String> {
        let state = PromptState::single_line(self.message, self.suggestion, self.validator);

        run(state)?.ok_or(InquireError::OperationCanceled)
    }
}

/// Renders `prompt` and feeds it terminal events until it is answered or skipped.
fn run(mut prompt: PromptState<'_>) -> InquireResult<Option<String>> {
    let _terminal_session = TerminalSession::enable()?;
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
    error: Option<String>,
    help_message: &'a str,
    message: &'a str,
    mode: Mode,
}

/// Kind of prompt a [`PromptState`] drives.
#[derive(Debug)]
enum Mode {
    /// Free text where alt+enter inserts a newline.
    Multiline,
    /// One line of text with an optional suggestion, checked before submit.
    SingleLine {
        suggestion: String,
        validator: Validator,
    },
}

impl<'a> PromptState<'a> {
    fn new(message: &'a str, help_message: &'a str) -> Self {
        Self {
            content: String::new(),
            cursor: 0,
            cursor_row: 0,
            error: None,
            help_message,
            message,
            mode: Mode::Multiline,
        }
    }

    /// Builds a single-line prompt.
    ///
    /// The suggestion is flattened to one line without control characters, so
    /// it can never put into the input what typing would reject. The help line
    /// is shown only when a suggestion is left.
    fn single_line(message: &'a str, suggestion: &str, validator: Validator) -> Self {
        let suggestion = flatten_to_line(suggestion);
        let help_message = if suggestion.is_empty() {
            ""
        } else {
            SUGGESTION_HELP
        };

        Self {
            mode: Mode::SingleLine {
                suggestion,
                validator,
            },
            ..Self::new(message, help_message)
        }
    }

    fn is_multiline(&self) -> bool {
        matches!(self.mode, Mode::Multiline)
    }

    /// Returns the help line to render. The suggestion hint goes away as soon
    /// as the input is not empty, because tab no longer fills it.
    fn visible_help(&self) -> &str {
        if self.is_multiline() || self.content.is_empty() {
            self.help_message
        } else {
            ""
        }
    }

    fn suggestion(&self) -> &str {
        match &self.mode {
            Mode::Multiline => "",
            Mode::SingleLine { suggestion, .. } => suggestion,
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> InquireResult<Option<Option<String>>> {
        self.error = None;

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
                if self.is_multiline() && modifiers.contains(KeyModifiers::ALT) =>
            {
                self.insert_char('\n');
            }
            (KeyCode::Enter | KeyCode::Char('\n' | '\r'), _) => return self.submit(),
            (KeyCode::Char('j'), modifiers) if modifiers.contains(KeyModifiers::CONTROL) => {
                return self.submit();
            }
            (KeyCode::Tab, KeyModifiers::NONE) => {
                self.accept_suggestion();
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
            (KeyCode::Right, modifiers) => {
                if modifiers != KeyModifiers::NONE || !self.accept_suggestion() {
                    self.move_right();
                }
            }
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

    /// Finishes the prompt with the content, or shows why it was rejected.
    fn submit(&mut self) -> InquireResult<Option<Option<String>>> {
        let rejection = match self.validation_error() {
            Ok(rejection) => rejection,
            Err(error) => {
                self.finish(None)?;
                return Err(error);
            }
        };
        if let Some(error) = rejection {
            self.error = Some(error);
            self.render()?;
            return Ok(None);
        }

        let answer = self.content.clone();
        self.finish(Some(&answer))?;
        Ok(Some(Some(answer)))
    }

    /// Returns the message to show when the validator rejects the content.
    fn validation_error(&self) -> InquireResult<Option<String>> {
        let Mode::SingleLine { validator, .. } = &self.mode else {
            return Ok(None);
        };

        match validator(&self.content).map_err(InquireError::Custom)? {
            Validation::Valid => Ok(None),
            Validation::Invalid(ErrorMessage::Custom(message)) => Ok(Some(message)),
            Validation::Invalid(ErrorMessage::Default) => Ok(Some("Invalid input.".into())),
        }
    }

    /// Copies the suggestion into the empty input and reports whether it did.
    fn accept_suggestion(&mut self) -> bool {
        let Mode::SingleLine { suggestion, .. } = &self.mode else {
            return false;
        };
        if !self.content.is_empty() || suggestion.is_empty() {
            return false;
        }

        self.content.push_str(suggestion);
        self.cursor = self.content.len();
        true
    }

    fn insert_char(&mut self, character: char) {
        if character.is_control() && !(self.is_multiline() && character == '\n') {
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

    /// Inserts pasted text without control characters. Line breaks are kept in
    /// the multi-line prompt and collapse into single spaces in the single-line
    /// one; tabs are dropped.
    fn insert_pasted_text(&mut self, value: &str) {
        if self.is_multiline() {
            let normalized: String = value
                .replace("\r\n", "\n")
                .replace('\r', "\n")
                .chars()
                .filter(|character| !character.is_control() || *character == '\n')
                .collect();
            self.insert_str(&normalized);
        } else {
            self.insert_str(&flatten_to_line(value));
        }
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
        if self.content.is_empty() && !self.suggestion().is_empty() {
            queue!(
                stderr,
                SetForegroundColor(Color::DarkGrey),
                Print(self.suggestion()),
                ResetColor
            )?;
        } else {
            write_multiline(&mut stderr, &self.content[self.cursor..])?;
        }
        if self.cursor == self.content.len() {
            stderr.execute(Print(' '))?;
        }
        if let Some(error) = &self.error {
            queue!(
                stderr,
                Print("\r\n"),
                SetForegroundColor(Color::Red),
                Print("# "),
                Print(error),
                ResetColor
            )?;
        }
        let help = self.visible_help();
        if !help.is_empty() {
            queue!(stderr, Print("\r\n["), Print(help), Print(']'))?;
        }
        queue!(stderr, RestorePosition, Show)?;
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

/// Joins the lines of `value` into one, dropping control characters.
///
/// Whitespace next to a line break is trimmed and empty lines disappear, so
/// the lines are separated by exactly one space. Text without a line break is
/// returned as is.
fn flatten_to_line(value: &str) -> String {
    let normalized = value.replace("\r\n", "\n").replace('\r', "\n");
    let cleaned: String = normalized
        .chars()
        .filter(|character| !character.is_control() || *character == '\n')
        .collect();
    let last = cleaned.matches('\n').count();

    cleaned
        .split('\n')
        .enumerate()
        .map(|(index, line)| {
            let line = if index > 0 { line.trim_start() } else { line };
            if index < last { line.trim_end() } else { line }
        })
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
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

    fn prompt_with(content: &str) -> PromptState<'static> {
        let mut prompt = PromptState::new("", "");
        prompt.insert_str(content);
        prompt
    }

    fn required(input: &str) -> Result<Validation, CustomUserError> {
        if input.is_empty() {
            Ok(Validation::Invalid("required".into()))
        } else {
            Ok(Validation::Valid)
        }
    }

    fn single_line_prompt(suggestion: &'static str) -> PromptState<'static> {
        PromptState::single_line("", suggestion, required)
    }

    #[test]
    fn single_line_accept_suggestion_fills_empty_content() {
        let mut prompt = single_line_prompt("añade 🚀 soporte");

        assert!(prompt.accept_suggestion());

        assert_eq!(prompt.content, "añade 🚀 soporte");
        assert_eq!(prompt.cursor, prompt.content.len());
    }

    #[test]
    fn single_line_accept_suggestion_is_inert_without_a_suggestion() {
        let mut single_line = single_line_prompt("");
        assert!(!single_line.accept_suggestion());
        assert_eq!(single_line.content, "");

        let mut multiline = PromptState::new("", "");
        assert!(!multiline.accept_suggestion());
        assert_eq!(multiline.content, "");
    }

    #[test]
    fn single_line_prompt_never_stores_newlines() {
        let mut prompt = single_line_prompt("");

        prompt.insert_char('a');
        prompt.insert_char('\n');
        prompt.insert_pasted_text("b\r\nc\rd\ne");

        assert_eq!(prompt.content, "ab c d e");
        assert_eq!(prompt.cursor, prompt.content.len());
    }

    #[test]
    fn single_line_validation_error_reports_rejected_content() {
        let mut prompt = single_line_prompt("delete some stars");
        assert_eq!(
            prompt.validation_error().unwrap(),
            Some("required".to_string())
        );

        prompt.insert_str("fix");
        assert_eq!(prompt.validation_error().unwrap(), None);

        let multiline = PromptState::new("", "");
        assert_eq!(multiline.validation_error().unwrap(), None);
    }

    #[test]
    fn single_line_validation_error_uses_default_message() {
        fn rejecting(_: &str) -> Result<Validation, CustomUserError> {
            Ok(Validation::Invalid(ErrorMessage::Default))
        }

        let prompt = PromptState::single_line("", "", rejecting);

        assert_eq!(
            prompt.validation_error().unwrap(),
            Some("Invalid input.".to_string())
        );
    }

    #[test]
    fn single_line_validation_error_propagates_validator_failures() {
        fn failing(_: &str) -> Result<Validation, CustomUserError> {
            Err("boom".into())
        }

        let prompt = PromptState::single_line("", "", failing);

        assert!(matches!(
            prompt.validation_error(),
            Err(InquireError::Custom(_))
        ));
    }

    fn press(prompt: &mut PromptState<'_>, code: KeyCode, modifiers: KeyModifiers) {
        let answer = prompt.handle_key(KeyEvent::new(code, modifiers)).unwrap();
        assert_eq!(answer, None, "the key must not end the prompt");
    }

    #[test]
    fn single_line_tab_and_right_fill_the_suggestion() {
        for code in [KeyCode::Tab, KeyCode::Right] {
            let mut prompt = single_line_prompt("delete some stars");

            press(&mut prompt, code, KeyModifiers::NONE);

            assert_eq!(prompt.content, "delete some stars");
            assert_eq!(prompt.cursor, prompt.content.len());
        }
    }

    #[test]
    fn single_line_tab_with_typed_content_is_inert() {
        let mut prompt = single_line_prompt("delete some stars");
        prompt.insert_str("fix");

        press(&mut prompt, KeyCode::Tab, KeyModifiers::NONE);

        assert_eq!(prompt.content, "fix");
        assert_eq!(prompt.cursor, 3);
    }

    #[test]
    fn single_line_right_with_typed_content_moves_the_cursor() {
        let mut prompt = single_line_prompt("delete some stars");
        prompt.insert_str("fix");
        prompt.cursor = 0;

        press(&mut prompt, KeyCode::Right, KeyModifiers::NONE);

        assert_eq!(prompt.content, "fix");
        assert_eq!(prompt.cursor, 1);
    }

    #[test]
    fn multiline_tab_and_right_ignore_the_suggestion() {
        let mut prompt = PromptState::new("", "");

        press(&mut prompt, KeyCode::Tab, KeyModifiers::NONE);
        press(&mut prompt, KeyCode::Right, KeyModifiers::NONE);

        assert_eq!(prompt.content, "");
    }

    #[test]
    fn single_line_alt_enter_submits_instead_of_inserting_a_newline() {
        let mut prompt = single_line_prompt("");
        prompt.insert_str("fix");

        let answer = prompt
            .handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::ALT))
            .unwrap();

        assert_eq!(answer, Some(Some("fix".to_string())));
    }

    #[test]
    fn multiline_alt_enter_inserts_a_newline() {
        let mut prompt = PromptState::new("", "");

        press(&mut prompt, KeyCode::Enter, KeyModifiers::ALT);

        assert_eq!(prompt.content, "\n");
    }

    #[test]
    fn single_line_ctrl_j_validates_before_submitting() {
        let mut prompt = single_line_prompt("");

        press(&mut prompt, KeyCode::Char('j'), KeyModifiers::CONTROL);
        assert_eq!(prompt.error.as_deref(), Some("required"));

        prompt.insert_str("fix");
        let answer = prompt
            .handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL))
            .unwrap();
        assert_eq!(answer, Some(Some("fix".to_string())));
    }

    #[test]
    fn single_line_validation_error_is_cleared_by_the_next_key() {
        let mut prompt = single_line_prompt("");
        press(&mut prompt, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(prompt.error.as_deref(), Some("required"));

        press(&mut prompt, KeyCode::Char('f'), KeyModifiers::NONE);

        assert_eq!(prompt.error, None);
        assert_eq!(prompt.content, "f");
    }

    #[test]
    fn single_line_suggestion_is_flattened_and_stripped_of_control_characters() {
        let prompt = single_line_prompt("add\nstars\r\n\tnow\u{1b}[0m");

        assert_eq!(prompt.suggestion(), "add stars now[0m");
        assert_eq!(prompt.help_message, SUGGESTION_HELP);
    }

    #[test]
    fn single_line_suggestion_without_printable_text_shows_no_help() {
        let prompt = single_line_prompt("\t\u{1b}");

        assert_eq!(prompt.suggestion(), "");
        assert_eq!(prompt.help_message, "");
    }

    #[test]
    fn single_line_modified_tab_and_right_keep_the_suggestion_unused() {
        for (code, modifiers) in [
            (KeyCode::Tab, KeyModifiers::SHIFT),
            (KeyCode::Right, KeyModifiers::SHIFT),
            (KeyCode::Right, KeyModifiers::ALT),
        ] {
            let mut prompt = single_line_prompt("delete some stars");

            press(&mut prompt, code, modifiers);

            assert_eq!(prompt.content, "", "{code:?} with {modifiers:?}");
        }
    }

    #[test]
    fn single_line_suggestion_returns_after_the_input_is_emptied() {
        let mut prompt = single_line_prompt("delete some stars");
        press(&mut prompt, KeyCode::Char('f'), KeyModifiers::NONE);
        press(&mut prompt, KeyCode::Backspace, KeyModifiers::NONE);

        press(&mut prompt, KeyCode::Tab, KeyModifiers::NONE);

        assert_eq!(prompt.content, "delete some stars");
    }

    #[test]
    fn single_line_help_is_hidden_once_the_input_is_not_empty() {
        let mut prompt = single_line_prompt("delete some stars");
        assert_eq!(prompt.visible_help(), SUGGESTION_HELP);

        prompt.insert_str("fix");
        assert_eq!(prompt.visible_help(), "");

        let multiline = PromptState::new("", "help");
        assert_eq!(multiline.visible_help(), "help");
        let mut typed = PromptState::new("", "help");
        typed.insert_str("text");
        assert_eq!(typed.visible_help(), "help");
    }

    #[test]
    fn single_line_paste_joins_lines_with_single_spaces() {
        for (pasted, expected) in [
            ("\nfix thing\n", "fix thing"),
            ("a\n\nb", "a b"),
            ("a \r\n  b", "a b"),
            ("keep trailing ", "keep trailing "),
        ] {
            let mut prompt = single_line_prompt("");

            prompt.insert_pasted_text(pasted);

            assert_eq!(prompt.content, expected, "pasted {pasted:?}");
        }
    }

    #[test]
    fn escape_and_ctrl_c_end_the_single_line_prompt() {
        let mut escaped = single_line_prompt("delete some stars");
        let answer = escaped
            .handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
            .unwrap();
        assert_eq!(answer, Some(None));

        let mut interrupted = single_line_prompt("delete some stars");
        let error = interrupted
            .handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL))
            .unwrap_err();
        assert!(matches!(error, InquireError::OperationInterrupted));
    }

    #[test]
    fn single_line_validator_failure_ends_the_prompt_with_the_error() {
        fn failing(_: &str) -> Result<Validation, CustomUserError> {
            Err("boom".into())
        }

        let mut prompt = PromptState::single_line("", "", failing);

        let error = prompt
            .handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .unwrap_err();

        assert!(matches!(error, InquireError::Custom(_)));
    }

    #[test]
    fn insert_char_ignores_control_characters_but_keeps_newlines() {
        let mut prompt = prompt_with("");
        prompt.insert_char('a');
        prompt.insert_char('\u{7}');
        prompt.insert_char('\n');
        prompt.insert_char('b');

        assert_eq!(prompt.content, "a\nb");
        assert_eq!(prompt.cursor, 3);
    }

    #[test]
    fn cursor_moves_by_grapheme() {
        let mut prompt = prompt_with("a👩\u{200d}👩b");

        prompt.move_left();
        assert_eq!(prompt.cursor, "a👩\u{200d}👩".len());
        prompt.move_left();
        assert_eq!(prompt.cursor, "a".len());
        prompt.move_left();
        prompt.move_left();
        assert_eq!(prompt.cursor, 0);

        prompt.move_right();
        prompt.move_right();
        assert_eq!(prompt.cursor, "a👩\u{200d}👩".len());
        prompt.move_right();
        prompt.move_right();
        assert_eq!(prompt.cursor, prompt.content.len());
    }

    #[test]
    fn backspace_removes_the_previous_grapheme() {
        let mut prompt = prompt_with("ab👩\u{200d}👩");

        prompt.delete_before_cursor();
        assert_eq!(prompt.content, "ab");
        prompt.delete_before_cursor();
        prompt.delete_before_cursor();
        prompt.delete_before_cursor();
        assert_eq!(prompt.content, "");
        assert_eq!(prompt.cursor, 0);
    }

    #[test]
    fn delete_removes_the_next_grapheme_and_word() {
        let mut prompt = prompt_with("one two three");
        prompt.cursor = 0;

        prompt.delete_after_cursor();
        assert_eq!(prompt.content, "ne two three");

        prompt.delete_word_after_cursor();
        assert_eq!(prompt.content, " two three");
        assert_eq!(prompt.cursor, 0);

        prompt.cursor = prompt.content.len();
        prompt.delete_after_cursor();
        prompt.delete_word_after_cursor();
        assert_eq!(prompt.content, " two three");
    }

    #[test]
    fn word_moves_update_the_cursor() {
        let mut prompt = prompt_with("one two");
        prompt.cursor = 0;

        prompt.move_to_next_word();
        assert_eq!(prompt.cursor, 3);
        prompt.move_to_next_word();
        assert_eq!(prompt.cursor, 7);
        prompt.move_to_previous_word();
        assert_eq!(prompt.cursor, 4);
        prompt.move_to_previous_word();
        assert_eq!(prompt.cursor, 0);
    }

    #[test]
    fn display_position_wraps_and_counts_widths() {
        let mut position = DisplayPosition::default();
        for grapheme in ["a", "b", "c", "d"] {
            position.advance(grapheme, 3);
        }
        assert_eq!((position.row, position.column), (1, 1));

        let mut position = DisplayPosition::default();
        position.advance("\n", 3);
        assert_eq!((position.row, position.column), (1, 0));

        let mut position = DisplayPosition::default();
        position.advance("\u{301}", 3);
        assert_eq!((position.row, position.column), (0, 0));
    }

    #[test]
    fn write_multiline_uses_carriage_returns() {
        let mut output = Vec::new();
        write_multiline(&mut output, "one\ntwo").unwrap();

        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("one\r\n"));
        assert!(output.ends_with("two"));
    }
}
