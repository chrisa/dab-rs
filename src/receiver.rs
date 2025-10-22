use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::thread::JoinHandle;

use crate::output::mpeg::{self};
use crate::{Cli, CliSource, ControlEvent, UiEvent};
use crate::{ControlData, EventData, pad};
use crate::{
    fic::{FastInformationChannelBuffer, ensemble::new_ensemble},
    msc::new_channel,
};

pub struct DABReceiver {
    args: Cli,
}

pub fn new_receiver(args: Cli) -> DABReceiver {
    DABReceiver { args }
}

impl DABReceiver {
    pub fn run(&mut self) -> (Receiver<UiEvent>, Sender<ControlEvent>, JoinHandle<()>) {
        let mut source = match self.args.source {
            CliSource::Wavefinder => crate::source::wavefinder::new_wavefinder_source(
                self.args.file.clone(),
                self.args.frequency.clone(),
            ),
            CliSource::File => crate::source::file::new_file_source(self.args.file.clone()),
        };

        let (source_rx, source_t) = source.run();

        let (ui_tx, ui_rx) = mpsc::channel();
        let (control_tx, control_rx) = mpsc::channel();

        let mut fic_decoder = crate::fic::new_decoder();
        let mut ens = new_ensemble();
        let service_id = self.args.service.clone();

        let receiver_t = thread::spawn(move || {
            // FIC

            while let Ok(buffer) = source_rx.recv() {
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

            ui_tx
                .send(UiEvent {
                    data: EventData::Ensemble(ens.clone()),
                })
                .expect("sending ensemble to app");

            // If service, MSC
            if let Some(service) = ens.find_service_by_id_str(&service_id) {
                let mut msc = new_channel(service);
                source.as_mut().select_channel(&msc);

                ui_tx
                    .send(UiEvent {
                        data: EventData::Service(service.clone()),
                    })
                    .expect("sending service to app");

                let pad = pad::new_padstate();
                let mut mpeg = mpeg::new_mpeg();
                mpeg.init();

                'msc: while let Ok(buffer) = source_rx.recv() {
                    if buffer.last {
                        break;
                    }

                    if let Ok(msg) = control_rx.try_recv() {
                        match msg {
                            ControlEvent {
                                data: ControlData::Stop(),
                            } => {
                                source.exit();
                                break 'msc;
                            },
                            ControlEvent {
                                data: ControlData::Select(service_id),
                            } => {
                                if let Some(service) = ens.find_service_by_id(service_id) {
                                    msc = new_channel(service);
                                    source.as_mut().select_channel(&msc);
                                }
                            }
                            _ => todo!(),
                        }
                    }

                    if !source.as_ref().ready() {
                        continue;
                    }

                    if let Some(main) = msc.try_buffer(&buffer) {
                        // if let Ok(label) = pad.output(&main)
                        //     && label.is_new {
                        //         eprintln!("DLS: {}", label.label);
                        //     }
                        mpeg.output(&main);
                    }
                }

                source_t.join().unwrap();
            }
        });

        (ui_rx, control_tx, receiver_t)
    }
}
