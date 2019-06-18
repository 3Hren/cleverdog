use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let dst = std::env::args().skip(1).next().unwrap().parse()?;

    dbg!(&dst);
    let info = cleverdog::lookup()?;
    dbg!(&info);

    cleverdog::stream(info.cid(), info.addr(), dst)?;

    Ok(())
}
