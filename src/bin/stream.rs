#[macro_use]
extern crate clap;

use std::{
    error::Error,
    net::TcpStream,
    io::Write,
    sync::mpsc::{self, Receiver, SyncSender},
    thread,
};

use clap::{App, AppSettings, Arg, SubCommand};
use native_tls::TlsConnector;
use rmpv::Value;

fn main() -> Result<(), Box<dyn Error>> {
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
            println!("{:?}", info);

            const PORT: u16 = 444;

            let (tx, rx): (SyncSender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::sync_channel(4096);

            let thread = thread::spawn(move || {
                let connector = TlsConnector::new().unwrap();

                loop {
                    println!("[  ] connect {}", dst);
                    let stream = TcpStream::connect(format!("{}:{}", dst, PORT)).unwrap();
                    println!("[OK] connect {}", dst);

                    let mut stream = connector.connect(&dst, stream).unwrap();

                    while let Ok(buf) = rx.recv() {
                        if let Err(err) = stream.write_all(&buf) {
                            println!("[ERROR] failed to send bytes: {}", err);
                            break;
                        }
                    }
                }
            });

            cleverdog::stream(info.cid(), info.addr(), |buf| {
                let mut msg = Vec::new();
                if let Err(err) = rmpv::encode::write_value(&mut msg, &Value::Binary(buf.into())) {
                    println!("[ERROR] failed to encode: {}", err);
                }

                if let Err(..) = tx.try_send(msg) {
                    println!("[ERROR] failed to send");
                }

                Ok(())
            })?;

            thread.join().unwrap();
        }
        (..) => unreachable!(),
    }

    Ok(())
}
