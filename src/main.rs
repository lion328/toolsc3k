extern crate byteorder;
extern crate clap;
extern crate image;

pub mod error;
pub mod format;

use error::*;
use std::fs::{self, File};
use std::path::Path;
use std::fmt::Write as WF;
use std::io::Write as WI;
use clap::{App, Arg, SubCommand, ArgMatches, AppSettings};

fn main() -> Result<()> {
    let matches = App::new("toolsc3k")
        .about("A tool for reading assets of SimCity 3000.")
        .subcommand(SubCommand::with_name("ixf")
            .about("Command for managing IXF file (i.e., *.sc3, *.DAT)")
            .subcommand(SubCommand::with_name("dump")
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
            .setting(AppSettings::SubcommandRequired)
        )
        .subcommand(SubCommand::with_name("refpack")
            .about("Command for managing files with RefPack compression")
            .subcommand(SubCommand::with_name("uncompress")
                .about("Uncompress a file with RefPack compression")
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
            .subcommand(SubCommand::with_name("compress")
                .about("Compress a file with RefPack compression")
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
            .setting(AppSettings::SubcommandRequired)
        )
        .subcommand(SubCommand::with_name("image")
            .about("Command for dealing with game image, such as savefile preview, etc.")
            .subcommand(SubCommand::with_name("to-png")
                .about("Convert game image to PNG format")
                .arg(Arg::with_name("INPUT")
                    .help("The input file")
                    .takes_value(true)
                    .required(true)
                )
                .arg(Arg::with_name("OUTPUT")
                    .help("The output file")
                    .takes_value(true)
                    .required(true)
                )
            )
            .setting(AppSettings::SubcommandRequired)
        )
        .setting(AppSettings::SubcommandRequired)
        .get_matches();

    match matches.subcommand() {
        ("ixf", Some(sub)) => ixf(sub)?,
        ("refpack", Some(sub)) => refpack(sub)?,
        ("image", Some(sub)) => image(sub)?,
        _ => println!("Unknown subcommand")
    }

    Ok(())
}

fn ixf(matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        ("dump", Some(sub_m)) => ixf_dump(
            sub_m.value_of("INPUT").unwrap(),
            sub_m.is_present("skip-bad"),
            sub_m.value_of("to-file")
        )?,
        _ => println!("Unknown subcommand")
    }

    Ok(())
}

fn ixf_dump(filename: &str, skip_bad: bool, binary_dump: Option<&str>) -> Result<()> {
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

fn refpack(matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        ("uncompress", Some(sub_m)) => refpack_uncompress(
            sub_m.value_of("INPUT").unwrap(),
            sub_m.value_of("OUTPUT").unwrap()
        )?,
        ("compress", Some(sub_m)) => refpack_compress(
            sub_m.value_of("INPUT").unwrap(),
            sub_m.value_of("OUTPUT").unwrap()
        )?,
        _ => println!("Unknown subcommand")
    }

    Ok(())
}

fn refpack_uncompress(input: &str, output: &str) -> Result<()> {
    /*let parsed = format::RefPackCompression::parse(&fs::read(input)?)?;

    parsed.instructions.iter().enumerate().for_each(|(i, insn)| println!("{}:\t{}\t{}\t{}\t{}",
        i, insn.append_offset, insn.append_len, insn.decoded_copy_offset, insn.decoded_copy_len));

    fs::write(output, parsed.uncompress()?)?;*/
    fs::write(output, format::RefPackCompression::uncompress_direct(&fs::read(input)?)?)?;
    Ok(())
}

fn refpack_compress(input: &str, output: &str) -> Result<()> {
    let data = fs::read(input)?;
    let compress = format::RefPackCompression::compress_bad(&data)?;
    assert_eq!(format::RefPackCompression::uncompress_direct(&compress)?, data);
    fs::write(output, compress)?;
    Ok(())
}

fn image(matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        ("to-png", Some(sub)) => image_to_png(
            sub.value_of("INPUT").unwrap(),
            sub.value_of("OUTPUT").unwrap()
        )?,
        _ => println!("Unknown subcommand")
    }

    Ok(())
}

fn image_to_png(input: &str, output: &str) -> Result<()> {
    let raw = fs::read(input)?;
    
    if raw.len() % 2 != 0 {
        return Err("Wrong format (the length cannot not divisible by 2)".into())
    }

    let len_half = raw.len() / 2;
    let width = (len_half as f64).sqrt() as usize; // It's not look good TBH.

    let mut buffer = vec![0u8; len_half * 3];

    for i in 0..len_half {
        let (a, b) = (raw[i * 2] as u32, raw[i * 2 + 1] as u32);

        buffer[i * 3 + 0] = (((a & 0b11111100) >> 2) * 0xFF / 0b111111) as u8;
        buffer[i * 3 + 1] = (((a & 0b00000011) << 3) | ((b & 0b11100000) >> 5) * 0xFF / 0b11111) as u8;
        buffer[i * 3 + 2] = ((a & 0b00011111) * 0xFF / 0b11111) as u8;
    }

    image::save_buffer(output, &buffer, width as u32, width as u32, image::ColorType::RGB(8))?;

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
