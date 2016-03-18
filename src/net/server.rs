use std::collections::HashMap;
use std::net::{TcpListener, Shutdown, SocketAddr};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::io;

use net::{FramedTcpStream, NetError};

#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub struct Uid(u64);

impl Uid {
    pub fn next(&self) -> Uid {
        Uid(self.0 + 1)
    }
}

const QUEUE_SIZE: usize = 32*1024;

#[derive(Debug)]
pub enum Event {
    Recv(Uid, Vec<u8>),
    Connected(Uid),
    Disconnected(Uid),
    UnexpectedError(MuxServerError),
}

#[derive(Debug)]
pub enum MuxServerError {
    NotConnected,
    Net(NetError),
    InvalidState(&'static str),
}

impl From<NetError> for MuxServerError {
    fn from(err: NetError) -> MuxServerError {
        MuxServerError::Net(err)
    }
}

impl From<io::Error> for MuxServerError {
    fn from(err: io::Error) -> MuxServerError {
        MuxServerError::Net(NetError::Io(err))
    }
}

impl From<&'static str> for MuxServerError {
    fn from(err: &'static str) -> MuxServerError {
        MuxServerError::InvalidState(err)
    }
}

/// Multiplexing server for framed messages over TCP.
pub struct MuxServer {
    addr: SocketAddr,
    listener: Option<TcpListener>,
    events: (SyncSender<Event>, Receiver<Event>),
    connections: HashMap<Uid, FramedTcpStream>,
    new_connections: Arc<Mutex<HashMap<Uid, FramedTcpStream>>>,
}

impl MuxServer {
    pub fn new(addr: SocketAddr) -> MuxServer {
        MuxServer {
            addr: addr,
            listener: None,
            events: sync_channel(QUEUE_SIZE), // TODO: how many events should be allowed here? make it unbounded?
            connections: HashMap::new(),
            // new_connections is used to avoid mutexes in the critical path
            new_connections: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Start accepting connections.
    pub fn start(&mut self) -> Result<(), MuxServerError> {
        self.listener = Some(try!(TcpListener::bind(&self.addr)));
        let l = try!(self.listener.as_ref().unwrap().try_clone());
        let ev = self.events.0.clone();
        let nc = self.new_connections.clone();
        // start accept thread
        thread::spawn(move || { stream_receiver(l, Uid(0), ev, nc) });
        Ok(())
    }

    /// Send a frame to the given destination. It should be connected already.
    pub fn send_to(&mut self, dest: Uid, data: &[u8]) -> Result<(), MuxServerError> {
        match self.connections.get_mut(&dest) {
            Some(s) => {
                try!(s.write_frame(data));
            }
            None => {
                return Err(MuxServerError::NotConnected);
            }
        }
        Ok(())
    }

    pub fn shutdown(&mut self) -> Result<(), MuxServerError> {
        for (_, c) in self.connections.iter_mut() {
            c.shutdown(Shutdown::Both).is_ok(); // don't care about result
        }
        Ok(())
    }

    fn next_event(&mut self) -> Result<Event, MuxServerError> {
        match self.events.1.recv() {
            Ok(Event::Connected(uid)) => {
                let mut newc = match self.new_connections.lock() {
                    Ok(newc) => newc,
                    Err(_) => return Err(MuxServerError::from("Mutex lock() error")),
                };
                match newc.remove(&uid) {
                    Some(c) => {
                        self.connections.insert(uid, c);
                        Ok(Event::Connected(uid))
                    }
                    None => {
                        Err(MuxServerError::from("Connected event with no connection"))
                    }
                }
            }
            Ok(Event::Disconnected(uid)) => {
                match self.connections.remove(&uid) {
                    Some(_) => {
                        Ok(Event::Disconnected(uid))
                    }
                    None => {
                        Err(MuxServerError::from("Disconnected event with no connection"))
                    }
                }
            }
            Ok(evt) => {
                Ok(evt)
            }
            Err(_) => Err(MuxServerError::from("Channel error")),
        }
    }
}

impl Iterator for MuxServer {
    type Item = Event;
    /// Block waiting for the next `Event`. Will never return
    /// `None`. Errors will be returned as `Event::UnexpectedError`.
    fn next(&mut self) -> Option<Event> {
        self.next_event().map_err(move |err| -> Result<Event, MuxServerError> {
            Ok(Event::UnexpectedError(err))
        }).ok()
    }
}

fn stream_receiver(l: TcpListener,
         uid: Uid,
         events: SyncSender<Event>,
         new_connections: Arc<Mutex<HashMap<Uid, FramedTcpStream>>>) {
    match l.accept() {
        Ok((stream, _)) => {
            // accept more connections
            {
                let new_connections = new_connections.clone();
                let events = events.clone();
                let uid = uid.next();
                thread::spawn(move || {
                    stream_receiver(l, uid, events, new_connections)
                });
            }
            match stream.try_clone() {
                Ok(outstream) => {
                    // register connection
                    let mut nc = new_connections.lock().unwrap(); // FIXME: is unwrap fine here?
                    nc.insert(uid, FramedTcpStream::new(outstream));
                    drop(nc);
                    // signal connected and start receiving
                    let mut stream = FramedTcpStream::new(stream);
                    if events.send(Event::Connected(uid)).is_ok() {
                        while let Ok(frame) = stream.read_frame() {
                            if events.send(Event::Recv(uid, frame)).is_err() {
                                break;
                            }
                        }
                    }
                    // signal disconnect and shutdown the connection
                    if events.send(Event::Disconnected(uid)).is_err() {
                        stream.shutdown(Shutdown::Both).unwrap();
                    }
                }
                Err(_) => {
                    events.send(Event::UnexpectedError(MuxServerError::from("Error cloning stream"))).unwrap();
                }
            }
        }
        Err(e) => {
            events.send(Event::UnexpectedError(MuxServerError::from(NetError::from(e)))).unwrap();
        }
    }
}
