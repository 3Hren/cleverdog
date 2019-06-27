#[macro_use]
extern crate log;

use core::{convert::TryFrom, time::Duration};
use std::{
    error::Error,
    io::{Cursor, Read, Write},
    net::{SocketAddr, UdpSocket},
    time::{Instant, SystemTime},
};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use crate::{
    protocol::{LookupInfo, ScanInfo, MAGIC},
    rtp::Header,
};

pub mod mac;
pub mod protocol;
mod rtp;

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

pub fn lookup() -> Result<LookupInfo, Box<dyn Error>> {
    let sock = UdpSocket::bind("0.0.0.0:0")?;
    sock.set_broadcast(true)?;
    sock.set_read_timeout(Some(Duration::new(1, 0)))?;

    let comm = create_command(Command::Scan, b"", b"00000000000000000000000000000000000000")?;
    sock.send_to(&comm, "192.168.1.71:10008")?;

    let mut buf = [0; 4096];

    loop {
        let (size, addr) = sock.recv_from(&mut buf[..])?;

        let mut buf = Cursor::new(&buf[..size]);

        let magic = buf.read_u16::<BigEndian>()?;

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
        let info = ScanInfo::try_from(&buf.into_inner()[idx..])?;
        let info = LookupInfo::new(addr, cid, info);

        return Ok(info);
    }
}

pub fn stream<F>(cid: &[u8], src: SocketAddr, f: F) -> Result<(), Box<dyn Error>>
where
    F: Fn(&[u8]) -> Result<(), Box<dyn Error>>,
{
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

        let hdr = Header::from_slice(&buf[4..])?;

        if hdr.version() != 2 {
            continue;
        }

        // Skip non-video frames.
        if buf[2] != 1 {
            continue;
        }

        if hdr.ssrc() != 16 {
            continue;
        }

        debug!("Sequence number: {}", hdr.sequence_number());
        debug!("Timestamp      : {}", hdr.timestamp());

        f(&buf[4..size])?;
    }
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

    buf.write_u16::<BigEndian>(MAGIC)?;
    buf.write_u16::<BigEndian>(cmd.into())?;
    buf.write_all(cid)?;
    buf.write_all(&b"000000000000000"[..15 - cid.len()])?;
    buf.write_all(&[0x0])?;
    buf.write_all(args)?;

    Ok(buf.into_inner())
}
