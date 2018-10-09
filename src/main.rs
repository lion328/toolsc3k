extern crate byteorder;
extern crate clap;

pub mod error;
pub mod format;

use error::*;
use std::fs::{self, File};
use std::path::Path;
use std::fmt::Write as WF;
use std::io::Write as WI;
use clap::{App, Arg, SubCommand};

fn main() -> Result<()> {
    let matches = App::new("toolsc3k")
        .about("A tool for reading assets of SimCity 3000.")
        .subcommand(SubCommand::with_name("dump-ixf")
            .about("Dump SC3K IXF file (.sc3, .DAT, etc.)")
            .arg(Arg::with_name("skip-bad")
                .help("Skip bad record")
                .long("skip-bad")
                .short("b")
            )
            .arg(Arg::with_name("to-file")
                .help("Dump into binary files")
                .long("to-file")
                .short("t")
                .takes_value(true)
            )
            .arg(Arg::with_name("INPUT")
                .help("The file to dump")
                .takes_value(true)
                .required(true)
            )
        )
        .subcommand(SubCommand::with_name("dbpf-uncompress")
            .about("Uncompress a file with DBPF compression")
            .arg(Arg::with_name("INPUT")
                .help("The input file")
                .takes_value(true)
                .required(true)
            )
            .arg(Arg::with_name("OUTPUT")
                .help("The output path")
                .takes_value(true)
                .required(true)
            )
        )
        .get_matches();

    match matches.subcommand() {
        ("dump-ixf", Some(sub_m)) => dump_ixf(
            sub_m.value_of("INPUT").unwrap(),
            sub_m.is_present("skip-bad"),
            sub_m.value_of("to-file")
        )?,
        ("dbpf-uncompress", Some(sub_m)) => dbpf_uncompress(
            sub_m.value_of("INPUT").unwrap(),
            sub_m.value_of("OUTPUT").unwrap()
        )?,
        _ => println!("Unknown subcommand")
    }

    Ok(())
}

fn dump_ixf(filename: &str, skip_bad: bool, binary_dump: Option<&str>) -> Result<()> {
    let vec = fs::read(filename)?;
    let sc3k = format::IXFFile::parse(&vec, skip_bad)?;

    for (i, r) in sc3k.records.iter().enumerate() {
        if let Some(ref dump_dir) = binary_dump {
            let path = Path::new(dump_dir).join(format!("{:X?}_{:X?}_{:X?}.bin", r.type_id, r.group_id, r.instance_id));

            let mut file = File::create(path)?;
            file.write_all(r.body)?;

            continue;
        }

        let mut out = String::new();
        writeln!(out, "Record number: {}", i);
        writeln!(out, "Type ID: 0x{:X?}", r.type_id);
        writeln!(out, "Group ID: 0x{:X?}", r.group_id);
        writeln!(out, "Instance ID: 0x{:X?}", r.instance_id);
        
        println!("{}Body:\n{}\n", out, dump_hex(r.body));
    }

    Ok(())
}

fn dbpf_uncompress(input: &str, output: &str) -> Result<()> {
    let parsed = format::DBPFCompression::parse(&fs::read(input)?)?;

    parsed.instructions.iter().enumerate().for_each(|(i, insn)| println!("{}:\t{}\t{}\t{}\t{}",
        i, insn.append_offset, insn.append_len, insn.decoded_copy_offset, insn.decoded_copy_len));

    fs::write(output, parsed.uncompress()?)?;
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
