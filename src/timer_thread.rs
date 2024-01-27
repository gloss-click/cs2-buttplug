use std::{collections::VecDeque, time::{SystemTime, Duration}};

use anyhow::Error;
use fehler::throws;
use tokio::{sync::broadcast, task::JoinHandle, runtime::Handle};
use crate::buttplug::BPCommand;


#[derive(Copy, Clone, Debug)]
pub enum ScriptCommand {
    VibrateFor(f64, f64),
    Stop,
    Close,
}

pub struct TimeStampEvent {
    command: BPCommand,
    timestamp: SystemTime
}

impl TimeStampEvent {
    pub fn new(command: BPCommand, duration: Duration) -> Self {
        TimeStampEvent{command, timestamp: SystemTime::now() + duration}
    }
}

#[throws]
pub fn spawn_timer_thread(h: Handle, send: broadcast::Sender<BPCommand>) -> (broadcast::Sender<ScriptCommand>, JoinHandle<()>) {
    let (nsend, nrecv) = broadcast::channel(64);

    let handle = tokio::task::spawn_blocking(move || { 
        h.block_on(timer_thread(send, nrecv)).expect("Timer thread dispatch failed."); 
    });
    
    (nsend, handle)
}

#[throws]
async fn timer_thread(send: broadcast::Sender<BPCommand>, mut recv: broadcast::Receiver<ScriptCommand>) {
    let mut queue: VecDeque<TimeStampEvent> = VecDeque::new();
    info!("started event process thread");
    let mut close = false;
    loop {

        let mut enqueue_func = |msg, queue: &mut VecDeque<TimeStampEvent>| {
            match msg {
                ScriptCommand::VibrateFor(strength, time) => {
                    queue.push_back(TimeStampEvent::new(BPCommand::Vibrate(strength), Duration::ZERO));
                    queue.push_back(TimeStampEvent::new(BPCommand::Stop, Duration::from_secs_f64(time)));
                },
                ScriptCommand::Stop => {
                    queue.push_back(TimeStampEvent::new(BPCommand::Stop, Duration::ZERO));
                },
                ScriptCommand::Close => {
                    close = true;
                },
            }
        };

        if !queue.is_empty() { 
            if let Ok(msg) = recv.try_recv() {
                enqueue_func(msg, &mut queue);
            };
        } else {
            if let Ok(msg) = recv.recv().await {
                enqueue_func(msg, &mut queue);
            }
        }

        if let Some(front) = queue.front() {
            if front.timestamp < SystemTime::now() {
                info!("submitting command");
                
                match send.send(front.command) {
                    Ok(_) => {},
                    Err(e) => {
                        info!("raw command send error {}", e);
                        break;
                    },
                }
                
                let _ = queue.pop_front();
            }
        }
        
        if close {
            break;
        }

    };
    info!("ending event process thread");
    ()
}