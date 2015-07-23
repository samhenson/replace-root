// Copyright (C) 2015 Sam Henson
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

#![feature(duration)]
#![feature(path_ext)]
#![feature(path_relative_from)]
#![feature(start)]
#![feature(thread_sleep)]
#![feature(rt)]

use std::fs;
use std::fs::PathExt;
use std::path::Path;
use std::thread;
use std::time::Duration;

#[macro_use]
extern crate syscall;

const MS_RDONLY : u32 = 1;
const MS_REMOUNT : u32 = 32;
const MS_MOVE : u32 = 8192;

fn remount_root(flags : u32) {
    let ret = unsafe {
        syscall!(MOUNT, "\0".as_ptr(), "/\0".as_ptr(), "ext3\0".as_ptr(), MS_REMOUNT | flags, "\0".as_ptr())
    };
    println!("mount() -> {}", ret);
}

fn umount(path : &str) {
    let ret = unsafe {
        syscall!(UMOUNT2, format!("{}\0", path).as_ptr(), 0)
    };
    println!("umount() -> {}", ret);
}

fn move_mount(from : &str, to : &str) {
    let ret = unsafe {
        syscall!(MOUNT, format!("{}\0", from).as_ptr(), format!("{}\0", to).as_ptr(), 0, MS_MOVE, 0)
    };
    println!("move_mount() -> {}", ret);
}

fn sync() {
    unsafe {
        syscall!(SYNC)
    };
}

const LINUX_REBOOT_MAGIC1 : u32 = 0xfee1dead;
const LINUX_REBOOT_MAGIC2 : u32 = 672274793;
const LINUX_REBOOT_CMD_RESTART : u32 = 0x1234567;

fn reboot() {
    let ret = unsafe {
        syscall!(REBOOT, LINUX_REBOOT_MAGIC1, LINUX_REBOOT_MAGIC2, LINUX_REBOOT_CMD_RESTART, 0)
    };
    println!("reboot() -> {}", ret);
}

// This is a workaround to allow running when /proc is not mounted.
// (see https://github.com/rust-lang/rust/issues/22642)
#[start]
fn start(argc: isize, argv: *const *const u8) -> isize {
    unsafe { ::std::rt::args::init(argc, argv); }
    main();
    return 0;
}

fn do_move(src : &str, dest : &str) {
    match fs::rename(src, dest) {
        Ok(_) => { },
        Err(e) => {
            println!("Error moving {} -> {}; {:?}", src, dest, e);
            thread::sleep( Duration::new(3, 0) );
        }
    }
}

fn main() {
    // verify source directory
    let src_dir = Path::new("/new_root");
    if src_dir.exists() == false || src_dir.is_dir() == false {
        println!("Error: /new_root does not exist or is not a directory");
        return;
    }

    // remount the root device read-write
    remount_root(0);

    // deleting files that are mapped into memory (this program and its libraries) will prevent
    // the filesystem from being cleanly unmounted (or remounted read-only).  to avoid this, we
    // preserve these files across the reboot and allow them to be cleaned from /tmp later.
    do_move("/sbin/init",                       "/new_root/tmp/init");
    do_move("/lib",                             "/new_root/tmp/lib");
    do_move("/usr/lib/gcc/x86_64-pc-linux-gnu", "/new_root/tmp/lib_gcc");

    // unmount filesystems that the initrd might have mounted
    umount("proc");
    umount("run");
    umount("sys");

    // move the mounted devtmpfs out of the way
    fs::create_dir("old_dev").unwrap();
    move_mount("dev", "old_dev");

    // delete everything except the new_root, dev, and lost+found directories
    for entry in fs::read_dir("/").unwrap() {
        let entry = entry.unwrap();
        let e_path = entry.path();

        if ! (e_path.ends_with("new_root") || e_path.ends_with("old_dev") || e_path.ends_with("lost+found") ) {
            if e_path.is_dir() {
                println!("deleting dir  {}", &e_path.display());
                fs::remove_dir_all(&e_path).unwrap();
            } else if e_path.is_file() {
                println!("deleting file {}", &e_path.display());
                fs::remove_file(&e_path).unwrap();
            } else {
                panic!("{:?} is neither a file nor a directory", e_path)
            }
        }
    }

    // move contents of source directory into current directory
    for entry in fs::read_dir(src_dir).unwrap() {
        let entry = entry.unwrap();
        let e_path = entry.path();
        let src = e_path.clone();
        let dest = e_path.relative_from(src_dir).unwrap();

        println!("moving {} -> {}", src.display(), dest.display());
        fs::rename(src, dest).unwrap();
    }

    // remove empty source directory
    println!("deleting dir  {}", src_dir.display());
    fs::remove_dir(src_dir).unwrap();

    // remove old_dev directory
    move_mount("old_dev", "dev");
    fs::remove_dir("old_dev").unwrap();

    println!("remounting root read-only");
    remount_root(MS_RDONLY);

    println!("calling sync() and reboot()");
    sync();
    thread::sleep( Duration::new(3, 0) );
    reboot();
}

