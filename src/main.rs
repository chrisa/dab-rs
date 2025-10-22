#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::too_many_arguments)]

use std::io;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::JoinHandle;
use std::time::Duration;

use color_eyre::Result;
use dab::fic::ensemble::{Ensemble, Service};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, poll};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::symbols::border;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Cell, Paragraph, Row, Table, TableState};
use ratatui::{DefaultTerminal, Frame};

use clap::Parser;
use dab::receiver::new_receiver;
use dab::{Cli, ControlData, ControlEvent, EventData, UiEvent};

struct App {
    exit: bool,
    control_tx: Sender<ControlEvent>,
    ui_rx: Receiver<UiEvent>,
    ensemble: Option<Ensemble>,
    service: Option<Service>,
    tablestate: TableState,
}

fn main() -> Result<()> {
    let args = Cli::parse();
    color_eyre::install()?;
    let terminal = ratatui::init();

    let mut receiver = new_receiver(args);
    let (ui_rx, control_tx, receiver_t) = receiver.run();

    let mut app = App {
        ui_rx,
        control_tx,
        ensemble: None,
        service: None,
        exit: false,
        tablestate: TableState::default().with_selected(0),
    };
    let result = app.run(terminal, receiver_t);

    ratatui::restore();
    result
}

impl App {
    fn run(&mut self, mut terminal: DefaultTerminal, receiver_t: JoinHandle<()>) -> Result<()> {
        loop {
            terminal.draw(|frame| self.draw(frame))?;

            if let Ok(events) = poll(Duration::from_millis(100))
                && events
            {
                self.handle_events()?;
            }

            if let Ok(event) = self.ui_rx.recv_timeout(Duration::from_millis(100)) {
                match event {
                    UiEvent {
                        data: EventData::Ensemble(ensemble),
                    } => {
                        self.ensemble = Some(ensemble);
                    }
                    _ => todo!("unexpected event"),
                }
            }

            if self.exit {
                break;
            }
        }

        // todo propagate properly
        let result = receiver_t.join();
        Ok(())
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
            self.exit = true;
            if self
                .control_tx
                .send(ControlEvent {
                    data: ControlData::Stop(),
                })
                .is_err()
            {
                eprintln!("failed to send q");
            }
        }
    }

    fn draw(&mut self, frame: &mut Frame) {
        let top_title = Line::from(" Wavefinder Receiver ");
        let top_block = Block::bordered()
            .title(top_title.centered())
            .border_set(border::THICK);

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Percentage(20), Constraint::Percentage(80)])
            .split(frame.area());

        if self.ensemble.is_some() {
            let status_text = Line::from("Ensemble Found");

            frame.render_widget(
                Paragraph::new(status_text).centered().block(top_block),
                layout[0],
            );

            self.render_table(frame, layout[1]);
        } else {
            let status_text = Line::from("Starting Up");

            frame.render_widget(
                Paragraph::new(status_text).centered().block(top_block),
                layout[0],
            );
        }
    }

    fn render_table(&mut self, frame: &mut Frame, area: Rect) {
        let ensemble = self.ensemble.as_ref().unwrap();

        let header = ["Ensemble", "Label", "Id", "Bitrate", "Type"]
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .height(1);

        let rows = ensemble
            .services()
            .into_iter()
            .enumerate()
            .map(|(i, service)| {
                [
                    Cell::from(Text::from(ensemble.label())),
                    Cell::from(Text::from(service.label())),
                    Cell::from(Text::from(format!("{:04x}", service.id))),
                    Cell::from(Text::from(format!(
                        "{}kbps",
                        service.subchannel().bitrate()
                    ))),
                    Cell::from(Text::from(format!(
                        "{:?}",
                        service.subchannel().subchannel_type()
                    ))),
                ]
                .into_iter()
                .collect::<Row>()
                .height(1)
            });

        let bottom_title = Line::from(" Ensemble Details ");
        let bottom_block = Block::bordered()
            .title(bottom_title.centered())
            .border_set(border::THICK);

        let table = Table::new(
            rows,
            [
                Constraint::Length(18),
                Constraint::Length(18),
                Constraint::Length(4),
                Constraint::Length(7),
                Constraint::Length(20),
            ],
        )
        .header(header)
        .block(bottom_block);
        frame.render_stateful_widget(table, area, &mut self.tablestate);
    }
}
