extern crate byteorder;
extern crate clap;

pub mod error;
pub mod format;

use error::*;
use std::fs;
use std::fmt::Write;
use clap::{App, Arg, SubCommand};

fn main() -> Result<()> {
    let matches = App::new("toolsc3k")
        .about("A tool for reading assets of SimCity 3000.")
        .subcommand(SubCommand::with_name("dump_ixf")
            .about("Dump SC3K IXF file (.sc3, .DAT, etc.)")
            .arg(Arg::with_name("skip-bad")
                .help("Skip bad record")
                .long("skip-bad")
                .short("b")
            )
            .arg(Arg::with_name("INPUT")
                .help("The file to dump")
                .takes_value(true)
                .required(true)
            )
        )
        .get_matches();

    match matches.subcommand() {
        ("dump_ixf", Some(sub_m)) => dump_ixf(
            sub_m.value_of("INPUT").unwrap(),
            sub_m.is_present("skip-bad")
        )?,
        _ => println!("Unknown subcommand")
    }

    Ok(())
}

fn dump_ixf(filename: &str, skip_bad: bool) -> Result<()> {
    let vec = fs::read(filename)?;
    let sc3k = format::IXFFile::parse(&vec, skip_bad)?;

    sc3k.records.iter().enumerate().for_each(|(i, r)| {
        let mut out = String::new();
        writeln!(out, "Record number: {}", i);
        writeln!(out, "Type ID: 0x{:X?}", r.type_id);
        writeln!(out, "Group ID: 0x{:X?}", r.group_id);
        writeln!(out, "Instance ID: 0x{:X?}", r.instance_id);
        
        println!("{}Body:\n{}\n", out, dump_hex(r.body));
    });

    Ok(())
}

fn dump_hex(data: &[u8]) -> String {
    let mut output = String::with_capacity((91 * (data.len() + 1) / 16) + 66);

    output.push_str("                 00 01 02 03 04 05 06 07  08 09 0A 0B 0C 0D 0E 0F\n");

    for (i, chunk) in data.chunks(16).enumerate() {
        let mut numeric = String::with_capacity(50);
        let mut ascii = String::with_capacity(16);

        for (i, &b) in chunk.iter().enumerate() {
            numeric.push_str(&format!("{:02X} ", b));

            if i == 7 {
                numeric.push(' ');
            }

            ascii.push(match b {
                0x20 ... 0x7e => b as char,
                _ => '.'
            });
        }

        write!(output, "{:016X} {:50}{}\n", i << 4, numeric, ascii).unwrap();
    }

    output
}
