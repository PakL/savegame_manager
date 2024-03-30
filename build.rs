use std::{env, io};
use winres::WindowsResource;

fn main() -> io::Result<()> {
    if env::var_os("CARGO_CFG_WINDOWS").is_some() {
        WindowsResource::new()
            .set_icon_with_id("assets/icon.ico", "MAINICON")
            .compile()?;
    }
    Ok(())
}