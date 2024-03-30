use std::{env, io};
use winres::WindowsResource;

fn main() -> io::Result<()> {
    if env::var_os("CARGO_CFG_WINDOWS").is_some() {
        WindowsResource::new()
            .set_language(0x0409) // English (US)
            .set("CompanyName", "pakl.dev")
            .set("LegalCopyright", "2024 by pakl.dev")
            .set_icon_with_id("assets/icon.ico", "MAINICON")
            .compile()?;
    }
    Ok(())
}