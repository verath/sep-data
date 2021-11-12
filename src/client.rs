use crate::{parser, se_types::SEOutputData};
use std::{
    cmp,
    io::{self, Read},
    net::{TcpStream, UdpSocket},
};
use thiserror::Error;

pub type Packet = Vec<SEOutputData>;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("could not connect")]
    Connect(#[source] io::Error),

    #[error("failed disconnecting")]
    Disconnect(#[source] io::Error),

    #[error("read failed")]
    Read(#[source] io::Error),

    #[error("read would block")]
    ReadWouldBlock,

    #[error("invalid packet")]
    InvalidPacket(#[source] parser::ParseFailedError),
}

pub trait Client {
    fn connect(&mut self) -> Result<(), ClientError>;
    fn disconnect(&mut self) -> Result<(), ClientError>;

    fn next(&mut self) -> Result<Packet, ClientError>;
}

struct TcpStreamReader {
    stream: TcpStream,
    buf: Vec<u8>,
    pos: usize,
}

impl TcpStreamReader {
    pub fn new(stream: TcpStream) -> Self {
        let buf = Vec::new();
        TcpStreamReader {
            stream,
            buf,
            pos: 0,
        }
    }

    fn buffer(&mut self) -> &[u8] {
        &self.buf[self.pos..]
    }

    fn grow(&mut self, wanted: usize) -> Result<(), ClientError> {
        let available = self.buf.capacity() - self.pos;
        if available < wanted {
            // Not enough remaining capacity, have to re-allocate.
            // Here we also reset pos, by moving the remaining last items from
            // old Vec to the front of the new Vec.
            let required = self.pos + wanted;
            let new_cap = cmp::max(required, self.buf.capacity());
            let mut new_buf: Vec<u8> = Vec::with_capacity(new_cap);
            new_buf.extend(&self.buf[self.pos..]);
            self.buf = new_buf;
            self.pos = 0;
        }
        let old_len = self.buf.len();
        self.buf.resize(old_len + wanted, 0u8);
        // TODO: handle server disconnected (read returning n=0?).
        self.stream
            .read_exact(&mut self.buf[old_len..])
            .map_err(|e| match e {
                ref e if e.kind() == io::ErrorKind::WouldBlock => ClientError::ReadWouldBlock,
                _ => ClientError::Read(e),
            })
    }

    fn reserve(&mut self, additional: usize) -> Result<(), ClientError> {
        let current_length = self.buffer().len();
        if current_length < additional {
            let needed_length = additional - current_length;
            self.grow(needed_length)?;
        }
        Ok(())
    }

    pub fn peek(&mut self, n: usize) -> Result<&[u8], ClientError> {
        self.reserve(n)?;
        Ok(&self.buffer()[..n])
    }

    pub fn consume(&mut self, n: usize) {
        if self.buffer().len() < n {
            panic!("consume out of range")
        }
        self.pos += n;
    }

    pub fn read(&mut self, n: usize) -> Result<&[u8], ClientError> {
        self.reserve(n)?;
        let prev_pos = self.pos;
        self.pos += n;
        Ok(&self.buf[prev_pos..self.pos])
    }
}

enum TCPClientState {
    Pending { addr: String },
    Connected { stream_reader: TcpStreamReader },
    Disconnected,
}

pub struct TCPClient {
    state: TCPClientState,
}

impl TCPClient {
    pub fn new(hostname: &str, port: u16) -> Self {
        let addr = format!("{}:{}", hostname, port);
        let state = TCPClientState::Pending { addr };
        TCPClient { state }
    }
}

impl Client for TCPClient {
    fn connect(&mut self) -> Result<(), ClientError> {
        match &self.state {
            TCPClientState::Pending { addr } => {
                let stream = TcpStream::connect(addr.as_str()).map_err(ClientError::Connect)?;
                stream.set_nonblocking(true).map_err(ClientError::Connect)?;
                let stream_reader = TcpStreamReader::new(stream);
                self.state = TCPClientState::Connected { stream_reader };
                Ok(())
            }
            _ => panic!("invalid state"),
        }
    }

    fn disconnect(&mut self) -> Result<(), ClientError> {
        match &self.state {
            TCPClientState::Connected { stream_reader } => {
                let shutdown_res = stream_reader
                    .stream
                    .shutdown(std::net::Shutdown::Both)
                    .map_err(ClientError::Disconnect);
                self.state = TCPClientState::Disconnected;
                shutdown_res
            }
            _ => panic!("invalid state"),
        }
    }

    fn next(&mut self) -> Result<Packet, ClientError> {
        if let TCPClientState::Connected { stream_reader } = &mut self.state {
            // Seek stream until we find a valid packet header.
            let packet_header = loop {
                let header_buf = stream_reader.peek(parser::PACKET_HEADER_SIZE)?;
                if let Ok(packet_header) = parser::parse_packet_header(header_buf) {
                    stream_reader.consume(parser::PACKET_HEADER_SIZE);
                    break packet_header;
                } else {
                    // Invalid header, skip forward 1 byte.
                    stream_reader.consume(1);
                }
            };
            // Parse packet data.
            let packet_data = stream_reader.read(packet_header.length as usize)?;
            parser::parse_packet_data(packet_header, packet_data)
                .map_err(ClientError::InvalidPacket)
        } else {
            panic!("invalid state")
        }
    }
}

enum UDPClientState {
    Pending { addr: String },
    Connected { socket: UdpSocket, buf: Vec<u8> },
    Disconnected,
}

pub struct UDPClient {
    state: UDPClientState,
}

impl UDPClient {
    pub fn new(port: u16) -> Self {
        let addr = format!("0.0.0.0:{}", port);
        let state = UDPClientState::Pending { addr };
        UDPClient { state }
    }
}

impl Client for UDPClient {
    fn connect(&mut self) -> Result<(), ClientError> {
        match &self.state {
            UDPClientState::Pending { addr } => {
                let socket = UdpSocket::bind(addr.as_str()).map_err(ClientError::Connect)?;
                socket.set_nonblocking(true).map_err(ClientError::Connect)?;
                // Pre-allocate buf.
                let mut buf = Vec::new();
                buf.resize(u16::MAX as usize, 0);
                self.state = UDPClientState::Connected { socket, buf };
                Ok(())
            }
            _ => panic!("invalid state"),
        }
    }

    fn disconnect(&mut self) -> Result<(), ClientError> {
        match self.state {
            UDPClientState::Connected { .. } => {
                self.state = UDPClientState::Disconnected;
                Ok(())
            }
            _ => panic!("invalid state"),
        }
    }

    fn next(&mut self) -> Result<Packet, ClientError> {
        if let UDPClientState::Connected { socket, buf } = &mut self.state {
            buf.resize(u16::MAX as usize, 0);
            let (n, _from) = socket.recv_from(&mut buf[..]).map_err(|e| match e {
                ref e if e.kind() == io::ErrorKind::WouldBlock => ClientError::ReadWouldBlock,
                _ => ClientError::Read(e),
            })?;
            parser::parse_packet(&buf[..n]).map_err(ClientError::InvalidPacket)
        } else {
            panic!("invalid state")
        }
    }
}
