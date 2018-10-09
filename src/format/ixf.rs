use std::io::{self, Read};
use error::*;
use byteorder::{ReadBytesExt, LittleEndian as LE};

pub static IXF_FILE_HEADER_IDENTIFIER: &[u8] = &[0xD7, 0x81, 0xC3, 0x80];

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

        if ident != IXF_FILE_HEADER_IDENTIFIER {
            return Err(Error::IXFFile(format!("invalid header: {:x?}", ident)));
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

                return Err(Error::IXFFile(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn invalid_header() {
        let data = [0xDE, 0xAD, 0xBA, 0xBE, 0xFA, 0x11];
        IXFFile::parse(&data, true).unwrap();
    }

    #[test]
    fn normal() {
        let data = [
            0xD7, 0x81, 0xC3, 0x80,
            0x12, 0x34, 0x56, 0x78,
            0x9A, 0xBC, 0xDE, 0xF0,
            0x29, 0x99, 0x79, 0x24,
            0x28, 0x00, 0x00, 0x00,
            0x04, 0x00, 0x00, 0x00,

            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,

            0xBE, 0xEF, 0xCA, 0xCE,
        ];
        
        let record = {
            let mut records = IXFFile::parse(&data, false).unwrap().records;
            assert_eq!(records.len(), 1);
            records.pop().unwrap()
        };

        assert_eq!(record.type_id, 0x78563412);
        assert_eq!(record.group_id, 0xF0DEBC9A);
        assert_eq!(record.instance_id, 0x24799929);
        assert_eq!(record.body, &data[0x28..]);
    }

    #[test]
    #[should_panic]
    fn bad_record() {
        let data = [
            0xD7, 0x81, 0xC3, 0x80,
            0x12, 0x34, 0x56, 0x78,
            0x9A, 0xBC, 0xDE, 0xF0,
            0x29, 0x99, 0x79, 0x24,
            0x28, 0xFF, 0x00, 0x00,
            0x04, 0x00, 0x00, 0x00,

            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ];
        
        IXFFile::parse(&data, false).unwrap();
    }

    #[test]
    fn bad_record_ignore() {
        let data = [
            0xD7, 0x81, 0xC3, 0x80,
            0x12, 0x34, 0x56, 0x78,
            0x9A, 0xBC, 0xDE, 0xF0,
            0x29, 0x99, 0x79, 0x24,
            0x28, 0xFF, 0x00, 0x00,
            0x04, 0x00, 0x00, 0x00,

            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,

            0xBE, 0xEF, 0xCA, 0xCE,
        ];
        
        let records = IXFFile::parse(&data, true).unwrap().records;
        assert_eq!(records.len(), 0);
    }
}
