mod application;
mod presentation;

use arx_core::error::Result;

fn main() -> Result<()> {
    application::run()
}
