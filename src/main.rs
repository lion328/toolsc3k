extern crate byteorder;
extern crate clap;
extern crate image;

pub mod error;
pub mod format;

use error::*;
use std::fs::{self, File};
use std::path::Path;
use std::fmt::Write as WF;
use std::io::{Write as WI, BufReader};
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
            .subcommand(SubCommand::with_name("reconstruct")
                .about("Reconstruct IXF file from \"dump\" command")
                .arg(Arg::with_name("INPUT")
                    .help("The input directory")
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
            .arg(Arg::with_name("type")
                .help("Image format type")
                .long("type")
                .short("t")
                .takes_value(true)
            )
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
            .subcommand(SubCommand::with_name("from-png")
                .about("Convert PNG image to game image format")
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
        ("reconstruct", Some(sub)) => ixf_reconstruct(
            sub.value_of("INPUT").unwrap(),
            sub.value_of("OUTPUT").unwrap()
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
            file.write_all(&r.body)?;

            continue;
        }

        let mut out = String::new();
        writeln!(out, "Record number: {}", i);
        writeln!(out, "Type ID: 0x{:X?}", r.type_id);
        writeln!(out, "Group ID: 0x{:X?}", r.group_id);
        writeln!(out, "Instance ID: 0x{:X?}", r.instance_id);
        
        println!("{}Body:\n{}\n", out, dump_hex(&r.body));
    }

    Ok(())
}

fn ixf_reconstruct(input: &str, output: &str) -> Result<()> {
    use std::ffi::OsStr;

    let mut ixf = format::IXFFile {
        records: Vec::new()
    };

    for entry in fs::read_dir(input)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension() != Some(OsStr::new("bin")) {
            continue;
        }

        let info = path.file_stem()
            .ok_or(Error::IXFFile("reconstruct: wrong file name format (file_stem)".into()))?
            .to_str()
            .ok_or(Error::from("cannot convert &OsStr to &Str"))?
            .split('_')
            .collect::<Vec<&str>>();

        if info.len() != 3 {
            println!("Wrong file name format for \"{:?}\", Skipped", path.file_name());
            continue;
        }

        ixf.records.push(format::IXFRecord {
            type_id: u32::from_str_radix(info[0], 16).map_err(|x| Error::OtherError(Box::new(x)))?,
            group_id: u32::from_str_radix(info[1], 16).map_err(|x| Error::OtherError(Box::new(x)))?,
            instance_id: u32::from_str_radix(info[2], 16).map_err(|x| Error::OtherError(Box::new(x)))?,
            body: fs::read(&path)?,
        })
    }

    fs::write(output, ixf.as_vec()?)?;

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
    let format = matches.value_of("type")
        .map(|s| format::ImageType::from_game_value(s.parse::<u32>().unwrap()).unwrap())
        .unwrap_or(format::ImageType::R5G6B5);

    match matches.subcommand() {
        ("to-png", Some(sub)) => image_to_png(
            sub.value_of("INPUT").unwrap(),
            sub.value_of("OUTPUT").unwrap(),
            format
        )?,
        ("from-png", Some(sub)) => image_from_png(
            sub.value_of("INPUT").unwrap(),
            sub.value_of("OUTPUT").unwrap(),
            format
        )?,
        _ => println!("Unknown subcommand")
    }

    Ok(())
}

fn image_to_png(input: &str, output: &str, image_type: format::ImageType) -> Result<()> {
    let raw = fs::read(input)?;
    let image = format::Image::new(image_type, raw)?;

    image::save_buffer(
        output,
        &image.to_rgb8(),
        image.width() as u32,
        image.width() as u32,
        image::ColorType::RGB(8)
    )?;

    Ok(())
}

fn image_from_png(input: &str, output: &str, image_type: format::ImageType) -> Result<()> {
    fs::write(
        output,
        &format::Image::from_rgb8(
            &image::load(BufReader::new(File::open(input)?), image::ImageFormat::PNG)
                .map_err(|x| Error::OtherError(Box::new(x)))?
                .to_rgb()
                .into_raw(),
            image_type
        )?.into_inner()
    )?;

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
