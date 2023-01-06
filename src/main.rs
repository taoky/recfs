use crate::fs::RecFs;
use fuse_mt::{mount, FuseMT};
use std::env;
use std::ffi::{OsStr, OsString};
use std::process::exit;

mod cache;
mod client;
mod fid;
mod fidmap;
mod fs;

fn main() {
    env_logger::init();

    let args = env::args_os().collect::<Vec<OsString>>();
    if args.len() != 2 {
        println!("usage: {} <mountpoint>", env::args().next().unwrap());
        exit(-1);
    }
    let fs = RecFs::new();
    let fuse_args = vec![OsStr::new("-o"), OsStr::new("auto_unmount")];
    mount(FuseMT::new(fs, 1), &args[1], &fuse_args).unwrap();
}
