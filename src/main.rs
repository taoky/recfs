use crate::fs::RecFs;
use fuse_mt::{mount, FuseMT};
use std::env;
use std::ffi::{OsStr, OsString};
use std::process::exit;

mod client;
mod fid;
mod fidmap;
mod fs;

fn main() {
    let args = env::args_os().collect::<Vec<OsString>>();
    if args.len() != 3 {
        println!(
            "usage: {} <target> <mountpoint>",
            env::args().next().unwrap()
        );
        exit(-1);
    }
    let fs = RecFs::new("token".to_owned());
    let fuse_args = vec![OsStr::new("-o"), OsStr::new("auto_unmount")];
    mount(FuseMT::new(fs, 1), &args[2], &fuse_args).unwrap();
}
