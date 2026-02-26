use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::task::JoinHandle;

use crate::output::worker::AudioWorker;
use crate::prs;
use crate::prs::sync::{PhaseReferenceSynchroniser, new_synchroniser};
use crate::source::{SourceControl, start_source};
use crate::{Cli, ControlData, ControlEvent, EventData, UiEvent, pad};
use crate::{
    fic::{FastInformationChannelBuffer, ensemble::new_ensemble},
    msc::new_channel,
};

pub struct ReceiverRuntime {
    pub ui_rx: UnboundedReceiver<UiEvent>,
    pub control_tx: UnboundedSender<ControlEvent>,
    pub task: JoinHandle<()>,
}

pub struct DABReceiver {
    args: Cli,
}

pub fn new_receiver(args: Cli) -> DABReceiver {
    DABReceiver { args }
}

impl DABReceiver {
    pub fn run(self) -> ReceiverRuntime {
        let (ui_tx, ui_rx) = unbounded_channel();
        let (control_tx, control_rx) = unbounded_channel();

        let task = tokio::task::spawn_local(async move {
            run_receiver(self.args, ui_tx, control_rx).await;
        });

        ReceiverRuntime {
            ui_rx,
            control_tx,
            task,
        }
    }
}

async fn run_receiver(
    args: Cli,
    ui_tx: UnboundedSender<UiEvent>,
    mut control_rx: UnboundedReceiver<ControlEvent>,
) {
    let Cli {
        source,
        service: service_id,
        file,
        frequency,
    } = args;

    let mut source_runtime = start_source(source, file, frequency);
    let mut fic_decoder = crate::fic::new_decoder();
    let mut ensemble = new_ensemble();
    let mut synchroniser = new_synchroniser();
    let mut prs_symbol = prs::new_symbol();
    let mut stop_requested = false;

    'fic: loop {
        tokio::select! {
            maybe_control = control_rx.recv() => {
                let Some(control) = maybe_control else {
                    stop_requested = true;
                    break 'fic;
                };

                if let ControlEvent {
                    data: ControlData::Stop(),
                } = control
                {
                    stop_requested = true;
                    break 'fic;
                }
            }
            maybe_buffer = source_runtime.buffers.recv() => {
                let Some(buffer) = maybe_buffer else {
                    break 'fic;
                };

                if buffer.last {
                    break 'fic;
                }

                sync_prs(&buffer, &mut prs_symbol, &mut synchroniser, &source_runtime.control);

                if !synchroniser.is_locked() {
                    continue;
                }

                if let Ok(fic_buffer) = TryInto::<FastInformationChannelBuffer>::try_into(&buffer)
                    && let Some(fibs) = fic_decoder.try_buffer(fic_buffer)
                {
                    for fib in fibs {
                        let figs = fic_decoder.extract_figs(&fib);
                        for fig in figs {
                            ensemble.add_fig(fig);
                        }
                    }
                    if ensemble.is_complete() {
                        break 'fic;
                    }
                }
            }
        }
    }

    if stop_requested {
        source_runtime.control.shutdown().await;
        return;
    }

    let _ = ui_tx.send(UiEvent {
        data: EventData::Ensemble(ensemble.clone()),
    });

    if let Some(service) = ensemble.find_service_by_id_str(&service_id) {
        let mut msc = new_channel(service);
        synchroniser.select_channel(&msc);

        let _ = ui_tx.send(UiEvent {
            data: EventData::Service(service.clone()),
        });

        let mut pad = pad::new_padstate();
        let audio = AudioWorker::new();

        'msc: loop {
            tokio::select! {
                maybe_control = control_rx.recv() => {
                    let Some(control) = maybe_control else {
                        break 'msc;
                    };

                    match control.data {
                        ControlData::Stop() => {
                            break 'msc;
                        }
                        ControlData::Select(service_id) => {
                            if let Some(service) = ensemble.find_service_by_id(service_id) {
                                msc = new_channel(service);
                                synchroniser.select_channel(&msc);
                                audio.reset().await;
                                let _ = ui_tx.send(UiEvent {
                                    data: EventData::Service(service.clone()),
                                });
                            }
                        }
                        ControlData::Tune(_) => {}
                    }
                }
                maybe_buffer = source_runtime.buffers.recv() => {
                    let Some(buffer) = maybe_buffer else {
                        break 'msc;
                    };

                    if buffer.last {
                        break 'msc;
                    }

                    sync_prs(&buffer, &mut prs_symbol, &mut synchroniser, &source_runtime.control);

                    if !synchroniser.is_locked() {
                        continue;
                    }

                    if let Some(main) = msc.try_buffer(&buffer) {
                        if let Ok(dls) = pad.output(&main) {
                            let _ = ui_tx.send(UiEvent {
                                data: EventData::Label(dls.label),
                            });
                        }
                        audio.try_send_frame(main);
                    }
                }
            }
        }

        audio.shutdown().await;
    }

    source_runtime.control.shutdown().await;
}

fn sync_prs(
    buffer: &crate::wavefinder::Buffer,
    prs_symbol: &mut prs::PhaseReferenceSymbol,
    synchroniser: &mut PhaseReferenceSynchroniser,
    source_control: &SourceControl,
) {
    prs_symbol.try_buffer(buffer);
    if prs_symbol.is_complete() {
        let complete_prs = std::mem::replace(prs_symbol, prs::new_symbol());
        for message in synchroniser.try_sync_prs(complete_prs) {
            source_control.send_wavefinder_message(message);
        }
    }
}
