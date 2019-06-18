#[macro_use]
extern crate clap;

use std::error::Error;

use clap::{App, AppSettings, Arg, SubCommand};

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
            let dst = matches.value_of("addr").unwrap().parse()?;

            let info = cleverdog::lookup()?;
            println!("{:?}", info);
            cleverdog::stream(info.cid(), info.addr(), dst)?;
        }
        (..) => unreachable!(),
    }

    Ok(())
}
