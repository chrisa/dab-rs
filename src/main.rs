#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::too_many_arguments)]

use std::sync::Arc;
use std::time::Duration;

use color_eyre::Result;
use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind};
use dab::fic::ensemble::{Ensemble, Service};
use futures_util::StreamExt;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::symbols::border;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Cell, Paragraph, Row, Table, TableState};
use ratatui::{DefaultTerminal, Frame};
use tokio::sync::{mpsc::UnboundedSender, watch};
use tokio::task::JoinHandle;
use dial9_tokio_telemetry::telemetry::{RotatingWriter, TracedRuntime};

use clap::Parser;
use dab::receiver::new_receiver;
use dab::{Cli, ControlData, ControlEvent, UiModel};

struct App {
    exit: bool,
    control_tx: UnboundedSender<ControlEvent>,
    ui_rx: watch::Receiver<UiModel>,
    ensemble: Option<Arc<Ensemble>>,
    service: Option<Arc<Service>>,
    label: Option<String>,
    tablestate: TableState,
}

fn main() -> Result<()> {
    let args = Cli::parse();
    color_eyre::install()?;
    let terminal = ratatui::init();

    let writer = RotatingWriter::new(
        "/tmp/my_traces/trace.bin",
        20 * 1024 * 1024,   // rotate after 20 MiB
        100 * 1024 * 1024,  // keep at most 100 MiB on disk
    )?;

    let mut builder = tokio::runtime::Builder::new_current_thread();
    builder.enable_all();
    // builder.worker_threads(4).enable_all();

    let (runtime, _guard) = TracedRuntime::build_and_start(builder, writer)?;

    let _ = runtime.block_on(async {

        let _ = tokio::task::LocalSet::new().run_until(async move {

            let receiver = new_receiver(args);
            let runtime = receiver.run();

            let mut app = App {
                ui_rx: runtime.ui_rx,
                control_tx: runtime.control_tx,
                ensemble: None,
                service: None,
                label: None,
                exit: false,
                tablestate: TableState::default().with_selected(0),
            };

            app.run(terminal, runtime.task).await
        })
        .await;
    });

    ratatui::restore();
    Ok(())
}

impl App {
    async fn run(
        &mut self,
        mut terminal: DefaultTerminal,
        receiver_t: JoinHandle<()>,
    ) -> Result<()> {
        let mut events = EventStream::new();
        let mut tick = tokio::time::interval(Duration::from_millis(100));

        loop {
            terminal.draw(|frame| self.draw(frame))?;

            tokio::select! {
                maybe_event = events.next() => {
                    if let Some(Ok(Event::Key(key_event))) = maybe_event
                        && key_event.kind == KeyEventKind::Press
                    {
                        self.handle_key_event(key_event);
                    }
                }
                changed = self.ui_rx.changed() => {
                    if changed.is_ok() {
                        self.handle_ui_model();
                    }
                }
                _ = tick.tick() => {}
            }

            if self.exit {
                break;
            }
        }

        let _ = receiver_t.await;
        Ok(())
    }

    fn set_selected_service(&mut self) {
        for (i, s) in self
            .ensemble
            .as_ref()
            .unwrap()
            .services()
            .into_iter()
            .enumerate()
        {
            if self.service.as_ref().unwrap().id == s.id {
                self.tablestate.select(Some(i));
            }
        }
    }

    fn handle_ui_model(&mut self) {
        let previous_service = self.service.as_ref().map(|service| service.id);
        let model = self.ui_rx.borrow_and_update().clone();
        self.ensemble = model.ensemble;
        self.service = model.service;
        self.label = model.label;

        let current_service = self.service.as_ref().map(|service| service.id);
        if previous_service != current_service && current_service.is_some() {
            self.set_selected_service();
        }
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') | KeyCode::Esc => self.quit(),
            KeyCode::Char('j') | KeyCode::Down => self.next_row(),
            KeyCode::Char('k') | KeyCode::Up => self.previous_row(),
            KeyCode::Enter => self.select_service(),
            _ => (),
        }
    }

    fn select_service(&mut self) {
        if self.ensemble.is_none() {
            return;
        }
        if let Some(i) = self.tablestate.selected() {
            let service = self.ensemble.as_ref().unwrap().services()[i];
            if self
                .control_tx
                .send(ControlEvent {
                    data: ControlData::Select(service.id),
                })
                .is_err()
            {
                eprintln!("failed to send Enter");
            }
        }
    }

    fn next_row(&mut self) {
        if self.ensemble.is_none() {
            return;
        }
        let i = match self.tablestate.selected() {
            Some(i) => {
                if i >= self.ensemble.as_ref().unwrap().services().len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.tablestate.select(Some(i));
    }

    fn previous_row(&mut self) {
        if self.ensemble.is_none() {
            return;
        }
        let i = match self.tablestate.selected() {
            Some(i) => {
                if i == 0 {
                    self.ensemble.as_ref().unwrap().services().len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.tablestate.select(Some(i));
    }

    fn quit(&mut self) {
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

    fn draw(&mut self, frame: &mut Frame) {
        let top_title = Line::from(" Wavefinder Receiver ");
        let top_block = Block::bordered()
            .title(top_title.centered())
            .border_set(border::THICK);

        let dls_title = Line::from(" DLS ");
        let dls_block = Block::bordered()
            .title(dls_title.centered())
            .border_set(border::THICK);

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Percentage(20),
                Constraint::Percentage(10),
                Constraint::Percentage(70),
            ])
            .split(frame.area());

        if self.ensemble.is_some() {
            let status_text = Line::from("Ensemble Found");

            frame.render_widget(
                Paragraph::new(status_text).centered().block(top_block),
                layout[0],
            );

            self.render_table(frame, layout[2]);
        } else {
            let status_text = Line::from("Starting Up");

            frame.render_widget(
                Paragraph::new(status_text).centered().block(top_block),
                layout[0],
            );
        }

        if let Some(label) = &self.label {
            let paragraph =
                Paragraph::new(Line::from(label.to_string())).alignment(Alignment::Left);

            frame.render_widget(paragraph.block(dls_block), layout[1])
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

        let selected_row_style = Style::default().add_modifier(Modifier::REVERSED);

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
        .block(bottom_block)
        .row_highlight_style(selected_row_style);

        frame.render_stateful_widget(table, area, &mut self.tablestate);
    }
}
