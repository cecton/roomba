use std::io;
use futures::channel::mpsc;
use std::thread;

use termion::event::Key;
use termion::input::TermRead;

pub enum Event<I> {
    Input(I),
}

pub fn new() -> mpsc::UnboundedReceiver<Event<Key>> {
    let (tx, rx) = mpsc::unbounded();
    thread::spawn(move || {
        let stdin = io::stdin();
        for evt in stdin.keys() {
            if let Ok(key) = evt {
                match tx.unbounded_send(Event::Input(key)) {
                    Err(e) if e.is_disconnected() => break,
                    Err(e) => {
                        eprintln!("{}", e);
                    },
                    Ok(()) => {},
                }
            }
        }
    });

    rx
}
