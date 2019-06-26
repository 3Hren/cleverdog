#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;

use std::{
    error::Error,
    io::Write,
    net::TcpStream,
    sync::mpsc::{self, Receiver, SyncSender},
    thread,
    time::Duration,
};

use clap::{App, AppSettings, Arg, SubCommand};
use native_tls::TlsConnector;
use rmpv::Value;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let matches = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .setting(AppSettings::SubcommandRequired)
        .subcommand(SubCommand::with_name("scan").about("scan local network for cleverdog camera(s)"))
        .subcommand(
            SubCommand::with_name("stream").about("stream H264 from camera").arg(
                Arg::with_name("addr")
                    .long("addr")
                    .value_name("ADDRESS")
                    .help("network address")
                    .required(true)
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
            let dst = matches.value_of("addr").unwrap().to_owned();

            let info = cleverdog::lookup()?;
            info!("Successfully resolved camera");
            info!("  Address: {}", info.addr());
            info!("  CID:     {}", core::str::from_utf8(info.cid())?);
            info!("  MAC:     {}", info.mac());
            info!("  Version: {}", info.version());

            const PORT: u16 = 444;

            let (tx, rx): (SyncSender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::sync_channel(4096);

            let thread = thread::spawn(move || {
                let connector = TlsConnector::new().unwrap();

                loop {
                    debug!("connecting to {}", dst);
                    let addr = format!("{}:{}", dst, PORT);
                    let stream = match TcpStream::connect(&addr) {
                        Ok(stream) => stream,
                        Err(err) => {
                            error!("failed to connect to {}: {}", addr, err);
                            break;
                        }
                    };

                    let mut stream = match connector.connect(&dst, stream) {
                        Ok(stream) => stream,
                        Err(err) => {
                            error!("failed to connect to {}: {}", addr, err);
                            break;
                        }
                    };

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

            cleverdog::stream(info.cid(), info.addr(), |buf| {
                let mut msg = Vec::new();
                if let Err(err) = rmpv::encode::write_value(&mut msg, &Value::Binary(buf.into())) {
                    error!("failed to encode datagram: {}", err);
                }

                if let Err(..) = tx.try_send(msg) {
                    error!("failed to send datagram due to backpressuring");
                }

                Ok(())
            })?;

            thread.join().unwrap();
        }
        (..) => unreachable!(),
    }

    Ok(())
}
