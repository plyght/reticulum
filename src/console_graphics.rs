use crate::constants::{
    LOGO_ASCII_ART, START_MESSAGE_LINE, USER_INPUT_PROMPT, USER_INPUT_PROMPT_LENGTH,
};
use crate::message::Message;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    execute, queue, style,
    terminal::{self, ClearType},
};
use std::io::{stdout, Write};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct GraphicsEngine {
    height: usize,
    width: usize,
    previous_height: usize,
    previous_width: usize,
    max_message_lines: usize,
    message_lines: Vec<String>,
}

impl Clone for GraphicsEngine {
    fn clone(&self) -> Self {
        Self {
            height: self.height,
            width: self.width,
            previous_height: self.previous_height,
            previous_width: self.previous_width,
            max_message_lines: self.max_message_lines,
            message_lines: self.message_lines.clone(),
        }
    }
}

impl GraphicsEngine {
    pub fn new(max_message_lines: usize) -> Self {
        let (width, height) = terminal::size().unwrap_or((80, 24));

        Self {
            height: height as usize,
            width: width as usize,
            previous_height: height as usize,
            previous_width: width as usize,
            max_message_lines,
            message_lines: Vec::new(),
        }
    }

    pub fn update_resolution(&mut self) {
        if let Ok((width, height)) = terminal::size() {
            self.width = width as usize;
            self.height = height as usize;
        }
    }

    // Make the compiler ignore this warning as we might need this function in the future
    #[allow(dead_code)]
    fn move_cursor(&mut self, height: usize, width: usize) -> std::io::Result<()> {
        self.update_resolution();

        let y_position = self.height.saturating_sub(height).saturating_sub(1);
        let x_position = width;

        execute!(
            stdout(),
            cursor::MoveTo(x_position as u16, y_position as u16)
        )
    }

    pub fn specific_line_print(&mut self, text: &str, line_height: usize) -> std::io::Result<()> {
        self.update_resolution();

        if line_height >= self.height {
            return Ok(());
        }

        let y_position = self.height.saturating_sub(line_height).saturating_sub(1);

        let mut stdout = stdout();

        // Save cursor position
        queue!(stdout, cursor::SavePosition)?;

        // Move to the specific line
        queue!(stdout, cursor::MoveTo(0, y_position as u16))?;

        // Clear the line
        queue!(stdout, terminal::Clear(ClearType::CurrentLine))?;

        // Print the text
        queue!(stdout, style::Print(text))?;

        // Restore cursor position
        queue!(stdout, cursor::RestorePosition)?;

        stdout.flush()
    }

    pub fn add_message(&mut self, message: &Message) {
        // Format sender info differently for local messages
        let message_text = if message.sender_ip() == "local" {
            format!("YOU >>> {}: {}", message.sender_name(), message.content())
        } else {
            format!(
                "{} >>> {}: {}",
                message.sender_ip(),
                message.sender_name(),
                message.content()
            )
        };

        self.message_lines.push(message_text);

        if self.message_lines.len() > self.max_message_lines {
            self.message_lines.remove(0);
        }
    }

    pub fn print_all_messages(&mut self, reserve_space: bool) -> std::io::Result<()> {
        self.update_resolution();

        if reserve_space {
            for _ in 0..self.max_message_lines + START_MESSAGE_LINE - 1 {
                println!();
            }
        }

        let current_buffer_vector_size = self.message_lines.len();
        let messages_copy = self.message_lines.clone();

        for i in 0..current_buffer_vector_size {
            let message_idx = current_buffer_vector_size - i - 1;
            if message_idx < messages_copy.len() {
                self.specific_line_print(&messages_copy[message_idx], START_MESSAGE_LINE + i)?;
            }
        }

        Ok(())
    }

    pub fn clear_console() -> std::io::Result<()> {
        execute!(
            stdout(),
            terminal::Clear(ClearType::All),
            cursor::MoveTo(0, 0)
        )
    }

    pub fn console_format_keeper(graphics_engine: Arc<Mutex<GraphicsEngine>>) {
        loop {
            let mut should_clear = false;
            let mut should_print = false;

            {
                let mut engine = graphics_engine.lock().unwrap();
                engine.update_resolution();

                if engine.previous_height != engine.height || engine.previous_width != engine.width
                {
                    should_clear = true;
                    engine.previous_height = engine.height;
                    engine.previous_width = engine.width;
                    should_print = true;
                }
            }

            if should_clear {
                let _ = Self::clear_console();
            }

            if should_print {
                let mut engine = graphics_engine.lock().unwrap();
                let _ = engine.print_all_messages(true);
                let _ = engine.print_input_prompt();
            }

            std::thread::sleep(Duration::from_millis(100));
        }
    }

    pub fn print_input_prompt(&mut self) -> std::io::Result<()> {
        self.specific_line_print(USER_INPUT_PROMPT, 0)?;
        let height = self.height;
        execute!(
            stdout(),
            cursor::MoveTo(USER_INPUT_PROMPT_LENGTH as u16, (height - 1) as u16)
        )
    }

    pub fn print_logo() -> std::io::Result<()> {
        println!("{}", LOGO_ASCII_ART);
        Ok(())
    }

    pub fn setup_terminal() -> std::io::Result<()> {
        terminal::enable_raw_mode()?;
        execute!(stdout(), terminal::EnterAlternateScreen)?;
        Ok(())
    }

    // This method will be called when properly handling program exit
    #[allow(dead_code)]
    pub fn restore_terminal() -> std::io::Result<()> {
        terminal::disable_raw_mode()?;
        execute!(stdout(), terminal::LeaveAlternateScreen)?;
        Ok(())
    }

    pub fn read_input(input: &mut String) -> std::io::Result<(bool, bool)> {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(KeyEvent {
                code, modifiers, ..
            }) = event::read()?
            {
                match code {
                    KeyCode::Enter => return Ok((true, false)),
                    KeyCode::Char('q') if modifiers == event::KeyModifiers::CONTROL => {
                        // Ctrl+Q exits the application
                        return Ok((false, true));
                    }
                    KeyCode::Char('c') if modifiers == event::KeyModifiers::CONTROL => {
                        // Ctrl+C also exits the application
                        return Ok((false, true));
                    }
                    KeyCode::Char(c) => {
                        input.push(c);
                        print!("{}", c);
                        stdout().flush()?;
                    }
                    KeyCode::Backspace => {
                        if !input.is_empty() {
                            input.pop();
                            execute!(
                                stdout(),
                                cursor::MoveLeft(1),
                                terminal::Clear(ClearType::UntilNewLine)
                            )?;
                        }
                    }
                    KeyCode::Esc => return Ok((false, true)),
                    _ => {}
                }
            }
        }
        Ok((false, false))
    }
}
