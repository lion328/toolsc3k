use std::io::{self, Read};
use error::*;
use byteorder::{ReadBytesExt, WriteBytesExt, LE};

#[derive(Debug, PartialEq)]
pub struct PAKFile {
    pub records: Vec<PAKRecord>,
}

#[derive(Debug, PartialEq)]
pub struct PAKRecord {
    pub name: String,
    pub lines: Vec<String>,
}

impl PAKFile {

    pub fn parse(data: &[u8]) -> Result<PAKFile> {
        let mut stream = io::Cursor::new(data);
        let records_len = stream.read_u32::<LE>()? as usize;
        let mut records = Vec::with_capacity(records_len);

        for _ in 0..records_len {
            let name = Self::read_string(&mut stream)?;
            let offset = stream.read_u32::<LE>()? as usize;

            if offset >= data.len() {
                return Err(Error::PAKFile(format!("offset out of bounds: 0x{:X?}", offset)));
            }

            let prev_pos = stream.position();

            stream.set_position(offset as u64);

            let lines_len = stream.read_u32::<LE>()? as usize;
            let mut lines = Vec::with_capacity(lines_len);

            for _ in 0..lines_len {
                lines.push(Self::read_string(&mut stream)?);
            }

            stream.set_position(prev_pos);

            records.push(PAKRecord {
                name: name,
                lines: lines,
            });
        }

        Ok(PAKFile {
            records: records,
        })
    }

    fn read_string(stream: &mut Read) -> Result<String> {
        let len = stream.read_u32::<LE>()? as usize;
        let mut buf = vec![0u8; len];
        stream.read_exact(buf.as_mut_slice())?;

        // TODO: Find proper encoding (likely an extended ASCII).
        // Use UTF-8 for now (at least it works).
        Ok(String::from_utf8_lossy(&buf).into())
    }
}

impl PAKRecord {

    pub fn as_single_string(&self) -> String {
        self.lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    lazy_static! {
        static ref DATA_0_STRUCT: PAKFile = PAKFile {
            records: vec![
                PAKRecord {
                    name: "Hello there".into(),
                    lines: vec![
                        "General Kenobi!".into(),
                    ],
                },
                PAKRecord {
                    name: "Rewrite it in Rust".into(),
                    lines: vec![],
                },
                PAKRecord {
                    name: "se tonight".into(),
                    lines: vec![
                        "party rockers in the hou".into(),
                        "everybody just ha".into(),
                        "ve a good time".into(),
                    ],
                },
            ]
        };
    }

    const DATA_0: &[u8] = b"\
        \x03\x00\x00\x00\
            \x0B\x00\x00\x00\
                Hello there\
            \x43\x00\x00\x00\
            \
            \x12\x00\x00\x00\
                Rewrite it in Rust\
            \x5A\x00\x00\x00\
            \
            \x0A\x00\x00\x00\
                se tonight\
            \x5E\x00\x00\x00\
        \
        \x01\x00\x00\x00\
            \x0F\x00\x00\x00\
                General Kenobi!\
        \
        \x00\x00\x00\x00\
        \
        \x03\x00\x00\x00\
            \x18\x00\x00\x00\
                party rockers in the hou\
            \x11\x00\x00\x00\
                everybody just ha\
            \x0E\x00\x00\x00\
                ve a good time\
    ";

    #[test]
    fn parse() {
        assert_eq!(PAKFile::parse(DATA_0).unwrap(), *DATA_0_STRUCT);
    }

    #[test]
    fn as_single_string() {
        assert_eq!(DATA_0_STRUCT.records[0].as_single_string(), "General Kenobi!");
        assert_eq!(DATA_0_STRUCT.records[1].as_single_string(), "");
        assert_eq!(DATA_0_STRUCT.records[2].as_single_string(), "party rockers in the hou\neverybody just ha\n\
            ve a good time");
    }
}
