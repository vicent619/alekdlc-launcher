use std::fs::{self, File};
use std::io::{BufWriter, ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use crate::install::download::Partial;
use crate::net::error::ApiError;

const REQUEST: u32 = 56;
const RESPONSE: u32 = 57;
const VERSION: u32 = 4;
const WAITING: u8 = 0;
const READY: u8 = 1;
const ERROR: u8 = 2;
const MAX_JAR: u64 = 192 * 1024 * 1024;
const MAX_ERROR: u64 = 512;
const HEADER: u64 = 9;
const CHUNK: usize = 64 * 1024;
const POLL: Duration = Duration::from_millis(250);
const IDLE_LIMIT: Duration = Duration::from_secs(30);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const WAIT_DELAY: Duration = Duration::from_secs(1);
const RETRY_DELAY: Duration = Duration::from_secs(3);
const BACKOFF_MIN: Duration = Duration::from_secs(1);
const BACKOFF_MAX: Duration = Duration::from_secs(10);
const UNREACHABLE_LIMIT: Duration = Duration::from_secs(60);
const QUEUE_LIMIT: Duration = Duration::from_secs(300);

pub enum Progress {
    Connecting,
    Waiting,
    Receiving { done: u64, total: u64 },
}

enum Reply {
    Waiting,
    Ready,
    Rejected(String),
}

enum Failure {
    Network,
    Protocol,
    Io,
    Cancelled,
}

pub struct ClientFetcher<'a, C: Fn() -> bool> {
    address: String,
    token: &'a str,
    cancelled: C,
}

impl<'a, C: Fn() -> bool> ClientFetcher<'a, C> {
    pub fn new(address: String, token: &'a str, cancelled: C) -> Self {
        Self { address, token, cancelled }
    }

    pub fn fetch<F: FnMut(Progress)>(&self, target: &Path, mut emit: F) -> Result<(), ApiError> {
        let mut connection: Option<Connection> = None;
        let mut backoff = BACKOFF_MIN;
        let mut contact = Instant::now();
        let mut queued: Option<Instant> = None;
        emit(Progress::Connecting);
        loop {
            if (self.cancelled)() {
                return Err(cancelled());
            }
            let active = match connection.as_mut() {
                Some(active) => active,
                None => match Connection::open(&self.address) {
                    Ok(opened) => {
                        contact = Instant::now();
                        connection.insert(opened)
                    }
                    Err(_) => {
                        if contact.elapsed() >= UNREACHABLE_LIMIT {
                            return Err(unreachable());
                        }
                        emit(Progress::Connecting);
                        self.pause(backoff).map_err(Failure::into_error)?;
                        backoff = (backoff * 2).min(BACKOFF_MAX);
                        continue;
                    }
                },
            };
            match active.exchange(self.token, target, &mut emit, &self.cancelled) {
                Ok(Reply::Ready) => return Ok(()),
                Ok(Reply::Waiting) => {
                    backoff = BACKOFF_MIN;
                    contact = Instant::now();
                    if queued.get_or_insert(contact).elapsed() >= QUEUE_LIMIT {
                        return Err(ApiError::new(crate::secret!("CLIENT_TIMEOUT"), 0));
                    }
                    emit(Progress::Waiting);
                    self.pause(WAIT_DELAY).map_err(Failure::into_error)?;
                }
                Ok(Reply::Rejected(message)) if transient(&message) => {
                    connection = None;
                    emit(Progress::Waiting);
                    self.pause(RETRY_DELAY).map_err(Failure::into_error)?;
                }
                Ok(Reply::Rejected(message)) => return Err(ApiError::new(crate::secret!("CLIENT_REJECTED"), 0).with_detail(message)),
                Err(Failure::Network) => {
                    connection = None;
                    if contact.elapsed() >= UNREACHABLE_LIMIT {
                        return Err(unreachable());
                    }
                    emit(Progress::Connecting);
                    self.pause(backoff).map_err(Failure::into_error)?;
                    backoff = (backoff * 2).min(BACKOFF_MAX);
                }
                Err(failure) => return Err(failure.into_error()),
            }
        }
    }

    fn pause(&self, duration: Duration) -> Result<(), Failure> {
        let deadline = Instant::now() + duration;
        while Instant::now() < deadline {
            if (self.cancelled)() {
                return Err(Failure::Cancelled);
            }
            thread::sleep(POLL.min(deadline.saturating_duration_since(Instant::now())));
        }
        Ok(())
    }
}

struct Connection {
    stream: TcpStream,
}

impl Connection {
    fn open(address: &str) -> Result<Self, Failure> {
        let targets: Vec<SocketAddr> = address.to_socket_addrs().map_err(|_| Failure::Network)?.collect();
        for target in targets {
            if let Ok(stream) = TcpStream::connect_timeout(&target, CONNECT_TIMEOUT) {
                stream.set_read_timeout(Some(POLL)).map_err(|_| Failure::Network)?;
                stream.set_write_timeout(Some(IDLE_LIMIT)).map_err(|_| Failure::Network)?;
                let _ = stream.set_nodelay(true);
                return Ok(Self { stream });
            }
        }
        Err(Failure::Network)
    }

    fn exchange<F: FnMut(Progress), C: Fn() -> bool>(
        &mut self,
        token: &str,
        target: &Path,
        emit: &mut F,
        cancelled: &C,
    ) -> Result<Reply, Failure> {
        self.stream.write_all(&request(token)).map_err(|_| Failure::Network)?;
        let length = u64::from(self.read_u32(cancelled)?);
        if !(HEADER..=HEADER + 4 + MAX_JAR).contains(&length) {
            return Err(Failure::Protocol);
        }
        if self.read_u32(cancelled)? != RESPONSE || self.read_u32(cancelled)? != VERSION {
            return Err(Failure::Protocol);
        }
        let status = self.read_array::<1, C>(cancelled)?[0];
        let rest = length - HEADER;
        match status {
            WAITING if rest == 0 => Ok(Reply::Waiting),
            READY => {
                let size = self.payload_size(rest, 1, MAX_JAR, cancelled)?;
                self.receive(size, target, emit, cancelled)?;
                Ok(Reply::Ready)
            }
            ERROR => {
                let size = self.payload_size(rest, 1, MAX_ERROR, cancelled)?;
                let mut message = vec![0u8; size as usize];
                self.read_exact(&mut message, cancelled)?;
                Ok(Reply::Rejected(String::from_utf8_lossy(&message).trim().to_owned()))
            }
            _ => Err(Failure::Protocol),
        }
    }

    fn payload_size<C: Fn() -> bool>(&mut self, rest: u64, min: u64, max: u64, cancelled: &C) -> Result<u64, Failure> {
        let size = u64::from(self.read_u32(cancelled)?);
        if rest != 4 + size || !(min..=max).contains(&size) {
            return Err(Failure::Protocol);
        }
        Ok(size)
    }

    fn receive<F: FnMut(Progress), C: Fn() -> bool>(
        &mut self,
        size: u64,
        target: &Path,
        emit: &mut F,
        cancelled: &C,
    ) -> Result<(), Failure> {
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|_| Failure::Io)?;
        }
        let partial = Partial::new(target);
        let file = File::create(&partial.path).map_err(|_| Failure::Io)?;
        let mut writer = BufWriter::with_capacity(CHUNK * 16, file);
        let mut buffer = vec![0u8; CHUNK];
        let mut done = 0u64;
        emit(Progress::Receiving { done, total: size });
        while done < size {
            let take = (size - done).min(CHUNK as u64) as usize;
            self.read_exact(&mut buffer[..take], cancelled)?;
            writer.write_all(&buffer[..take]).map_err(|_| Failure::Io)?;
            done += take as u64;
            emit(Progress::Receiving { done, total: size });
        }
        let file = writer.into_inner().map_err(|_| Failure::Io)?;
        file.sync_all().map_err(|_| Failure::Io)?;
        drop(file);
        partial.commit(target).map_err(|_| Failure::Io)
    }

    fn read_u32<C: Fn() -> bool>(&mut self, cancelled: &C) -> Result<u32, Failure> {
        Ok(u32::from_be_bytes(self.read_array::<4, C>(cancelled)?))
    }

    fn read_array<const N: usize, C: Fn() -> bool>(&mut self, cancelled: &C) -> Result<[u8; N], Failure> {
        let mut bytes = [0u8; N];
        self.read_exact(&mut bytes, cancelled)?;
        Ok(bytes)
    }

    fn read_exact<C: Fn() -> bool>(&mut self, buffer: &mut [u8], cancelled: &C) -> Result<(), Failure> {
        let mut filled = 0;
        let mut last = Instant::now();
        while filled < buffer.len() {
            match self.stream.read(&mut buffer[filled..]) {
                Ok(0) => return Err(Failure::Network),
                Ok(read) => {
                    filled += read;
                    last = Instant::now();
                }
                Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                    if cancelled() {
                        return Err(Failure::Cancelled);
                    }
                    if last.elapsed() >= IDLE_LIMIT {
                        return Err(Failure::Network);
                    }
                }
                Err(error) if error.kind() == ErrorKind::Interrupted => {}
                Err(_) => return Err(Failure::Network),
            }
        }
        Ok(())
    }
}

impl Failure {
    fn into_error(self) -> ApiError {
        match self {
            Failure::Network => ApiError::offline(),
            Failure::Protocol => ApiError::new(crate::secret!("CLIENT_PROTOCOL"), 0),
            Failure::Io => ApiError::new(crate::secret!("IO"), 0),
            Failure::Cancelled => cancelled(),
        }
    }
}

fn unreachable() -> ApiError {
    ApiError::new(crate::secret!("CLIENT_UNREACHABLE"), 0)
}

fn cancelled() -> ApiError {
    ApiError::new(crate::secret!("CANCELLED"), 0)
}

fn transient(message: &str) -> bool {
    message.starts_with(crate::secret!("INPUT_REPLACED")) || message.starts_with(crate::secret!("SERVER_SHUTTING_DOWN"))
}

fn request(token: &str) -> Vec<u8> {
    let token = token.as_bytes();
    let body = 12 + token.len();
    let mut frame = Vec::with_capacity(4 + body);
    frame.extend_from_slice(&(body as u32).to_be_bytes());
    frame.extend_from_slice(&REQUEST.to_be_bytes());
    frame.extend_from_slice(&VERSION.to_be_bytes());
    frame.extend_from_slice(&(token.len() as u32).to_be_bytes());
    frame.extend_from_slice(token);
    frame
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_frame_layout() {
        let frame = request("B8BO2.abc");
        assert_eq!(&frame[0..4], &21u32.to_be_bytes());
        assert_eq!(&frame[4..8], &56u32.to_be_bytes());
        assert_eq!(&frame[8..12], &4u32.to_be_bytes());
        assert_eq!(&frame[12..16], &9u32.to_be_bytes());
        assert_eq!(&frame[16..], b"B8BO2.abc");
    }
}
