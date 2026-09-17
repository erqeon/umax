/* There are errors here right now, but there’s always time to fix everything, isn’t there? */

/* oh well, it's just importing libraries */
use std::{io::{self, Write}};

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{Event, KeyCode, KeyModifiers, read}, 
    execute, 
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, 
        disable_raw_mode, enable_raw_mode, size
    },
};

/* We don't know what this is, if we knew... */
struct Editor {
    buffer: Vec<Vec<char>>,
    cursor_x: usize,
    cursor_y: usize,
    filename: String,
    ctrl_x_pressed: bool,
    saved_buffer: Vec<Vec<char>>,
    row_offset: usize,
    redraw_all: bool,
    redraw_current_line: bool,
    col_offset: usize,
}

impl Editor {
    /* This function is responsible for rendering on the screen. */
    pub fn render(&mut self, stdout: &mut io::Stdout) -> io::Result<()> {
        execute!(stdout, Hide)?;
        execute!(stdout, MoveTo(0, 0))?;

        let (width, height) = size().unwrap();
        let visible_height = (height as usize) - 1;

        if self.cursor_y >= self.row_offset + visible_height {
            self.row_offset = self.cursor_y - visible_height + 1;
            self.redraw_all = true;
        }

        if self.cursor_y < self.row_offset {
            self.row_offset = self.cursor_y;
            self.redraw_all = true;
        }

        let end_row = std::cmp::min(self.buffer.len(), self.row_offset + visible_height);
        let visible_rows = &self.buffer[self.row_offset .. end_row];

        if self.redraw_all {
            for i in 0..visible_height {
                execute!(stdout, MoveTo(0, i as u16))?;
                execute!(stdout, crossterm::terminal::Clear(crossterm::terminal::ClearType::UntilNewLine))?;

                if i < visible_rows.len() {
                    let row = &visible_rows[i];
                    let line: String = if self.col_offset < row.len() {
                        let end_col = std::cmp::min(row.len(), self.col_offset + width as usize);
                        row[self.col_offset..end_col].iter().collect()
                    } else {
                        String::new()
                    };

                    print!("{}", line);
                } else {
                    print!("");
                }
            }
            self.redraw_all = false;
        }

        if self.redraw_current_line {
            let screen_y = self.cursor_y - self.row_offset;
            
            if screen_y < visible_height && self.cursor_y < self.buffer.len() {
                let row = &self.buffer[self.cursor_y];
                let line: String = if self.col_offset < row.len() {
                    let end_col = std::cmp::min(row.len(), self.col_offset + width as usize);
                    row[self.col_offset..end_col].iter().collect()
                } else {
                    String::new()
                };

                execute!(stdout, MoveTo(0, screen_y as u16))?;
                execute!(stdout, crossterm::terminal::Clear(crossterm::terminal::ClearType::UntilNewLine))?;
                print!("{}", line);
            }

            self.redraw_current_line = false;
        }

        std::io::Write::flush(&mut std::io::stdout())?;

        execute!(stdout, MoveTo(0, visible_height as u16))?;

        execute!(
            stdout,
            crossterm::style::SetBackgroundColor(crossterm::style::Color::AnsiValue(252)),
            crossterm::style::SetForegroundColor(crossterm::style::Color::Black), 
        )?;

        execute!(stdout, crossterm::terminal::Clear(crossterm::terminal::ClearType::UntilNewLine))?;
        
        print!(
            "{}{} L{}, C{}. {}", 
            self.filename, 
            if self.buffer == self.saved_buffer { "" } else { "*" }, 
            self.cursor_y + 1, self.cursor_x + 1, 
            if self.ctrl_x_pressed { "C-x" } else { "" },
        );

        execute!(
            stdout,
            crossterm::style::ResetColor,
        )?;
        
        let screen_x = self.cursor_x - self.col_offset;
        let screen_y = self.cursor_y - self.row_offset;
        execute!(stdout, MoveTo(screen_x as u16, screen_y as u16))?;
        
        execute!(stdout, Show)?;
        stdout.flush()?;
        Ok(())
    }

    /* This function handles control/input. */
    pub fn handle_key(&mut self, key_event: crossterm::event::KeyEvent) -> bool {
        if key_event.kind != crossterm::event::KeyEventKind::Press {
            return false;
        }

        if self.ctrl_x_pressed == false && key_event.code == KeyCode::Char('x') && key_event.modifiers == KeyModifiers::CONTROL {
            self.ctrl_x_pressed = true;
            return false;
        }

        if self.ctrl_x_pressed == true {
            if key_event.code == KeyCode::Char('s') && key_event.modifiers == KeyModifiers::CONTROL {
                let mut content = String::new();

                for line in &self.buffer {
                    let line_str: String = line.iter().collect();
                    content.push_str(&line_str);
                    content.push_str("\n");
                }

                self.ctrl_x_pressed = false;
                let _ = std::fs::write(&self.filename, content);
                self.saved_buffer = self.buffer.clone();
                self.redraw_all = true;
                return false;
            } else if key_event.code == KeyCode::Char('c') && key_event.modifiers == KeyModifiers::CONTROL {
                self.redraw_all = true;
                return true;
            } else {
                self.ctrl_x_pressed = false;
                self.redraw_all = true;
            }

            return false;
        }

        if key_event.code == KeyCode::Backspace {
            if self.cursor_x > 0 {
                if self.cursor_x >= 4 {
                    let row = &self.buffer[self.cursor_y];
                    let left_chars = &row[self.cursor_x - 4 .. self.cursor_x];

                    if left_chars.iter().all(|&c| c == ' ') {
                        self.cursor_x -= 4;
                        for _ in 0..4 {
                            self.buffer[self.cursor_y].remove(self.cursor_x);
                        }

                        self.redraw_current_line = true;
                        return false;
                    }
                }

                self.cursor_x -= 1;
                self.buffer[self.cursor_y].remove(self.cursor_x);
                self.redraw_current_line = true;
            } else {
                if self.cursor_y > 0 {
                    let current_line = self.buffer.remove(self.cursor_y);
                    self.cursor_y -= 1;
                    let prev_line_len = self.buffer[self.cursor_y].len();
                    self.cursor_x = prev_line_len;
                    self.buffer[self.cursor_y].extend(current_line);
                    
                    self.redraw_all = true;
                }
            }
        }

        if key_event.code == KeyCode::Tab {
            /* If you like, you can customize it to suit your needs. */
            for _ in 0..4 {
                self.buffer[self.cursor_y].insert(self.cursor_x, ' ');
                self.cursor_x += 1;
            }

            self.redraw_current_line = true;
            return false;
        }

        if key_event.code == KeyCode::Enter {
            let mut spaces_count = 0;

            for &c in &self.buffer[self.cursor_y] {
                if c == ' ' {
                    spaces_count += 1;
                } else {
                    break;
                }
            }

            let new_line = self.buffer[self.cursor_y].split_off(self.cursor_x);
            self.buffer.insert(self.cursor_y + 1, new_line);
            self.cursor_y += 1;

            for _ in 0..spaces_count {
                self.buffer[self.cursor_y].insert(0, ' ');
            }

            self.cursor_x = spaces_count;
            self.redraw_all = true;
        }

        if key_event.code == KeyCode::Up {
            if self.cursor_y > 0 {
                self.cursor_y -= 1;
                
                let len = self.buffer[self.cursor_y].len();
                
                if self.cursor_x > len {
                    self.cursor_x = len;
                }

                if self.cursor_y < self.row_offset {
                    self.row_offset = self.cursor_y;
                    self.redraw_all = true;
                }
                 
                if self.cursor_x < self.col_offset {
                    self.col_offset = self.cursor_x;
                    self.redraw_all = true;
                }
            }
        }

        if key_event.code == KeyCode::Down {
            if self.cursor_y < self.buffer.len() - 1 {
                self.cursor_y += 1;
                
                let len = self.buffer[self.cursor_y].len();
                
                if self.cursor_x > len {
                    self.cursor_x = len;
                }
                
                let (_, height) = size().unwrap();
                let mut visible_height = 1;

                if height > 1 {
                    visible_height = height as usize - 1;
                }

                if self.cursor_y >= self.row_offset + visible_height {
                    self.row_offset = self.cursor_y - visible_height + 1;
                    self.redraw_all = true;
                }

                if self.cursor_x < self.col_offset {
                    self.col_offset = self.cursor_x;
                    self.redraw_all = true;
                }
            }
        }

        if key_event.code == KeyCode::Left {
            if self.cursor_x > 0 {
                self.cursor_x -= 1;
                if self.cursor_x < self.col_offset {
                    self.col_offset = self.cursor_x;
                    self.redraw_all = true;
                }
            }
        }

        if key_event.code == KeyCode::Right {
            let line = self.buffer[self.cursor_y].len();

            if self.cursor_x < line {
                self.cursor_x += 1;

                let (width, _) = size().unwrap();

                if self.cursor_x >= self.col_offset + width as usize {
                    self.col_offset = self.cursor_x - width as usize + 1;
                    self.redraw_all = true;
                }
            }
        }

        if let KeyCode::Char(c) = key_event.code {
            self.buffer[self.cursor_y].insert(self.cursor_x, c);
            self.cursor_x += 1;
            self.redraw_current_line = true;
        }

        return false;
    }
}

/* 
    Well, that’s the main function.
    (incidentally, this is exactly where people start learning Rust and write "Hello, world!").
*/
fn main() -> io::Result<()> {
    let mut stdout = io::stdout();
    let args: Vec<String> = std::env::args().collect();
    
    let filename = if args.len() > 1 {
        args[1].clone()
    } else {
        String::from("untitled.txt")
    };

    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen)?;

    execute!(stdout, crossterm::terminal::Clear(crossterm::terminal::ClearType::All))?;
    stdout.flush()?;

    let buffer = std::fs::read_to_string(&filename);

    let file_buffer = match buffer {
        Ok(text) => {
            let mut lines: Vec<Vec<char>> = text.lines().map(|line| line.chars().collect()).collect();
            if lines.is_empty() {
                lines.push(Vec::new());
            }
            
            lines
        }
        Err(_) => {
            vec![Vec::new()]
        }
    };

    let mut ed1 = Editor {
        buffer: file_buffer.clone(),
        cursor_x: 0,
        cursor_y: 0,
        filename: filename,
        ctrl_x_pressed: false,
        saved_buffer: file_buffer,
        row_offset: 0,
        redraw_all: true,
        redraw_current_line: false,
        col_offset: 0,
    };

    loop {
        ed1.render(&mut stdout)?;
        
        if let Event::Key(key_event) = read()? {
            if ed1.handle_key(key_event) {
                break;
            } 
        }
    }

    execute!(stdout, Show)?;
    execute!(stdout, LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}
