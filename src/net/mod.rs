pub mod server;

use std::io;
use std::io::{BufReader, Write, Read};
use std::net::{TcpStream, Shutdown};
use net2::TcpStreamExt;
use std::slice;

const HDR: usize = 4;

#[derive(Debug)]
pub enum NetError {
    Io(io::Error),
    FrameTooBig(usize),
}

impl From<io::Error> for NetError {
    fn from(err: io::Error) -> NetError {
        NetError::Io(err)
    }
}

pub struct FramedTcpStream {
    /// Tcp connection
    instream: BufReader<TcpStream>,
    outstream: TcpStream,
    /// Size header len,
    h: [u8; HDR],
}

impl FramedTcpStream {
    /// Create a new `TcpFrameReader` reading from the given
    /// stream. Each frame is preceded by a 4 bytes 'len' header.
    pub fn new(stream: TcpStream) -> FramedTcpStream {
        stream.set_nodelay(true).unwrap();
        FramedTcpStream {instream: BufReader::new(stream.try_clone().unwrap()),
                         outstream: stream,
                         h: [0; HDR]}
    }

    pub fn shutdown(&mut self, how: Shutdown) -> Result<(), NetError> {
        try!(self.outstream.shutdown(how));
        Ok(())
    }

    /// Read and return the next frame
    pub fn read_frame(&mut self) -> Result<Vec<u8>, NetError> {
        try!(self.instream.read_exact(&mut self.h));
        let len = network_to_u32(&self.h) as usize;
        let mut msg = vec![0;len];
        try!(self.instream.read_exact(msg.as_mut_slice()));
        Ok(msg)
    }

    /// Read the next frame into the given buffer and return its
    /// size. The buffer should be large enough to contain the
    /// message.
    pub fn read_frame_into(&mut self, buf: &mut [u8]) -> Result<usize, NetError> {
        try!(self.instream.read_exact(&mut self.h));
        let len = network_to_u32(&self.h) as usize;
        if buf.len() < len {
            Err(NetError::FrameTooBig(len))
        } else {
            try!(self.instream.read_exact(&mut buf[0..len]));
            Ok(len)
        }
    }

    /// Writes a frame preceded by its length to the stream
    pub fn write_frame(&mut self, frame: &[u8]) -> Result<(), NetError> {
        let len = (frame.len() as u32).to_be();
        let bytes: &[u8];
        unsafe {
            bytes = slice::from_raw_parts((&len as *const u32) as *const u8, 4);
        }
        try!(self.outstream.write_all(bytes));
        try!(self.outstream.write_all(frame));
        Ok(())
    }

    /// Writes data directly into the underlying stream (no framing)
    pub unsafe fn raw_write(&mut self, bytes: &[u8]) -> Result<(), NetError> {
        try!(self.outstream.write_all(bytes));
        Ok(())
    }
}

impl Iterator for FramedTcpStream {
    type Item = Vec<u8>;
    /// Returns the next frame or None in case of error
    fn next(&mut self) -> Option<Vec<u8>> {
        self.read_frame().ok()
    }
}

pub fn network_to_u32(bytes: &[u8]) -> u32 {
    debug_assert!(bytes.len() >= 4);
    let p = &bytes[0] as *const u8 as *const u32;
    unsafe {
        u32::from_be(*p)
    }
}
