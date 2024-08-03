use std::{fs::create_dir, io::Error, path::PathBuf};

fn main() -> Result<(), Error> {
    let path = PathBuf::from("static_files");

    if !path.exists() {
        create_dir(&path)?;
    }

    Ok(())
}
