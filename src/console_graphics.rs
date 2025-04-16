use crate::constants::{
    COMMON_COMMANDS, LOGO_ASCII_ART, START_MESSAGE_LINE, STATUS_BAR_LINE, USER_INPUT_PROMPT,
    USER_INPUT_PROMPT_LENGTH,
};
use crate::message::Message;
use chrono::Local;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    execute, queue,
    style::{self, Color, SetBackgroundColor, SetForegroundColor},
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
    input_history: Vec<String>,
    history_position: usize,
    current_input: String,
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
            input_history: self.input_history.clone(),
            history_position: self.history_position,
            current_input: self.current_input.clone(),
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
            input_history: Vec::with_capacity(50),
            history_position: 0,
            current_input: String::new(),
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
            let timestamp = chrono::Local::now().format("%H:%M:%S");
            format!(
                "[{}] YOU >>> {}: {}",
                timestamp,
                message.sender_name(),
                message.content()
            )
        } else {
            let timestamp = chrono::Local::now().format("%H:%M:%S");
            format!(
                "[{}] {} >>> {}: {}",
                timestamp,
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
                let _ = engine.print_status_bar();
                let _ = engine.print_input_prompt();
            }

            // Update status bar every second
            let mut engine = graphics_engine.lock().unwrap();
            let _ = engine.print_status_bar();
            drop(engine);

            std::thread::sleep(Duration::from_millis(100));
        }
    }

    pub fn print_status_bar(&mut self) -> std::io::Result<()> {
        self.update_resolution();

        // Get current time
        let now = Local::now();
        let time_str = now.format("%H:%M:%S").to_string();
        let date_str = now.format("%Y-%m-%d").to_string();

        // Calculate spaces for centering and padding
        let terminal_info = format!("{}x{}", self.width, self.height);
        let help_text = "Ctrl+L: Clear | ↑↓: History";
        
        // Create a more readable status line with distinct sections
        let status = format!(
            " 🕒 {} | 📅 {} | 📺 {} | ⌨️  {} ",
            time_str, date_str, terminal_info, help_text
        );

        // Truncate if needed
        let status_display = if status.len() > self.width {
            status[..self.width].to_string()
        } else {
            status
        };

        let mut stdout = stdout();

        // Save cursor position
        queue!(stdout, cursor::SavePosition)?;

        // Move to status bar line
        queue!(
            stdout,
            cursor::MoveTo(0, (self.height - STATUS_BAR_LINE - 1) as u16)
        )?;

        // Set colors and print status with improved visibility
        queue!(
            stdout,
            SetBackgroundColor(Color::DarkBlue),
            SetForegroundColor(Color::White),
            style::SetAttribute(style::Attribute::Bold),
            terminal::Clear(ClearType::CurrentLine),
            style::Print(status_display),
            style::SetAttribute(style::Attribute::Reset),
            SetBackgroundColor(Color::Reset),
            SetForegroundColor(Color::Reset)
        )?;

        // Restore cursor position
        queue!(stdout, cursor::RestorePosition)?;

        stdout.flush()
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
    pub fn restore_terminal() -> std::io::Result<()> {
        // First clear the terminal to remove any leftover UI elements
        let _ = Self::clear_console();

        // Disable raw mode and leave alternate screen
        terminal::disable_raw_mode()?;
        execute!(stdout(), terminal::LeaveAlternateScreen)?;

        // Flush stdout to ensure all terminal commands are processed
        stdout().flush()?;

        Ok(())
    }

    pub fn read_input(&mut self, input: &mut String) -> std::io::Result<(bool, bool)> {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(KeyEvent {
                code, modifiers, ..
            }) = event::read()?
            {
                match code {
                    KeyCode::Enter => {
                        if !input.is_empty()
                            && (self.input_history.is_empty()
                                || self.input_history.last().unwrap() != input)
                        {
                            self.input_history.push(input.clone());
                            if self.input_history.len() > 50 {
                                self.input_history.remove(0);
                            }
                        }
                        self.history_position = self.input_history.len();
                        self.current_input.clear();
                        return Ok((true, false));
                    }
                    KeyCode::Char('q') if modifiers == event::KeyModifiers::CONTROL => {
                        // Ctrl+Q exits the application immediately
                        println!("\nExiting application via Ctrl+Q...");
                        stdout().flush()?;
                        return Ok((false, true));
                    }
                    KeyCode::Char('c') if modifiers == event::KeyModifiers::CONTROL => {
                        // Ctrl+C also exits the application immediately
                        println!("\nExiting application via Ctrl+C...");
                        stdout().flush()?;
                        return Ok((false, true));
                    }
                    KeyCode::Char('l') if modifiers == event::KeyModifiers::CONTROL => {
                        // Ctrl+L clears the screen
                        let _ = Self::clear_console();
                        let _ = self.print_all_messages(true);
                        let _ = self.print_status_bar();
                        let _ = self.print_input_prompt();
                        print!("{}", input);
                        stdout().flush()?;
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
                    KeyCode::Tab => {
                        // Tab completion for commands
                        if input.starts_with('/') {
                            let matching_commands: Vec<&str> = COMMON_COMMANDS
                                .iter()
                                .filter(|&cmd| cmd.starts_with(input.as_str()))
                                .cloned()
                                .collect();

                            match matching_commands.len() {
                                1 => {
                                    // Exact match, complete the command
                                    input.clear();
                                    input.push_str(matching_commands[0]);

                                    // Clear line and print the completed command
                                    execute!(
                                        stdout(),
                                        cursor::MoveTo(
                                            USER_INPUT_PROMPT_LENGTH as u16,
                                            (self.height - 1) as u16
                                        ),
                                        terminal::Clear(ClearType::UntilNewLine),
                                        style::Print(input)
                                    )?;
                                }
                                n if n > 1 => {
                                    // Multiple matches - show options above the input line
                                    let mut stdout = stdout();

                                    // Save cursor position
                                    queue!(stdout, cursor::SavePosition)?;

                                    // Move to the line above input
                                    queue!(stdout, cursor::MoveTo(0, (self.height - 2) as u16))?;

                                    // Print matches
                                    let matches_str = matching_commands.join("  ");
                                    queue!(
                                        stdout,
                                        terminal::Clear(ClearType::CurrentLine),
                                        SetForegroundColor(Color::Yellow),
                                        style::Print(matches_str),
                                        SetForegroundColor(Color::Reset)
                                    )?;

                                    // Restore cursor position
                                    queue!(stdout, cursor::RestorePosition)?;
                                    stdout.flush()?;

                                    // Find common prefix if any
                                    if let Some(common_prefix) =
                                        Self::find_common_prefix(&matching_commands)
                                    {
                                        if common_prefix.len() > input.len() {
                                            input.clear();
                                            input.push_str(&common_prefix);

                                            // Update the input line
                                            execute!(
                                                stdout,
                                                cursor::MoveTo(
                                                    USER_INPUT_PROMPT_LENGTH as u16,
                                                    (self.height - 1) as u16
                                                ),
                                                terminal::Clear(ClearType::UntilNewLine),
                                                style::Print(input)
                                            )?;
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    KeyCode::Up => {
                        if !self.input_history.is_empty() {
                            if self.history_position == self.input_history.len() {
                                self.current_input = input.clone();
                            }

                            if self.history_position > 0 {
                                self.history_position -= 1;
                                input.clear();
                                input.push_str(&self.input_history[self.history_position]);

                                // Clear current line and print new input
                                execute!(
                                    stdout(),
                                    cursor::MoveTo(
                                        USER_INPUT_PROMPT_LENGTH as u16,
                                        (self.height - 1) as u16
                                    ),
                                    terminal::Clear(ClearType::UntilNewLine),
                                    style::Print(input)
                                )?;
                            }
                        }
                    }
                    KeyCode::Down => {
                        if self.history_position < self.input_history.len() {
                            self.history_position += 1;
                            input.clear();

                            if self.history_position == self.input_history.len() {
                                input.push_str(&self.current_input);
                            } else {
                                input.push_str(&self.input_history[self.history_position]);
                            }

                            // Clear current line and print new input
                            execute!(
                                stdout(),
                                cursor::MoveTo(
                                    USER_INPUT_PROMPT_LENGTH as u16,
                                    (self.height - 1) as u16
                                ),
                                terminal::Clear(ClearType::UntilNewLine),
                                style::Print(input)
                            )?;
                        }
                    }
                    KeyCode::Esc => {
                        // Escape key exits the application
                        println!("\nExiting application via Escape key...");
                        stdout().flush()?;
                        return Ok((false, true));
                    }
                    _ => {}
                }
            }
        }
        Ok((false, false))
    }

    // Helper function to find the common prefix among strings
    fn find_common_prefix(strings: &[&str]) -> Option<String> {
        if strings.is_empty() {
            return None;
        }

        if strings.len() == 1 {
            return Some(strings[0].to_string());
        }

        let first = strings[0];
        let mut common_prefix = String::new();

        for (i, c) in first.chars().enumerate() {
            if strings.iter().all(|s| s.chars().nth(i) == Some(c)) {
                common_prefix.push(c);
            } else {
                break;
            }
        }

        if common_prefix.is_empty() {
            None
        } else {
            Some(common_prefix)
        }
    }
}
