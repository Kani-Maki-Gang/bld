use anyhow::{anyhow, Result};
use web_sys::window;

pub fn build_url(route: &str) -> Result<String> {
    let window = window().ok_or_else(|| anyhow!("window not found"))?;
    let origin = window
        .location()
        .origin()
        .map_err(|_| anyhow!("unable to find window origin"))?;
    Ok(format!("{origin}{route}"))
}
