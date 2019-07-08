#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;

use std::{
    error::Error,
    io::{BufWriter, Write},
    net::{SocketAddr, TcpStream, UdpSocket},
    sync::{
        mpsc::{self, Receiver, SyncSender},
        Arc,
    },
    thread,
    time::Duration,
};

use clap::{App, AppSettings, Arg, SubCommand};
use rmpv::ValueRef;

#[derive(Debug)]
enum Address {
    Udp(SocketAddr),
    Https(String, u16),
}

impl Address {
    pub fn from_str(addr: &str) -> Result<Self, Box<dyn Error>> {
        if !addr.contains("://") {
            return Err("invalid address - must be an URL".into());
        }

        let mut it = addr.splitn(2, "://");

        let protocol = match it.next() {
            Some(protocol) => protocol,
            None => return Err("missing protocol".into()),
        };

        let addr = match it.next() {
            Some(addr) => addr,
            None => return Err("missing address".into()),
        };

        match protocol {
            "udp" => {
                let addr = addr.parse()?;
                Ok(Address::Udp(addr))
            }
            "https" => {
                let (host, port) = split_host_port(addr)?;
                Ok(Address::Https(host.into(), port))
            }
            protocol => Err(format!("unknown protocol: {}", protocol).into()),
        }
    }
}

fn split_host_port(addr: &str) -> Result<(&str, u16), Box<dyn Error>> {
    let mut it = addr.rsplitn(2, ':');
    let port = match it.next() {
        Some(port) => port.parse()?,
        None => 443,
    };

    let host = match it.next() {
        Some(host) => host,
        None => return Err("missing hostname".into()),
    };

    Ok((host, port))
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let matches = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .setting(AppSettings::SubcommandRequired)
        .subcommand(SubCommand::with_name("scan").about("scan local network for cleverdog camera(s)"))
        .subcommand(
            SubCommand::with_name("stream")
                .about("stream H264 from camera")
                .arg(
                    Arg::with_name("addr")
                        .long("addr")
                        .value_name("ADDRESS")
                        .help("network address, udp:// or https://")
                        .required(true)
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("retries")
                        .long("retries")
                        .value_name("NUMBER")
                        .default_value("18446744073709551615")
                        .help("number of retries in case of camera hanging")
                        .takes_value(true),
                ),
        )
        .get_matches();

    match matches.subcommand() {
        ("scan", ..) => {
            let info = cleverdog::lookup()?;
            println!("Address: {}", info.addr());
            println!("CID:     {}", core::str::from_utf8(info.cid())?);
            println!("MAC:     {}", info.mac());
            println!("Version: {}", info.version());
        }
        ("stream", Some(matches)) => {
            // This cannot panic because of CLAP required flag.
            let dst = matches.value_of("addr").unwrap();
            let mut num: u64 = matches.value_of("retries").unwrap().parse()?;

            let addr = Address::from_str(dst)?;
            info!("Destination address: {:?}", addr);

            let info = cleverdog::lookup()?;
            info!("Successfully resolved camera");
            info!("  Address: {}", info.addr());
            info!("  CID:     {}", core::str::from_utf8(info.cid())?);
            info!("  MAC:     {}", info.mac());
            info!("  Version: {}", info.version());

            match addr {
                Address::Udp(addr) => {
                    let sock = UdpSocket::bind("0.0.0.0:0")?;

                    cleverdog::stream(info.cid(), info.addr(), |buf| {
                        debug!("-> {}", buf.len());
                        sock.send_to(buf, addr)?;
                        Ok(())
                    })?;
                }
                Address::Https(host, port) => {
                    let addr = format!("{}:{}", host, port);

                    let (tx, rx): (SyncSender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::sync_channel(4096);

                    let thread = thread::spawn(move || {
                        let mut cfg = rustls::ClientConfig::new();
                        cfg.root_store.add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);
                        let cfg = Arc::new(cfg);
                        let hostname = webpki::DNSNameRef::try_from_ascii_str(&host).expect("ASCII hostname");

                        loop {
                            let mut session = rustls::ClientSession::new(&cfg, hostname);

                            debug!("connecting to {}", addr);
                            let mut stream = match TcpStream::connect(&addr) {
                                Ok(stream) => stream,
                                Err(err) => {
                                    error!("failed to connect to {}: {}", addr, err);
                                    thread::sleep(Duration::new(1, 0));
                                    continue;
                                }
                            };

                            let mut stream = BufWriter::new(rustls::Stream::new(&mut session, &mut stream));

                            info!("successfully connected to {}", addr);

                            while let Ok(buf) = rx.recv() {
                                if let Err(err) = stream.write_all(&buf) {
                                    error!("failed to send bytes: {}", err);
                                    break;
                                }
                            }

                            thread::sleep(Duration::new(1, 0));
                        }
                    });

                    let on_data = |buf: &[u8]| {
                        debug!("-> {}", buf.len());

                        let mut msg = Vec::new();
                        if let Err(err) = rmpv::encode::write_value_ref(&mut msg, &ValueRef::Binary(buf)) {
                            error!("failed to encode datagram: {}", err);
                        }

                        if let Err(..) = tx.try_send(msg) {
                            error!("failed to send datagram due to backpressuring");
                        }

                        Ok(())
                    };

                    while num > 0 {
                        if let Err(err) = cleverdog::stream(info.cid(), info.addr(), on_data) {
                            warn!("streaming stopped: {}", err);
                        }

                        num -= 1;
                        thread::sleep(Duration::new(1, 0));
                    }

                    thread.join().unwrap();
                }
            }
        }
        (..) => unreachable!(),
    }

    Ok(())
}

// ffmpeg -protocol_whitelist file,udp,rtp -i /mnt/hls/camera.sdp -preset
// ultrafast -vcodec libx264 -r 15 -b 300k -f flv rtmp://localhost/show/camera0
