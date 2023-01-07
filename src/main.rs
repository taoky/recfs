use crate::fs::RecFs;
use clap::Parser;
use env_logger::Env;
use fuse_mt::{mount, FuseMT};
use std::env;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::process::exit;

mod cache;
mod client;
mod fid;
mod fidmap;
mod fs;

#[derive(Parser)]
pub struct Args {
    #[arg(long, default_value_t = false)]
    /// Clear keyring item before login
    clear: bool,

    /// The mountpoint
    mountpoint: PathBuf,

    #[arg(long, default_value_t = false)]
    /// Request server for non-existing files in local tree structure cache
    no_fast_path: bool,
}

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let cli = Args::parse();
    let fs = RecFs::new(&cli);
    let fuse_args = vec![OsStr::new("-o"), OsStr::new("auto_unmount")];
    mount(FuseMT::new(fs, 1), &cli.mountpoint, &fuse_args).unwrap();
}
