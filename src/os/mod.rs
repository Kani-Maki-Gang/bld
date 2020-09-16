#![allow(dead_code)]

use std::path::PathBuf;

macro_rules! path {
    ($($x:expr), *) => {
        {
            let mut path = PathBuf::new();
            $(
                path.push($x);
            )*
            path
        }
    };
}

#[derive(PartialEq, Eq)]
pub enum OSname {
    Windows,
    Mac,
    Linux,
    Unknown,
}

fn is_linux() -> bool {
    let lsb_path = path!["/", "etc", "lsb-release"];

    if lsb_path.is_file() {
        return true;
    };

    let deb_path = path!["/", "etc", "debian-version"];
    if deb_path.is_file() {
        return true;
    }

    false
}

fn is_mac() -> bool {
    let path = path!["System", "Library", "CoreServices", "SystemVersion.plist"];
    path.is_file()
}

fn is_windows() -> bool {
    let path = path!["C:\\", "Windows", "System32"];
    path.is_dir()
}

pub fn name() -> OSname {
    if is_linux() {
        return OSname::Linux;
    }

    if is_mac() {
        return OSname::Mac;
    }

    if is_windows() {
        return OSname::Windows;
    }

    OSname::Unknown
}
