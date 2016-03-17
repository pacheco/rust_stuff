use std::io;
use std::io::{Read, Write};
use std::net::{TcpStream};
use std::slice;

const HDR: usize = 4;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    FrameTooBig(usize),
}


impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

/// Wrapper for frames over a `TcpStream`
pub struct FramedTcpStream {
    /// Tcp connection
    stream: TcpStream,
    /// Size header len,
    h: [u8; HDR],
}

impl FramedTcpStream {
    /// Create a new `TcpFrameReader` reading from the given
    /// stream. Each frame is preceded by a 4 bytes 'len' header.
    pub fn new(stream: TcpStream) -> FramedTcpStream {
        FramedTcpStream {stream: stream, h: [0; HDR]}
    }

    /// Read and return the next frame
    pub fn read_frame(&mut self) -> Result<Vec<u8>, Error> {
        try!(self.stream.read_exact(&mut self.h));
        let len = network_to_u32(&self.h) as usize;
        let mut msg = vec![0;len];
        try!(self.stream.read_exact(msg.as_mut_slice()));
        Ok(msg)
    }

    /// Read the next frame into the given buffer and return its
    /// size. The buffer should be large enough to contain the
    /// message.
    pub fn read_frame_into(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        try!(self.stream.read_exact(&mut self.h));
        let len = network_to_u32(&self.h) as usize;
        if buf.len() < len {
            Err(Error::FrameTooBig(len))
        } else {
            try!(self.stream.read_exact(&mut buf[0..len]));
            Ok(len)
        }
    }

    /// Writes a frame preceded by its length to the stream
    pub fn write_frame(&mut self, frame: &[u8]) -> Result<(), Error> {
        let len = (frame.len() as u32).to_be();
        let bytes: &[u8];
        unsafe {
            bytes = slice::from_raw_parts((&len as *const u32) as *const u8, 4);
        }
        try!(self.stream.write_all(bytes));
        try!(self.stream.write_all(frame));
        Ok(())
    }

    /// Writes data directly into the underlying stream (no framing)
    pub unsafe fn raw_write(&mut self, bytes: &[u8]) -> Result<(), Error> {
        try!(self.stream.write_all(bytes));
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

fn network_to_u32(bytes: &[u8]) -> u32 {
    debug_assert!(bytes.len() >= 4);
    let p = &bytes[0] as *const u8 as *const u32;
    unsafe {
        u32::from_be(*p)
    }
}
