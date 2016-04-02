use rand;
use ::net::network_to_u32;
use std::slice;
use std::collections::{VecDeque, HashSet};
use std::net::SocketAddr;
use mio::*;
use mio::tcp::*;
use mio::util::Slab;
use std::io;
use bytes::{ByteBuf, RingBuf, Buf};

const MAX_CONNECTIONS: usize = 128;
const MAX_MSG_SIZE: usize = 32*1024;
const MSG_HDR_SIZE: usize = 4;

const SERVER: Token = Token(0);

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

pub struct Server<H: ServerHandler> {
    token: Token,
    socket: TcpListener,
    connections: Slab<Connection>,
    connections_new: VecDeque<TcpStream>,
    connections_closed: Option<HashSet<Token>>,
    connections_reregister: Option<HashSet<Token>>,
    handler: Option<H>,
    shutdown: bool,
}

impl<H: ServerHandler> Server<H> {
    /// Create a new Server bound to the given address
    pub fn bind(addr: &SocketAddr, handler: H) -> Result<Self, Error> {
        let s = try!(TcpListener::bind(&addr));
        Ok(Server {
            token: Token(0),
            socket: s,
            connections: Slab::new_starting_at(Token(1), MAX_CONNECTIONS), // max number of concurrent connections
            connections_new: VecDeque::new(),
            connections_closed: Some(HashSet::with_capacity(2*MAX_CONNECTIONS)),
            connections_reregister: Some(HashSet::with_capacity(2*MAX_CONNECTIONS)),
            handler: Some(handler),
            shutdown: false,
        })
    }

    pub fn run(&mut self) -> Result<(), Error> {
        let mut evl = try!(EventLoop::new());
        try!(evl.register(&self.socket, self.token,
                          EventSet::readable(),
                          PollOpt::edge()));
        try!(evl.run(self));
        Ok(())
    }

    fn accept(&mut self) {
        loop {
            match self.socket.accept() {
                Ok(Some((s, _))) => {
                    self.connections_new.push_back(s);
                }
                Ok(None) => break,
                Err(err) => {
                    error!("error accepting connection: {}", err);
                    self.shutdown();
                }
            }
        }
    }

    fn register_new_connections(&mut self, evloop: &mut EventLoop<Self>) {
        while let Some(conn) = self.connections_new.pop_front() {
            match self.connections.insert_with(move |token| Connection::new(token, conn)) {
                Some(token) => {
                    if self.connections[token].register(evloop).is_err() {
                        panic!("could not register new connection on event loop");
                    }
                    let mut h = self.handler.take().unwrap();
                    let uid = self.connections[token].uid.clone();
                    debug!("new connection {:?}", uid);
                    h.connection(self, uid);
                    self.handler = Some(h);
                }
                None => {
                    error!("cannot accept new connection: limit reached");
                }
            }
        }
    }

    fn reregister_connections(&mut self, evloop: &mut EventLoop<Self>) {
        let mut uids = self.connections_reregister.take().unwrap();
        for uid in uids.drain() {
            if let Err(e) = self.connections[uid].reregister(evloop) {
                panic!("error reregistering connection on event loop: {:?}", e);
            }
        }
        self.connections_reregister = Some(uids);
    }

    fn remove_closed_connections(&mut self, evloop: &mut EventLoop<Self>) {
        let mut to_close = self.connections_closed.take().unwrap();
        for token in to_close.drain() {
            if let Some(ref mut c) = self.connections.remove(token) {
                if let Err(e) = c.deregister(evloop) {
                    panic!("error deregistering connection from event loop: {:?}", e);
                }
                let mut h = self.handler.take().unwrap();
                h.connection_closed(self, &c.uid);
                self.handler = Some(h);
            }
        }
        self.connections_closed = Some(to_close);
    }
}

struct ServerMessage<M: Send> {
    token: Token,
    msg: M,
}

struct ServerTimeout<T> {
    token: Token,
    timeout: T,
}

impl<H: ServerHandler> Handler for Server<H> {
    type Message = ();
    type Timeout = ();

    fn ready(&mut self, _evloop: &mut EventLoop<Self>, token: Token, events: EventSet) {
        match token {
            SERVER => {
                if events.is_readable() {
                    self.accept();
                } else {
                    panic!("invalid event set for server socket: {:?}", events);
                }
            }
            client => {
                let uid = self.connections[client].uid.clone();
                // readable
                if events.is_readable() {
                    loop {
                        match self.connections[client].read_msg() {
                            Ok(ReadResult::Msg(msg)) => {
                                let mut h = self.handler.take().unwrap();
                                h.message(self, &uid, msg);
                                self.handler = Some(h);
                            }
                            Ok(ReadResult::None) => {
                                self.connections_reregister.as_mut().unwrap().insert(client);
                                break;
                            }
                            Ok(ReadResult::Closed) => {
                                self.connections_closed.as_mut().unwrap().insert(client);
                                break;
                            }
                            Err(e) => {
                                debug!("read error from {:?}: {:?}", uid.addr, e);
                                self.connections_closed.as_mut().unwrap().insert(client);
                                break;
                            }
                        }
                    }
                }
                // writable
                if events.is_writable(){
                    if let Err(e) = self.connections[client].write() {
                        debug!("write error for {:?}: {:?}", uid.addr, e);
                        self.connections_closed.as_mut().unwrap().insert(client);
                    } else {
                        self.connections_reregister.as_mut().unwrap().insert(client);
                    }
                }
                // hup
                if events.is_hup() {
                    debug!("hup event for {:?}", uid);
                    self.connections_closed.as_mut().unwrap().insert(client);
                }
            }
        }
    }

    #[allow(unused_variables)]
    fn notify(&mut self, evloop: &mut EventLoop<Self>, msg: Self::Message) {
    }

    #[allow(unused_variables)]
    fn timeout(&mut self, evloop: &mut EventLoop<Self>, timeout: Self::Timeout) {
    }

    #[allow(unused_variables)]
    fn interrupted(&mut self, evloop: &mut EventLoop<Self>) {
    }

    fn tick(&mut self, evloop: &mut EventLoop<Self>) {
        self.register_new_connections(evloop);
        self.reregister_connections(evloop);
        self.remove_closed_connections(evloop);
        // TODO: shutdown if self.shutdown == true
    }
}

pub trait ServerControl {
    /// Send a msg to the destination. Will ignore a non existing connection
    fn send(&mut self, uid: &ConnectionUid, msg: &[u8]);
    /// Send a msg to the destination. Will ignore non existing connections
    fn multicast(&mut self, uids: &mut Iterator<Item=&ConnectionUid>, msg: &[u8]);
    /// Close the connection
    fn close_connection(&mut self, uid: ConnectionUid);
    /// Shutdown the server
    fn shutdown(&mut self);
}


impl<H: ServerHandler> ServerControl for Server<H> {
    fn send(&mut self, uid: &ConnectionUid, msg: &[u8]) {
        if let Some(c) = self.connections.get_mut(uid.token) {
            if &c.uid == uid {
                c.send_msg(msg);
                self.connections_reregister.as_mut().unwrap().insert(uid.token);
            }
        }
    }
    fn multicast(&mut self, uids: &mut Iterator<Item=&ConnectionUid>, msg: &[u8]) {
        for uid in uids {
            self.send(uid, msg);
        }
    }
    fn close_connection(&mut self, uid: ConnectionUid) {
        self.connections_closed.as_mut().unwrap().insert(uid.token);
    }
    fn shutdown(&mut self) {
        self.shutdown = true;
    }
}

enum ConnectionState {
    ReadSize,
    ReadData(usize),
}

struct Connection {
    uid: ConnectionUid,
    state: ConnectionState,
    buf: RingBuf,
    to_send: VecDeque<ByteBuf>, // TODO: copy into a single buffer to avoid calling read multiple times?
    token: Token,
    socket: TcpStream,
    interest: EventSet,
}

enum ReadResult {
    // read a message
    Msg(Vec<u8>),
    // would block
    None,
    // connection closed
    Closed,
}


#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConnectionUid {
    id: u32,
    token: Token,
    addr: SocketAddr,
}

impl Connection {
    fn new(token: Token, socket: TcpStream) -> Self {
        Connection {
            uid: ConnectionUid {
                id: rand::random::<u32>(),
                token: token,
                addr: socket.peer_addr().unwrap()
            },
            state: ConnectionState::ReadSize,
            buf: RingBuf::new(MAX_MSG_SIZE+MSG_HDR_SIZE),
            to_send: VecDeque::new(),
            token: token,
            socket: socket,
            interest: EventSet::readable() | EventSet::hup(),
        }
    }

    fn register<H: ServerHandler>(&mut self, evloop: &mut EventLoop<Server<H>>) -> Result<(), Error> {
        try!(evloop.register(&self.socket, self.token, self.interest, PollOpt::edge() | PollOpt::oneshot()));
        Ok(())
    }

    fn reregister<H: ServerHandler>(&mut self, evloop: &mut EventLoop<Server<H>>) -> Result<(), Error> {
        try!(evloop.reregister(&self.socket, self.token, self.interest, PollOpt::edge() | PollOpt::oneshot()));
        Ok(())
    }

    fn deregister<H: ServerHandler>(&mut self, evloop: &mut EventLoop<Server<H>>) -> Result<(), Error> {
        try!(evloop.deregister(&self.socket));
        Ok(())
    }

    // try to parse a message from the connection buffer
    fn try_parse_msg(&mut self) -> Option<Vec<u8>> {
        loop {
            match self.state {
                ConnectionState::ReadSize => {
                    if self.buf.remaining() >= 4 {
                        let size = network_to_u32(self.buf.bytes()) as usize;
                        self.buf.advance(4);
                        self.state = ConnectionState::ReadData(size);
                    } else {
                        return None;
                    }
                }
                ConnectionState::ReadData(size) => {
                    if self.buf.remaining() >= size {
                        self.state = ConnectionState::ReadSize;
                        // TODO: use a fixed buf/vec in the connection
                        // to avoid allocation for every msg (passing
                        // slice to the handler)
                        let mut msg = Vec::with_capacity(size);
                        let r = self.buf.try_read_buf(&mut msg).unwrap().unwrap();
                        debug_assert_eq!(size, r);
                        return Some(msg);
                    } else {
                        return None;
                    }
                }
            }
        }
    }

    // try to read a whole message
    fn read_msg(&mut self) -> Result<ReadResult, Error> {
        if let Some(msg) = self.try_parse_msg() {
            return Ok(ReadResult::Msg(msg));
        }
        loop {
            match self.socket.try_read_buf(&mut self.buf) {
                Ok(None) => {
                    return Ok(ReadResult::None);
                }
                Ok(Some(r)) => {
                    if r == 0 {
                        return Ok(ReadResult::Closed);
                    }
                    if let Some(msg) = self.try_parse_msg() {
                        return Ok(ReadResult::Msg(msg));
                    }
                }
                Err(e) => {
                    return Err(Error::from(e));
                }
            }
        }
    }

    fn send_msg(&mut self, msg: &[u8]) {
        let mut buf = ByteBuf::mut_with_capacity(MSG_HDR_SIZE + msg.len());
        let hdr: &[u8];
        let len = (msg.len() as u32).to_be();
        unsafe {
            hdr = slice::from_raw_parts((&len as *const u32) as *const u8, 4);
        }
        buf.write_slice(hdr);
        buf.write_slice(msg);
        self.to_send.push_back(buf.flip());
        // try to write immediatelly
        if let Err(e) = self.write() {
            debug!("write error for {:?}: {:?}", self.uid.addr, e);
        }
        self.interest.insert(EventSet::writable());
    }

    fn write(&mut self) -> Result<(), Error> {
        while !self.to_send.is_empty() {
            match self.socket.try_write_buf(&mut self.to_send[0]) {
                Ok(Some(_)) => (),
                Ok(None) => break, // retry later
                Err(e) => return Err(Error::from(e)),
            }
            if self.to_send[0].remaining() == 0 {
                self.to_send.pop_front().unwrap();
            }
        }
        if self.to_send.is_empty() {
            self.interest.remove(EventSet::writable());
        }
        Ok(())
    }
}

#[allow(unused_variables)]
pub trait ServerHandler {
    fn connection(&mut self, server: &mut ServerControl, uid: ConnectionUid) {
    }
    fn connection_closed(&mut self, server: &mut ServerControl, uid: &ConnectionUid) {
    }
    fn message(&mut self, server: &mut ServerControl, uid: &ConnectionUid, msg: Vec<u8>);
    fn shutting_down(&mut self, server: &mut ServerControl, err: Option<Error>) {
    }
}
