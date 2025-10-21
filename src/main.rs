#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::too_many_arguments)]

use std::io;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

use color_eyre::Result;
use dab::fic::ensemble::{Ensemble, Service};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, poll};
use ratatui::layout::Rect;
use ratatui::symbols::border;
use ratatui::text::Line;
use ratatui::widgets::{Block, Paragraph, Widget};
use ratatui::{DefaultTerminal, Frame};

use dab::receiver::new_receiver;
use dab::{Cli, ControlData, ControlEvent, UiEvent};
use clap::Parser;

struct App {
    control_tx: Sender<ControlEvent>,
    ui_rx: Receiver<UiEvent>,
    ensemble: Option<Ensemble>,
    service: Option<Service>,
}

fn main() -> Result<()> {
    let args = Cli::parse();
    color_eyre::install()?;
    let terminal = ratatui::init();

    let mut receiver = new_receiver(args);
    let (ui_rx, control_tx, receiver_t) = receiver.run();

    let mut app = App { ui_rx, control_tx, ensemble: None, service: None };
    let result = app.run(terminal);

    ratatui::restore();
    result
}


impl App {

    fn run(&mut self, mut terminal: DefaultTerminal) -> Result<()> {
        loop {
            terminal.draw(|frame| self.draw(frame))?;

            if let Ok(events) = poll(Duration::from_millis(100))
                && events
            {
                self.handle_events()?;
            }
        }
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            // it's important to check that the event is a key press event as
            // crossterm also emits key release and repeat events on Windows.
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        if let KeyCode::Char('q') = key_event.code {
            if let Err(_) = self.control_tx.send(ControlEvent { data: ControlData::Stop() }) {
                eprintln!("failed to send q");
            }
        }
    }

}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let title = Line::from(" Wavefinder Receiver ");
        let block = Block::bordered()
            .title(title.centered())
            .border_set(border::THICK);
        let status_text = Line::from("stuff");
        Paragraph::new(status_text)
            .centered()
            .block(block)
            .render(area, buf);
    }
}
