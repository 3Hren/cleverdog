use std::{
    convert::TryFrom,
    error::Error,
    io::{Cursor, Read, Write},
    net::{SocketAddr, UdpSocket},
    time::{Duration, Instant, SystemTime},
};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use crate::protocol::MAGIC;

pub mod protocol;

enum Command {
    Scan,
    ScanReply,
    StartRtp,
}

impl Command {
    pub fn as_u16(&self) -> u16 {
        match self {
            Command::Scan => 0x1004,
            Command::ScanReply => 0x100e,
            Command::StartRtp => 0x1007,
        }
    }
}

impl From<Command> for u16 {
    #[inline]
    fn from(v: Command) -> u16 {
        v.as_u16()
    }
}

#[derive(Debug)]
pub struct ScanInfo {
    /// Camera MAC address.
    mac: String,
    /// Firmware version.
    version: String,
}

impl TryFrom<&[u8]> for ScanInfo {
    type Error = Box<dyn Error>;

    fn try_from(v: &[u8]) -> Result<Self, Self::Error> {
        let mut it = v.split(|&ch| ch == b'\0');

        let mac = match it.next() {
            Some(mac) => String::from_utf8(mac.into())?,
            None => return Err("missing mac address".into()),
        };

        let version = match it.next() {
            Some(version) => String::from_utf8(version.into())?,
            None => return Err("missing version".into()),
        };

        let v = Self { mac, version };

        Ok(v)
    }
}

#[derive(Debug)]
pub struct LookupInfo {
    /// Camera endpoint.
    addr: SocketAddr,
    /// Camera ID.
    cid: [u8; 16],
    /// Scan info.
    info: ScanInfo,
}

impl LookupInfo {
    /// Returns socket address where the camera is bound.
    #[inline]
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Returns camera's client id.
    #[inline]
    pub fn cid(&self) -> &[u8] {
        &self.cid[..]
    }

    /// Returns camera's MAC address.
    #[inline]
    pub fn mac(&self) -> &str {
        &self.info.mac
    }

    /// Returns camera's firmware version.
    #[inline]
    pub fn version(&self) -> &str {
        &self.info.version
    }
}

pub fn lookup() -> Result<LookupInfo, Box<dyn Error>> {
    let sock = UdpSocket::bind("0.0.0.0:0")?;
    sock.set_broadcast(true)?;

    let comm = create_command(Command::Scan, b"", b"00000000000000000000000000000000000000")?;
    sock.send_to(&comm, "192.168.1.71:10008")?;

    let mut buf = [0; 4096];

    loop {
        let (size, addr) = sock.recv_from(&mut buf[..])?;

        let mut buf = Cursor::new(&buf[..size]);

        let mut magic = [0; 2];
        buf.read_exact(&mut magic[..])?;

        if magic != MAGIC {
            return Err("invalid magic header".into());
        }

        let comm = buf.read_u16::<BigEndian>()?;

        if comm != Command::ScanReply.as_u16() {
            continue;
        }

        let mut cid = [0; 16];
        buf.read_exact(&mut cid[..])?;

        let idx = buf.position() as usize;
        let info = LookupInfo {
            addr,
            cid,
            info: ScanInfo::try_from(&buf.into_inner()[idx..])?,
        };

        return Ok(info);
    }
}

pub fn stream(cid: &[u8], src: SocketAddr, dst: SocketAddr) -> Result<(), Box<dyn Error>> {
    let sock = UdpSocket::bind("0.0.0.0:0")?;
    let local_addr = sock.local_addr()?;

    let mut args = Cursor::new(Vec::new());
    args.write_all(b"00000000000000000000000000000000000000")?;
    args.write_fmt(format_args!("{}:{}\0", local_addr.port(), local_addr.port()))?;

    let comm = create_command(Command::StartRtp, cid, &args.into_inner())?;
    sock.send_to(&comm, src)?;

    let mut timestamp = Instant::now();
    let mut buf = [0; 4096];
    loop {
        let (size, addr) = sock.recv_from(&mut buf[..]).unwrap();

        if timestamp.elapsed() >= Duration::from_secs(1) {
            timestamp = Instant::now();
            send_rtcp(&sock, &addr)?;
        }

        if buf[..size].len() < 16 {
            continue;
        }

        // Skip non-video frames.
        if buf[2] != 1 {
            continue;
        }

        sock.send_to(&buf[4..size], dst)?;
    }

    Ok(())
}

fn send_rtcp(sock: &UdpSocket, camera: &SocketAddr) -> Result<(), Box<dyn Error>> {
    let mut buf = Cursor::new(Vec::new());

    buf.write_all(&[
        0x00, 0x00, 0x01, 0x00, // Header.
        0x80, // RTP v2
        0xc8, // RTCP sender report packet type
        0x00, 0x06,
    ])?;
    buf.write_u32::<BigEndian>(0x00000002)?;

    let msecs = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?.as_nanos() / 1e6 as u128 + 2208988800000;
    let seconds = (msecs / 1000) as u32;
    let fraction = (0x100000000 * (msecs % 1000) / 1000) as u32;

    buf.write_u32::<BigEndian>(seconds)?;
    buf.write_u32::<BigEndian>(fraction)?;
    buf.write_u32::<BigEndian>(0)?;
    buf.write_u32::<BigEndian>(0)?;
    buf.write_u32::<BigEndian>(0)?;

    sock.send_to(&buf.into_inner(), camera)?;

    Ok(())
}

fn create_command(cmd: Command, mut cid: &[u8], args: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    if cid.len() > 15 {
        cid = &cid[..15]
    }

    let mut buf = Cursor::new(Vec::new());

    buf.write_all(MAGIC)?;
    buf.write_u16::<BigEndian>(cmd.into())?;
    buf.write_all(cid)?;
    buf.write_all(&b"000000000000000"[..15 - cid.len()])?;
    buf.write_all(&[0x0])?;
    buf.write_all(args)?;

    Ok(buf.into_inner())
}
