use crate::log::Log;
use byteorder::{LittleEndian, WriteBytesExt};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    io::{self, ErrorKind, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs},
};

pub struct NetListener {
    listener: TcpListener,
}

impl NetListener {
    pub fn bind<A: ToSocketAddrs>(addr: A) -> io::Result<Self> {
        let listener = TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;
        Ok(Self { listener })
    }

    pub fn local_address(&self) -> io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    pub fn accept_connections(&self) -> Vec<NetStream> {
        let mut streams = Vec::new();
        while let Ok(result) = self.listener.accept() {
            streams.push(NetStream::from_inner(result.0).unwrap())
        }
        streams
    }
}

pub struct NetStream {
    stream: TcpStream,
    rx_buffer: Vec<u8>,
    tx_buffer: Vec<u8>,
}

impl NetStream {
    pub fn from_inner(stream: TcpStream) -> io::Result<Self> {
        stream.set_nonblocking(true)?;
        stream.set_nodelay(true)?;

        Ok(Self {
            stream,
            rx_buffer: Default::default(),
            tx_buffer: Default::default(),
        })
    }

    pub fn connect<A: ToSocketAddrs>(addr: A) -> io::Result<Self> {
        Self::from_inner(TcpStream::connect(addr)?)
    }

    pub fn send_message<T>(&mut self, data: &T) -> io::Result<()>
    where
        T: Serialize,
    {
        self.tx_buffer.clear();
        if self.tx_buffer.capacity() < std::mem::size_of::<T>() {
            self.tx_buffer.reserve(std::mem::size_of::<T>());
        }
        bincode::serialize_into(&mut self.tx_buffer, data)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        self.stream
            .write_u32::<LittleEndian>(self.tx_buffer.len() as u32)?;
        self.stream.write_all(&self.tx_buffer)?;
        Ok(())
    }

    pub fn peer_address(&self) -> io::Result<SocketAddr> {
        self.stream.peer_addr()
    }

    pub fn string_peer_address(&self) -> String {
        self.peer_address()
            .map(|addr| addr.to_string())
            .unwrap_or_else(|_| "Unknown".into())
    }

    fn next_message<M>(&mut self) -> Option<M>
    where
        M: DeserializeOwned,
    {
        if self.rx_buffer.len() < 4 {
            return None;
        }

        let length = u32::from_le_bytes([
            self.rx_buffer[0],
            self.rx_buffer[1],
            self.rx_buffer[2],
            self.rx_buffer[3],
        ]) as usize;

        let end = 4 + length;

        // The actual data could be missing (i.e. because it is not delivered yet).
        if let Some(data) = self.rx_buffer.as_slice().get(4..end) {
            let message = match bincode::deserialize::<M>(data) {
                Ok(message) => Some(message),
                Err(err) => {
                    Log::err(format!(
                        "Failed to parse a network message of {} bytes long. Reason: {:?}",
                        length, err
                    ));

                    None
                }
            };

            self.rx_buffer.drain(..end);

            message
        } else {
            None
        }
    }

    pub fn process_input<M>(&mut self, mut func: impl FnMut(M))
    where
        M: DeserializeOwned,
    {
        // Receive all bytes from the stream first.
        loop {
            let mut bytes = [0; 8192];
            match self.stream.read(&mut bytes) {
                Ok(bytes_count) => {
                    if bytes_count == 0 {
                        break;
                    } else {
                        self.rx_buffer.extend(&bytes[..bytes_count])
                    }
                }
                Err(err) => match err.kind() {
                    ErrorKind::WouldBlock => {
                        break;
                    }
                    ErrorKind::Interrupted => {
                        // Retry
                    }
                    _ => {
                        Log::err(format!(
                            "An error occurred when reading data from socket: {}",
                            err
                        ));

                        self.rx_buffer.clear();

                        return;
                    }
                },
            }
        }

        // Extract all the messages and process them.
        while let Some(message) = self.next_message() {
            func(message)
        }
    }
}
