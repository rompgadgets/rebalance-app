#[allow(dead_code)]
mod util;

use crate::util::{
    app::App,
    event::{Event, Events},
    ui,
};
use regex::Regex;
use std::{
    error::Error,
    io::{self},
};
use termion::{event::Key, input::MouseTerminal, raw::IntoRawMode, screen::AlternateScreen};
use tui::{backend::TermionBackend, Terminal};
use util::app::InputMode;

fn main() -> Result<(), Box<dyn Error>> {
    // Terminal initialization
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut events = Events::new();

    let mut app = App::new();
    //not used right now but will eventually be used to edit portfolio target allocations
    let mut toggle: bool = false;

    // Validate that input matches a dollar amount e.g. 1000.00
    let input_validation = Regex::new(r"^\d+[.]?\d{2}?$").unwrap();

    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;

        //Event loop to handle the three modes (Input, Edit, and Exec)
        match events.next()? {
            Event::Input(input) => match app.input_mode {
                util::app::InputMode::Normal => match input {
                    Key::Char('e') => {
                        app.input_mode = util::app::InputMode::Editing;
                        events.disable_exit_key();
                    }
                    Key::Char('r') => {
                        app.input_mode = util::app::InputMode::Exec;
                        events.disable_exit_key();
                    }
                    Key::Char('q') => {
                        break;
                    }
                    Key::Left => {
                        app.items.unselect();
                    }
                    Key::Down => {
                        if toggle {
                            app.items.next();
                        } else {
                            app.table_portfolio.next();
                        }
                    }
                    Key::Up => {
                        if toggle {
                            app.items.previous();
                        } else {
                            app.table_portfolio.previous();
                        }
                    }
                    Key::Char(c) => {
                        //let event_str = format!("{} pressed ({:x})", c, c as u8);
                        //app.add_custom_event(event_str);
                        // picks up the keycode for tab
                        if c as u8 == 9 {
                            toggle = !toggle;
                        }
                    }
                    Key::Backspace => {
                        toggle = !toggle;
                        app.add_custom_event("Backspace PRESSED".to_string());
                    }
                    _ => {}
                },
                util::app::InputMode::Editing => match input {
                    Key::Char('\n') => {
                        //pull the new portfolio amount
                        let new_value: String = app.input.drain(..).collect();

                        //validate the input to be a dollar amount
                        if !input_validation.is_match(&new_value) {
                            app.error_msg =
                                "Input must be in the format of a dollar amount".to_string();
                            app.input_mode = util::app::InputMode::ErrorDisplay;
                        } else {
                            if let Some(index) = app.table_portfolio.state.selected() {
                                let row = &mut app.table_portfolio.items[index];
                                row[1] = format!("${}", new_value.clone());
                                //update the underlying asset
                                app.update_asset(index, new_value);
                            }
                            app.input_mode = InputMode::Normal;
                            events.enable_exit_key();
                        }
                    }
                    Key::Char(c) => {
                        app.input.push(c);
                    }
                    Key::Backspace => {
                        app.input.pop();
                    }
                    Key::Esc => {
                        app.input_mode = InputMode::Normal;
                        events.enable_exit_key();
                    }
                    _ => {}
                },
                util::app::InputMode::Exec => match input {
                    Key::Char('\n') => {
                        //get the rebalance amount
                        let new_investment: String = app.input.drain(..).collect();

                        //validate the input to be a dollar amount
                        if !input_validation.is_match(&new_investment) {
                            app.error_msg =
                                "Input must be in the format of a dollar amount".to_string();
                            app.input_mode = util::app::InputMode::ErrorDisplay;
                        } else {
                            //used for logging in a debug UI widget
                            app.add_custom_event(
                                format!("Execute Rebalance with {}", new_investment).to_string(),
                            );
                            app.contribution_amount = new_investment.parse().unwrap();
                            app.lazy_rebalance();
                            //snapshot our portfolio to a csv file
                            app.save_portfolio().expect("Error saving portfolio");
                            //go back to normal mode after doing the rebalance
                            app.input_mode = InputMode::Normal;
                            events.enable_exit_key();
                        }
                    }
                    Key::Char(c) => {
                        if c.is_numeric() || c == '.' {
                            app.input.push(c);
                        }
                    }
                    Key::Backspace => {
                        app.input.pop();
                    }
                    Key::Esc => {
                        app.input_mode = InputMode::Normal;
                        events.enable_exit_key();
                    }
                    _ => {}
                },
                util::app::InputMode::ErrorDisplay => match input {
                    Key::Esc => {
                        app.input_mode = InputMode::Normal;
                        events.enable_exit_key();
                    }
                    _ => {}
                },
            },
            Event::Tick => {}
        }
    }

    Ok(())
}
