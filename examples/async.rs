extern crate rust_stuff;
extern crate mio;
extern crate bytes;
#[macro_use]
extern crate log;
extern crate env_logger;

use std::net::SocketAddr;
use mio::*;
use mio::tcp::*;
use mio::util::*;
use std::io;
use bytes::{ByteBuf};

const MAX_CONNECTIONS: usize = 128;
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

struct Server<H: ServerHandler> {
    token: Token,
    socket: TcpListener,
    connections: Slab<Connection>,
    connections_new: Vec<TcpStream>,
    connections_closed: Vec<Token>,
    handler: H,
    shutdown: bool,
}

impl<H: ServerHandler> Server<H> {
    /// Create and bind a to the given address
    pub fn bind(addr: &SocketAddr, handler: H) -> Result<Self, Error> {
        let s = try!(TcpListener::bind(&addr));
        Ok(Server {
            token: Token(0),
            socket: s,
            connections: Slab::new_starting_at(Token(1), MAX_CONNECTIONS), // max number of concurrent connections?
            connections_new: vec!(),
            connections_closed: vec!(),
            handler: handler,
            shutdown: false,
        })
    }

    /// Register the server in the event loop
    pub fn register(&self, evloop: &mut EventLoop<Self>) -> Result<(), Error> {
        try!(evloop.register(&self.socket, self.token,
                             EventSet::readable(),
                             PollOpt::edge()));
        Ok(())
    }

    fn accept(&mut self) {
        match self.socket.accept() {
            Ok(Some((s, addr))) => {
                self.connections_new.push(s);
            }
            Ok(None) => debug!("accept would block"),
            Err(err) => {
                error!("error accepting connection: {}", err);
                self.shutdown();
            }
        }
    }

    fn shutdown(&mut self) {
        info!("shutting down!");
        self.shutdown = true;
    }

    fn register_new_connections(&mut self, evloop: &mut EventLoop<Self>) {
        while let Some(conn) = self.connections_new.pop() {
            match self.connections.insert_with(|token| Connection::new(token, conn)) {
                Some(token) => {
                    if self.connections[token].register(evloop).is_err() {
                        error!("could not register new connection on event loop");
                        self.shutdown();
                        break;
                    }
                }
                None => {
                    error!("cannot accept new connection: limit reached");
                }
            }
        }
    }

    fn remove_closed_connections(&mut self, evloop: &mut EventLoop<Self>) {
        while let Some(token) = self.connections_closed.pop() {
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
                self.connections[client].ready(evloop, events);
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
        self.register_new_connections(evloop);
        self.remove_closed_connections(evloop);
    }
}

struct Connection {
    token: Token,
    addr: SocketAddr,
    socket: TcpStream,
    interest: EventSet,
}

impl Connection {
    fn new(token: Token, socket: TcpStream) -> Self {
        Connection {
            token: token,
            addr: socket.peer_addr().unwrap(),
            socket: socket,
            interest: EventSet::readable(),
        }
    }

    fn register<H: ServerHandler>(&mut self, evloop: &mut EventLoop<Server<H>>) -> Result<(), Error> {
        try!(evloop.register(&self.socket, self.token, self.interest, PollOpt::edge() | PollOpt::oneshot()));
        debug!("registering new connection {}", self.addr);
        Ok(())
    }

    fn reregister<H: ServerHandler>(&mut self, evloop: &mut EventLoop<Server<H>>) -> Result<(), Error> {
        debug!("reregistering connection {} for {:?}", self.addr, self.interest);
        try!(evloop.reregister(&self.socket, self.token, self.interest, PollOpt::edge() | PollOpt::oneshot()));
        Ok(())
    }

    fn deregister<H: ServerHandler>(&mut self, evloop: &mut EventLoop<Server<H>>) -> Result<(), Error> {
        debug!("deregistering connection {}", self.addr);
        try!(evloop.deregister(&self.socket));
        Ok(())
    }

    fn ready<H: ServerHandler>(&mut self, evloop: &mut EventLoop<Server<H>>, events: EventSet) {
        debug!("events for {}: {:?}", self.addr, events);
    }
}

#[derive(Copy, Clone, Debug)]
struct ConnectionUid(u32, Token, SocketAddr);

trait ServerControl {
    fn send(&mut self, uid: &ConnectionUid, msg: ByteBuf);
    fn multicast(&mut self, uids: Iterator<Item=&ConnectionUid>, msg: ByteBuf);
    fn close_connection(&mut self, uid: &ConnectionUid);
    fn shutdown(&mut self);
}

trait ServerHandler {
    fn connection(&mut self, uid: ConnectionUid);
    fn connection_closed(&mut self, uid: ConnectionUid);
    fn message(&mut self, uid: ConnectionUid, msg: ByteBuf);
    fn shutting_down(&mut self, err: Option<Error>);
}

// ------------------------------------------------------

impl ServerHandler for () {
    fn connection(&mut self, uid: ConnectionUid) {}
    fn connection_closed(&mut self, uid: ConnectionUid) {}
    fn message(&mut self, uid: ConnectionUid, msg: ByteBuf) {}
    fn shutting_down(&mut self, err: Option<Error>) {}
}

fn main() {
    env_logger::init().unwrap();
    let addr = "127.0.0.1:10000".parse().unwrap();
    let mut server = Server::bind(&addr, ()).unwrap();
    let mut evloop = EventLoop::new().unwrap();
    server.register(&mut evloop).unwrap();
    evloop.run(&mut server).unwrap();
}
