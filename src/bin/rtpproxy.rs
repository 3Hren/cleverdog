#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;

use std::{
    error::Error,
    io::{Cursor, ErrorKind, Read},
    net::{SocketAddr, TcpListener, TcpStream, UdpSocket},
};

use clap::{App, AppSettings, Arg};
use rmpv::ValueRef;

fn process(mut stream: TcpStream) -> Result<(), Box<dyn Error>> {
    let local_addr = stream.local_addr()?;

    let sock = UdpSocket::bind("0.0.0.0:0").unwrap();
    let addr: SocketAddr = "127.0.0.1:8088".parse().unwrap();

    let mut rx_offset = 0;
    let mut rd_offset = 0;
    let mut buf = [0; 8192];
    loop {
        match stream.read(&mut buf[rd_offset..]) {
            Ok(0) => {
                info!("EOF {}", local_addr);
                return Ok(());
            }
            Ok(nread) => {
                debug!("received {} bytes from {}", nread, local_addr);
                rd_offset += nread;

                loop {
                    let mut rdbuf = Cursor::new(&buf[rx_offset..rd_offset]);

                    match rmpv::decode::read_value_ref(&mut rdbuf) {
                        Ok(ValueRef::Binary(v)) => {
                            if let Err(err) = sock.send_to(v, addr) {
                                error!("failed to recast: {}", err);
                            }

                            rx_offset += rdbuf.position() as usize;
                        }
                        Ok(..) => {
                            return Err("unexpected frame".into());
                        }
                        Err(ref err) if err.kind() == ErrorKind::UnexpectedEof => {
                            break;
                        }
                        Err(err) => {
                            error!("I/O error: {}", err);
                            return Err(err.into());
                        }
                    }
                }

                let pending = rd_offset - rx_offset;
                if rx_offset != 0 {
                    unsafe {
                        core::ptr::copy(buf.as_ptr().offset(rx_offset as isize), buf.as_mut_ptr(), pending);
                    }

                    rd_offset = pending;
                    rx_offset = 0;
                }
            }
            Err(err) => {
                error!("I/O error: {}", err);
                return Err(err.into());
            }
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let matches = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .setting(AppSettings::SubcommandRequired)
        .arg(
            Arg::with_name("addr")
                .long("addr")
                .value_name("ADDRESS")
                .help("network address to listen")
                .required(true)
                .takes_value(true),
        )
        .get_matches();

    // This cannot panic because of CLAP required flag.
    let addr = matches.value_of("addr").unwrap();

    let listener = TcpListener::bind(&addr)?;
    info!("listening {}", listener.local_addr()?);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(err) = process(stream) {
                    warn!("failed to process stream: {}", err);
                }
            }
            Err(err) => {
                warn!("failed to process stream: {}", err);
            }
        }
    }

    Ok(())
}
