use std::io::{self, Read};
use error::*;
use byteorder::{ReadBytesExt, LittleEndian as LE};

pub static SC3K_FILE_HEADER_IDENTIFIER: &[u8] = &[0xD7, 0x81, 0xC3, 0x80];

#[derive(Debug)]
pub struct IXFFile<'a> {
    pub records: Vec<IXFRecord<'a>>,
}

#[derive(Debug)]
pub struct IXFRecord<'a> {
    pub type_id: u32,
    pub group_id: u32,
    pub instance_id: u32,
    pub body: &'a [u8],
}

impl<'a> IXFFile<'a> {

    pub fn parse(data: &[u8], skip_bad: bool) -> Result<IXFFile> {
        let mut ident = [0u8; 4];
        let mut stream = io::Cursor::new(data);
        stream.read_exact(&mut ident)?;

        if ident != SC3K_FILE_HEADER_IDENTIFIER {
            return Err(Error::SC3KFile(format!("invalid header: {:x?}", ident)));
        }

        let mut records = Vec::new();

        loop {
            let type_id = stream.read_u32::<LE>()?;
            let group_id = stream.read_u32::<LE>()?;
            let instance_id = stream.read_u32::<LE>()?;

            if type_id == 0 && group_id == 0 && instance_id == 0 {
                break
            }

            let address = stream.read_u32::<LE>()? as usize;
            let length = stream.read_u32::<LE>()? as usize;

            if address >= data.len() || address > data.len() - length {
                if skip_bad {
                    continue
                }

                return Err(Error::SC3KFile(
                    format!("record out of bounds: address 0x{:X?}, length 0x{:X?}, max: 0x{:X?}", address, length,
                        data.len() - 1)));
            }

            records.push(IXFRecord {
                type_id: type_id,
                group_id: group_id,
                instance_id: instance_id,
                body: &data[address..address + length],
            });
        }

        Ok(IXFFile {
            records: records,
        })
    }

    pub fn as_vec(&self) -> Vec<u8> {
        unimplemented!()
    }
}
