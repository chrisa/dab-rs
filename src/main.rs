#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::too_many_arguments)]

use std::io;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use clap::Parser;
use color_eyre::Result;
use dab::output::mpeg::{self};
use dab::pad;
use dab::source::Source;
use dab::wavefinder::Buffer;
use dab::{
    fic::{
        FastInformationChannelBuffer,
        ensemble::{Ensemble, new_ensemble},
    },
    msc::{MainServiceChannel, new_channel},
};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, poll};
use ratatui::layout::Rect;
use ratatui::symbols::border;
use ratatui::text::Line;
use ratatui::widgets::{Block, Paragraph, Widget};
use ratatui::{DefaultTerminal, Frame};

#[derive(Debug, Clone, clap::ValueEnum, PartialEq)]
enum CliSource {
    Wavefinder,
    File,
}

#[derive(Parser, Debug)]
#[command(about, version)]
struct Cli {
    #[clap(value_enum, default_value_t=CliSource::Wavefinder)]
    source: CliSource,
    #[arg(short, long)]
    service: String,
    #[arg(short, long)]
    file: Option<std::path::PathBuf>,
    #[arg(long)]
    frequency: Option<String>,
}

struct DABReceiver {
    exit: bool,
    rx: Receiver<Buffer>,
    source: Box<dyn Source>,
    service_id: String,
}

fn main() -> Result<()> {
    let args = Cli::parse();
    color_eyre::install()?;
    let terminal = ratatui::init();
    let mut receiver = new_receiver(args);
    let result = receiver.run(terminal);
    ratatui::restore();
    result
}

fn new_receiver(args: Cli) -> DABReceiver {
    let (tx, rx) = mpsc::channel();

    match args.source {
        CliSource::Wavefinder => DABReceiver {
            exit: false,
            source: Box::new(dab::source::wavefinder::new_wavefinder_source(
                tx,
                args.file,
                args.frequency,
            )),
            rx,
            service_id: args.service,
        },
        CliSource::File => DABReceiver {
            exit: false,
            source: Box::new(dab::source::file::new_file_source(tx, args.file)),
            rx,
            service_id: args.service,
        },
    }
}

impl DABReceiver {
    fn run(&mut self, terminal: DefaultTerminal) -> Result<()> {
        let t = self.source.run();

        // FIC
        let ens = self.fic();
        ens.display();

        // If service, MSC
        if let Some(service) = ens.find_service_by_id(&self.service_id) {
            let mut msc = new_channel(service);
            self.source.as_mut().select_channel(&msc);
            let result = self.msc(&mut msc, terminal);
            t.join().unwrap();
            result
        } else {
            // eprintln!("Service '{}' not found in ensemble", &self.service_id);
            Ok(())
        }
    }

    fn fic(&self) -> Ensemble {
        let mut fic_decoder = dab::fic::new_decoder();
        let mut ens = new_ensemble();
        let service_name = self.service_id.to_owned();

        while let Ok(buffer) = self.rx.recv() {
            if buffer.last {
                break;
            }
            if let Ok(fic_buffer) = TryInto::<FastInformationChannelBuffer>::try_into(&buffer)
                && let Some(fibs) = fic_decoder.try_buffer(fic_buffer)
            {
                for fib in fibs {
                    let figs = fic_decoder.extract_figs(&fib);
                    for fig in figs {
                        ens.add_fig(fig);
                    }
                }
                if ens.is_complete() {
                    break;
                }
            }
        }
        ens
    }

    fn msc(
        &mut self,
        channel: &mut MainServiceChannel,
        mut terminal: DefaultTerminal,
    ) -> Result<()> {
        let pad = pad::new_padstate();
        let mut mpeg = mpeg::new_mpeg();
        mpeg.init();

        while let Ok(buffer) = self.rx.recv() {
            if buffer.last {
                break;
            }
            if self.exit {
                self.source.exit();
                break;
            }

            if !self.source.as_ref().ready() {
                continue;
            }

            if let Some(main) = channel.try_buffer(&buffer) {
               // if let Ok(label) = pad.output(&main)
                //     && label.is_new {
                //         eprintln!("DLS: {}", label.label);
                //     }
                mpeg.output(&main);
            }

            terminal.draw(|frame| self.draw(frame))?;
            if let Ok(events) = poll(Duration::from_secs(0))
                && events
            {
                self.handle_events()?;
            }
        }

        Ok(())
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
            self.exit()
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}

impl Widget for &DABReceiver {
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
