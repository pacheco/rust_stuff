// TODO: implement ServerControl connect_to with retry policies?

use rand;
use ::net::network_to_u32;
use std::slice;
use std::collections::{VecDeque, HashSet};
use std::net::SocketAddr;
pub use mio::Timeout as TimeoutUid;
pub use mio::Sender as Sender;
use mio::{Token, TimerError, EventLoop, EventSet, PollOpt, Handler, TryRead, TryWrite};
use mio::tcp::*;
use mio::util::Slab;
use std::io;
use bytes::{ByteBuf, RingBuf, Buf};

const MAX_MSG_SIZE: usize = 32*1024;
const MSG_HDR_SIZE: usize = 4;

const SERVER: Token = Token(0);

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Timer(TimerError),
    ConnectionLimit,
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<TimerError> for Error {
    fn from(err: TimerError) -> Error {
        Error::Timer(err)
    }
}

/// Asynchronous IO, message-based, TCP Server
pub struct Server<H: ServerHandler> {
    token: Token,
    socket: Option<TcpListener>,
    connections: Slab<Connection>,
    connections_new: VecDeque<TcpStream>,
    // TODO: HashSet is very slow for iterating: slowdown for large numbers of connections
    connections_closed: Option<HashSet<Token>>,
    connections_reregister: Option<HashSet<Token>>,
    handler: Option<H>,
    shutdown: bool,
    shutdown_error: Option<Error>,
}

impl<H: ServerHandler> Server<H> {
    /// Create a new Server that doesn't bind to a local address, can
    /// only connect to others (i.e. a client).
    pub fn new(handler: H, max_connections: usize) -> Result<Self, Error> {
        Ok(Server {
            token: Token(0),
            socket: None,
            connections: Slab::new_starting_at(Token(1), max_connections), // max number of concurrent connections
            connections_new: VecDeque::new(),
            connections_closed: Some(HashSet::new()), // Some(HashSet::with_capacity(max_connections)),
            connections_reregister: Some(HashSet::new()),// Some(HashSet::with_capacity(max_connections)),
            handler: Some(handler),
            shutdown: false,
            shutdown_error: None,
        })
    }

    /// Create a new Server bound to the given address
    pub fn bind(addr: &SocketAddr, handler: H, max_connections: usize) -> Result<Self, Error> {
        let s = try!(TcpListener::bind(&addr));
        Ok(Server {
            token: Token(0),
            socket: Some(s),
            connections: Slab::new_starting_at(Token(1), max_connections), // max number of concurrent connections
            connections_new: VecDeque::new(),
            connections_closed: Some(HashSet::new()), // Some(HashSet::with_capacity(max_connections)),
            connections_reregister: Some(HashSet::new()),// Some(HashSet::with_capacity(max_connections)),
            handler: Some(handler),
            shutdown: false,
            shutdown_error: None,
        })
    }

    /// Start the server's event loop, accepting new connections
    pub fn run(&mut self) -> Result<(), Error> {
        let mut evl = try!(EventLoop::new());
        if let Some(ref s) = self.socket {
            try!(evl.register(s, self.token,
                              EventSet::readable(),
                              PollOpt::edge()));
        }
        let mut h = self.handler.take().unwrap();
        h.init(&mut ServerControl::new(self, &mut evl));
        self.handler = Some(h);
        try!(evl.run(self));
        Ok(())
    }

    fn accept(&mut self) {
        loop {
            match self.socket.as_ref().unwrap().accept() {
                Ok(Some((s, _))) => {
                    self.connections_new.push_back(s);
                }
                Ok(None) => break,
                Err(err) => {
                    error!("error accepting connection: {}", err);
                    self.shutdown_with_err(Error::from(err));
                }
            }
        }
    }

    fn register_new_connections(&mut self, evloop: &mut EventLoop<Self>) {
        while let Some(conn) = self.connections_new.pop_front() {
            match self.connections.insert_with(
                move |token| Connection::new(token, conn, ConnectionState::ReadSize)
            ) {
                Some(token) => {
                    if let Err(err) = self.connections[token].register(evloop) {
                        error!("could not register new connection on event loop");
                        self.shutdown_with_err(Error::from(err));
                    }
                    let mut h = self.handler.take().unwrap();
                    let uid = self.connections[token].uid.clone();
                    debug!("new connection {:?}", uid);
                    h.connection(&mut ServerControl::new(self, evloop), uid);
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
            if let Err(err) = self.connections[uid].reregister(evloop) {
                error!("could not reregister connection on event loop");
                self.shutdown_with_err(Error::from(err));
            }
        }
        self.connections_reregister = Some(uids);
    }

    fn remove_closed_connections(&mut self, evloop: &mut EventLoop<Self>) {
        let mut to_close = self.connections_closed.take().unwrap();
        for token in to_close.drain() {
            if let Some(ref mut c) = self.connections.remove(token) {
                if let Err(err) = c.deregister(evloop) {
                    error!("could not deregister connection from event loop");
                    self.shutdown_with_err(Error::from(err));
                }
                let mut h = self.handler.take().unwrap();
                h.connection_closed(&mut ServerControl::new(self, evloop), &c.uid);
                self.handler = Some(h);
            }
        }
        self.connections_closed = Some(to_close);
    }

    fn shutdown_with_err(&mut self, err: Error) {
        self.shutdown_error = Some(err);
    }
}

pub enum ServerTimeout<T> {
    User(T),
}

impl<H: ServerHandler> Handler for Server<H> {
    type Message = H::Message;
    type Timeout = ServerTimeout<H::Timeout>;

    fn ready(&mut self, evloop: &mut EventLoop<Self>, token: Token, events: EventSet) {
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
                // hup
                if events.is_hup() || events.is_error() {
                    debug!("hup/error event for {:?}", uid);
                    match self.connections[client].state {
                        ConnectionState::Connecting(_) => {
                            let uid = self.connections[client].uid;
                            self.connections[client].deregister(evloop).unwrap();
                            let mut h = self.handler.take().unwrap();
                            h.connect_failed(&mut ServerControl::new(self, evloop), &uid);
                            self.handler = Some(h);
                        }
                        _ => {self.connections_closed.as_mut().unwrap().insert(client);}
                    }
                    return;
                }
                // readable
                if events.is_readable() {
                    loop {
                        match self.connections[client].read_msg() {
                            Ok(ReadResult::Msg(msg)) => {
                                let mut h = self.handler.take().unwrap();
                                h.message(&mut ServerControl::new(self, evloop), &uid, msg);
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
                    if let ConnectionState::Connecting(addr) = self.connections[client].state {
                        debug!("connected to {}", addr);
                        self.connections[client].state = ConnectionState::ReadSize;
                        self.connections[client].interest.insert(EventSet::readable());
                        let uid = self.connections[client].uid;
                        let mut h = self.handler.take().unwrap();
                        h.connection(&mut ServerControl::new(self, evloop), uid);
                        self.handler = Some(h);
                    }
                    if let Err(e) = self.connections[client].write() {
                        debug!("write error for {:?}: {:?}", uid.addr, e);
                        self.connections_closed.as_mut().unwrap().insert(client);
                    } else {
                        self.connections_reregister.as_mut().unwrap().insert(client);
                    }
                }
            }
        }
    }

    #[allow(unused_variables)]
    fn notify(&mut self, evloop: &mut EventLoop<Self>, msg: Self::Message) {
        let mut h = self.handler.take().unwrap();
        h.notify(&mut ServerControl::new(self, evloop), msg);
        self.handler = Some(h);
    }

    #[allow(unused_variables)]
    fn timeout(&mut self, evloop: &mut EventLoop<Self>, timeout: Self::Timeout) {
        match timeout {
            ServerTimeout::User(timeout) => {
                let mut h = self.handler.take().unwrap();
                h.timeout(&mut ServerControl::new(self, evloop), timeout);
                self.handler = Some(h);
            }
        }
    }

    #[allow(unused_variables)]
    fn interrupted(&mut self, evloop: &mut EventLoop<Self>) {
    }

    fn tick(&mut self, evloop: &mut EventLoop<Self>) {
        self.register_new_connections(evloop);
        self.reregister_connections(evloop);
        self.remove_closed_connections(evloop);
        // shutdown check should be the last thing here, to catch the ServerHandler request for shutdown
        if self.shutdown {
            evloop.shutdown();
            let mut h = self.handler.take().unwrap();
            h.shutting_down(self.shutdown_error.take());
            return;
        }
    }
}

/// Used to control the async server: send messages, schedule
/// timeouts, close connections, shutdown and so on
pub struct ServerControl<'a, H: 'a + ServerHandler> {
    server: &'a mut Server<H>,
    evloop: &'a mut EventLoop<Server<H>>,
}


impl<'a, H: ServerHandler> ServerControl<'a, H>{
    fn new(server: &'a mut Server<H>, evloop: &'a mut EventLoop<Server<H>>) -> Self {
        ServerControl {
            server: server,
            evloop: evloop,
        }
    }
    /// Send a msg to the destination. Will ignore a non existing connection
    pub fn send(&mut self, uid: &ConnectionUid, msg: &[u8]) {
        if let Some(c) = self.server.connections.get_mut(uid.token) {
            if &c.uid == uid {
                match c.send_msg(msg) {
                    Ok(false) => {self.server.connections_reregister.as_mut().unwrap().insert(uid.token);}
                    Err(_) => {self.server.connections_closed.as_mut().unwrap().insert(uid.token);}
                    Ok(true) => (), // message already sent, no need to reregister
                }
            }
        }
    }
    /// Send a msg to the destination. Will ignore non existing connections
    pub fn multicast(&mut self, uids: &mut Iterator<Item=&ConnectionUid>, msg: &[u8]) {
        for uid in uids {
            self.send(uid, msg);
        }
    }
    /// Asynchronously connect to the given address. An `Ok(uid)`
    /// result *does not* mean the connection was successful. The
    /// `uid` returned can be used when
    /// `connection()/connect_failed()` are called to determine
    /// success/failure
    pub fn connect(&mut self, addr: SocketAddr) -> Result<ConnectionUid, Error> {
        let conn = try!(TcpStream::connect(&addr));
        let token = try!(match self.server.connections.insert_with(
            move |token| Connection::new(token, conn, ConnectionState::Connecting(addr))
        ) {
            Some(token) => Ok(token),
            None => Err(Error::ConnectionLimit),
        });
        self.server.connections[token].interest.insert(EventSet::writable() | EventSet::error());
        try!(self.server.connections[token].register(self.evloop).or_else(|e| {
            self.server.connections.remove(token);
            error!("could not register socket on event loop");
            Err(e)
        }));
        Ok(self.server.connections[token].uid)
    }
    /// Close the connection
    pub fn close_connection(&mut self, uid: ConnectionUid) {
        self.server.connections_closed.as_mut().unwrap().insert(uid.token);
    }
    /// Shutdown the server
    pub fn shutdown(&mut self) {
        self.server.shutdown = true;
    }
    /// Get a channel for notify msgs
    pub fn notify_channel(&mut self) -> Sender<H::Message> {
        self.evloop.channel()
    }
    /// Schedule a timeout event
    pub fn timeout_ms(&mut self, timeout: H::Timeout, delay: u64) -> Result<TimeoutUid, Error> {
        self.evloop.timeout_ms(ServerTimeout::User(timeout), delay).or_else(|err| Err(Error::from(err)))
    }
    /// Cancel a scheduled timeout
    pub fn timeout_cancel(&mut self, timeout: TimeoutUid) {
        self.evloop.clear_timeout(timeout);
    }
}

enum ConnectionState {
    Connecting(SocketAddr),
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
/// Unique id identifying a given connection.
pub struct ConnectionUid {
    id: u32,
    token: Token,
    addr: SocketAddr,
}

impl Connection {
    fn new(token: Token, socket: TcpStream, state: ConnectionState) -> Self {
        let addr;
        let interest;
        match state {
            ConnectionState::Connecting(_addr) => {
                addr = _addr;
                interest = EventSet::writable() | EventSet::error() | EventSet::hup();
            }
            ConnectionState::ReadSize => {
                addr = socket.peer_addr().unwrap();
                interest = EventSet::readable() | EventSet::error() | EventSet::hup();
            }
            _ => {
                panic!("invalid initial connection state");
            }
        }
        Connection {
            uid: ConnectionUid {
                id: rand::random::<u32>(),
                token: token,
                addr: addr,
            },
            state: state,
            buf: RingBuf::new(MAX_MSG_SIZE+MSG_HDR_SIZE),
            to_send: VecDeque::new(),
            token: token,
            socket: socket,
            interest: interest,
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
                _ => {
                    panic!("reading from invalid connection!");
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

    // Result(true) if the message has already been written out (no
    // need to reregister the connection)
    fn send_msg(&mut self, msg: &[u8]) -> Result<bool, Error> {
        let hdr: &[u8];
        let len = (msg.len() as u32).to_be();
        unsafe {
            hdr = slice::from_raw_parts((&len as *const u32) as *const u8, 4);
        }
        let mut buf = ByteBuf::mut_with_capacity(MSG_HDR_SIZE + msg.len());
        buf.write_slice(hdr);
        buf.write_slice(msg);
        let mut buf = buf.flip();
        // try to write immediatelly
        while buf.remaining() != 0 {
            match self.socket.try_write_buf(&mut buf) {
                Ok(Some(_)) => (),
                Ok(None) => break, // retry later
                Err(e) => {
                    debug!("write error for {:?}: {:?}", self.uid.addr, e);
                    return Err(Error::from(e));
                }
            }
        }
        if buf.remaining() != 0 {
            self.to_send.push_back(buf);
            self.interest.insert(EventSet::writable());
            Ok(false)
        } else {
            Ok(true)
        }
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

// TODO: make the ServerHandler methods return Self? Would it facilitate STM-ish implementations?

#[allow(unused_variables)]
/// Trait for handling server events. The handler can use the
/// `ServerControl` to issue operations such as sending messages,
/// closing connections and setting timeouts.
pub trait ServerHandler {
    type Message: Send;
    type Timeout;
    /// Called right before starting the server's eventloop.
    fn init(&mut self, server: &mut ServerControl<Self>) where Self: Sized {
    }
    /// Called on a new connection. Use the given ConnectionUid to
    /// identify incoming messages and to send messages to the
    /// connection.
    fn connection(&mut self, server: &mut ServerControl<Self>, uid: ConnectionUid) where Self: Sized {
    }
    /// Called on a disconnect
    fn connection_closed(&mut self, server: &mut ServerControl<Self>, uid: &ConnectionUid) where Self: Sized {
    }
    /// Called when a new network message is received
    fn message(&mut self, server: &mut ServerControl<Self>, uid: &ConnectionUid, msg: Vec<u8>) where Self: Sized;
    /// Called when a new notify message is received through the
    /// server's channel (possibly from outside the event loop)
    fn notify(&mut self, server: &mut ServerControl<Self>, msg: Self::Message) where Self: Sized {
    }
    /// Called when a timeout triggers
    fn timeout(&mut self, server: &mut ServerControl<Self>, timeout: Self::Timeout) where Self: Sized {
    }
    /// Called when the server is shutting down.
    fn shutting_down(&mut self, err: Option<Error>) where Self: Sized {
    }
    /// Called whan a connect fails
    fn connect_failed(&mut self, server: &mut ServerControl<Self>, uid: &ConnectionUid)
        where Self: Sized {
    }
}
