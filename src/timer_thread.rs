use std::{collections::{BTreeMap, VecDeque}, time::{SystemTime, Duration}};

use anyhow::Error;
use fehler::throws;
use tokio::{sync::broadcast, task::JoinHandle, runtime::Handle};
use crate::buttplug::BPCommand;


#[derive(Copy, Clone, Debug)]
pub enum ScriptCommand {
    VibrateFor(f64, f64),
    VibrateForWithIndex(f64, f64, u32),
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
    //let mut queue: VecDeque<TimeStampEvent> = VecDeque::new();
    let mut pqueue = BTreeMap::new();

    info!("started event process thread");
    let mut close = false;
    loop {

        let mut enqueue_func = |msg, pqueue: &mut BTreeMap<SystemTime, BPCommand>| {
            let timestamp = SystemTime::now();
            match msg {
                ScriptCommand::VibrateFor(strength, time) => {
                    pqueue.insert(timestamp, BPCommand::Vibrate(strength));
                    pqueue.insert(timestamp + Duration::from_secs_f64(time), BPCommand::Stop);
                    //queue.push_back(TimeStampEvent::new(BPCommand::Vibrate(strength), Duration::ZERO));
                    //queue.push_back(TimeStampEvent::new(BPCommand::Stop, Duration::from_secs_f64(time)));
                },
                ScriptCommand::VibrateForWithIndex(strength, time, index) => {
                    pqueue.insert(timestamp, BPCommand::VibrateIndex(strength, index));
                    pqueue.insert(timestamp + Duration::from_secs_f64(time), BPCommand::VibrateIndex(0.0, index));
                    //queue.push_back(TimeStampEvent::new(BPCommand::VibrateIndex(strength, index), Duration::ZERO));
                    //queue.push_back(TimeStampEvent::new(BPCommand::VibrateIndex(0.0, index), Duration::from_secs_f64(time)));
                },
                ScriptCommand::Stop => {
                    pqueue.insert(timestamp, BPCommand::Stop);
                    //queue.push_back(TimeStampEvent::new(BPCommand::Stop, Duration::ZERO));
                },
                ScriptCommand::Close => {
                    close = true;
                },
            }
        };

        if !pqueue.is_empty() { 
            if let Ok(msg) = recv.try_recv() {
                enqueue_func(msg, &mut pqueue);
            };
        } else {
            if let Ok(msg) = recv.recv().await {
                enqueue_func(msg, &mut pqueue);
            }
        }

        if let Some(front) = pqueue.first_entry() {
            if front.key() < &SystemTime::now() {
                let (_key, value) = front.remove_entry();
                info!("submitting command");
                
                match send.send(value) {
                    Ok(_) => {},
                    Err(e) => {
                        info!("raw command send error {}", e);
                        break;
                    },
                }
                
            }
        }
        
        if close {
            break;
        }

    };
    info!("ending event process thread");
    ()
}