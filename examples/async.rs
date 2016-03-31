extern crate rust_stuff;
extern crate mio;
extern crate bytes;
#[macro_use]
extern crate log;
extern crate env_logger;

use std::collections::VecDeque;
use rust_stuff::net::network_to_u32;
use std::net::SocketAddr;
use mio::*;
use mio::tcp::*;
use mio::util::*;
use std::io;
use bytes::{RingBuf, Buf, BufExt, ByteBuf};

const MAX_MSG_SIZE: usize = 32*1024;

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

#[derive(Debug, Clone, Copy)]
pub struct Uid (Token, u64);

pub struct Server<T> where T: MsgHandler {
    uid_next: u64,
    socket: TcpListener,
    token: Token,
    connections: Slab<Connection>,
    handler: T,
    // event handling state
    shutdown: bool,
    new_connections: Vec<TcpStream>,
    closed: Vec<Token>,
}

impl<T> Server<T> where T: MsgHandler {
    pub fn bind(addr: &SocketAddr, handler: T) -> Result<Self, Error> {
        let s = try!(TcpListener::bind(&addr));
        Ok(Server {
            uid_next: 0,
            socket: s,
            token: Token(0),
            connections: Slab::new_starting_at(Token(1), 128), // max number of concurrent connections?
            handler: handler,
            new_connections: vec!(),
            shutdown: false,
            closed: vec!(),
        })
    }

    pub fn register(&self, evloop: &mut EventLoop<Self>) -> Result<(), Error> {
        try!(evloop.register(&self.socket, self.token,
                             EventSet::readable(),
                             PollOpt::edge()));
        Ok(())
    }

    fn accept(&mut self) {
        match self.socket.accept() {
            Ok(Some((s, addr))) => {
                debug!("new connection from {}", addr);
                self.new_connections.push(s);
            }
            Ok(None) => {
                debug!("accept would block");
            }
            Err(err) => {
                error!("error accepting connection: {}", err);
                self.shutdown();
            }
        }
    }

    fn register_new(&mut self, evloop: &mut EventLoop<Self>) {
        while let Some(conn) = self.new_connections.pop() {
            let uid = self.uid_next;
            match self.connections.insert_with(|token| Connection::new(Uid(token, uid), conn)) {
                Some(token) => {
                    if self.connections[token].register(evloop, token).is_err() {
                        error!("could not register new connection on event loop");
                        self.shutdown();
                        break;
                    }
                }
                None => {
                    error!("could not add new connection");
                }
            }
            self.uid_next += 1;
        }
    }

    fn remove_closed(&mut self, evloop: &mut EventLoop<Self>) {
        while let Some(token) = self.closed.pop() {
            match self.connections.remove(token) {
                Some(ref mut c) => {
                    if c.deregister(evloop).is_err() {
                        error!("error deregistering connection");
                    }
                }
                None => panic!("connection not present on slab"),
            }
        }
    }

    fn shutdown(&mut self) {
        info!("shutting down!");
        self.shutdown = true;
    }
}

impl<T> mio::Handler for Server<T> where T: MsgHandler {
    type Timeout = usize;
    type Message = usize;

    fn ready(&mut self, evloop: &mut EventLoop<Self>, token: Token, events: EventSet) {
        match token {
            SERVER => {
                if events.is_readable() {
                    self.accept();
                } else {
                    error!("invalid event set for server socket: {:?}", events);
                    panic!();
                }
            }
            client => {
                let mut interest = None;
                // readable
                if events.is_readable() {
                    match self.connections[client].read(&mut self.handler) {
                        Interest::Drop => {
                            self.closed.push(client);
                            return;
                        }
                        i => interest = Some(i),
                    }
                }
                // writable
                if events.is_writable(){
                    match self.connections[client].write() {
                        Interest::Drop => {
                            self.closed.push(client);
                            return;
                        }
                        i => interest = Some(i),
                    }
                }
                // hup
                if events.is_hup() {
                    debug!("connection closed");
                    debug!("-----------------");
                    self.closed.push(client);
                    return;
                }
                // reregister connection
                if let Some(Interest::Reregister) = interest {
                    if self.connections[client].reregister(evloop, client).is_err() {
                        debug!("error reregistering connection");
                        self.closed.push(client);
                        return;
                    }
                }
            }
        }
    }

    #[allow(unused_variables)]
    fn notify(&mut self, evloop: &mut EventLoop<Self>, msg: Self::Message) {
        debug!("notify");
    }

    #[allow(unused_variables)]
    fn timeout(&mut self, evloop: &mut EventLoop<Self>, timeout: Self::Timeout) {
        debug!("timeout");
    }

    #[allow(unused_variables)]
    fn interrupted(&mut self, evloop: &mut EventLoop<Self>) {
        debug!("interrupted");
    }

    fn tick(&mut self, evloop: &mut EventLoop<Self>) {
        self.register_new(evloop);
        self.remove_closed(evloop);
    }
}

struct Connection {
    // TODO: add a unique id to differentiate new connections (Slab id is reused)
    socket: TcpStream,
    interest: EventSet,
    parser: MsgParser,
    write_bufs: VecDeque<ByteBuf>,
    read_buf: Option<RingBuf>,
}

enum Interest {
    Reregister,
    Drop,
}

impl Connection {
    fn new(uid: Uid, s: TcpStream) -> Self {
        Connection {
            socket: s,
            interest: EventSet::readable() | EventSet::hup(),
            parser: MsgParser::new(uid),
            write_bufs: VecDeque::new(),
            read_buf: Some(RingBuf::new(MAX_MSG_SIZE)),
        }
    }

    fn read<T: MsgHandler>(&mut self, handler: &mut T) -> Interest {
        let mut read_buf = self.read_buf.take().unwrap();
        loop {
            if read_buf.is_full() {
                debug!("message too large: killing connection");
                return Interest::Drop;
            }
            match self.socket.try_read_buf(&mut read_buf) {
                Ok(None) => {
                    debug!("read: would block");
                    break;
                }
                Ok(Some(r)) => {
                    debug!("read: {} bytes", r);
                    if r == 0 {
                        break;
                    }
                    while let Some(resp) = self.parser.try_consume(&mut read_buf, handler) {
                        
                    }
                }
                Err(e) => {
                    debug!("read: error {}", e);
                    self.read_buf = Some(read_buf);
                    return Interest::Drop
                }
            }
        }
        self.read_buf = Some(read_buf);
        Interest::Reregister
    }

    fn write(&mut self) -> Interest {
        debug!("write event");
        Interest::Reregister
    }

    fn register<T: MsgHandler>(&mut self, evloop: &mut EventLoop<Server<T>>, token: Token) -> Result<(), Error> {
        try!(evloop.register(&self.socket, token, self.interest, PollOpt::edge() | PollOpt::oneshot()));
        Ok(())
    }

    fn reregister<T: MsgHandler>(&mut self, evloop: &mut EventLoop<Server<T>>, token: Token) -> Result<(), Error> {
        debug!("reregistering connection for {:?}", self.interest);
        try!(evloop.reregister(&self.socket, token, self.interest, PollOpt::edge() | PollOpt::oneshot()));
        Ok(())
    }

    fn deregister<T: MsgHandler>(&mut self, evloop: &mut EventLoop<Server<T>>) -> Result<(), Error> {
        try!(evloop.deregister(&self.socket));
        Ok(())
    }
}

enum MsgParserState {
    Size,
    Msg(usize),
}

struct MsgParser {
    uid: Uid,
    state: MsgParserState,
    msg: [u8; MAX_MSG_SIZE],
}

impl MsgParser {
    /// create a new `MsgParser`
    fn new(uid: Uid) -> Self {
        MsgParser {
            uid: uid,
            state: MsgParserState::Size,
            msg: [0; MAX_MSG_SIZE],
        }
    }

    /// try to read messages from the buffer
    fn try_consume<T: MsgHandler>(&mut self, buf: &mut RingBuf, handler: &mut T)
                                  -> Option<MsgHandlerResponse> {
        loop {
            match self.state {
                MsgParserState::Size => {
                    debug!("read header buf.remaining {}", buf.remaining());
                    if buf.remaining() >= 4 {
                        let size = network_to_u32(buf.bytes()) as usize;
                        buf.advance(4);
                        self.state = MsgParserState::Msg(size);
                        debug!("read header: {}", size);
                    } else {
                        return None;
                    }
                }
                MsgParserState::Msg(size) => {
                    debug!("read msg buf.remaining {}", buf.remaining());
                    if buf.remaining() >= size {
                        self.state = MsgParserState::Size;
                        let mut msg = &mut self.msg[0..size];
                        buf.read_slice(msg);
                        return Some(handler.handle(self.uid, msg));
                    } else {
                        return None;
                    }
                }
            }
        }
    }
}

pub trait MsgHandler {
    fn handle(&mut self, from: Uid, msg: &[u8]) -> MsgHandlerResponse;
}

pub enum MsgHandlerResponse {
    Send(Uid, ByteBuf),
    Multicast(Vec<Uid>, ByteBuf),
    Wait,
    //WaitFor(time::Duration),
    Drop,
    Multi(Vec<MsgHandlerResponse>),
}

impl MsgHandlerResponse {
    pub fn then(self, next: MsgHandlerResponse) -> MsgHandlerResponse {
        match self {
            MsgHandlerResponse::Multi(mut v) => {
                v.push(next);
                MsgHandlerResponse::Multi(v)
            }
            _ => MsgHandlerResponse::Multi(vec![self, next])
        }
    }
}

impl MsgHandler for () {
    fn handle(&mut self, from: Uid, msg: &[u8]) -> MsgHandlerResponse {
        use MsgHandlerResponse::*;
        debug!("got a msg from {:?}: {}", from, String::from_utf8_lossy(msg));
        Send(from, ByteBuf::from_slice(msg))
            .then(Wait)
    }
}

fn main() {
    env_logger::init().unwrap();
    let addr = "127.0.0.1:10000".parse().unwrap();
    let mut server = Server::bind(&addr, ()).unwrap();
    let mut evloop = EventLoop::new().unwrap();
    server.register(&mut evloop).unwrap();
    evloop.run(&mut server).unwrap();
}
