use std::io::{self, Read, Write, Cursor};
use error::*;
use byteorder::{ReadBytesExt, WriteBytesExt, LE};

pub const IXF_FILE_HEADER_IDENTIFIER: &[u8] = &[0xD7, 0x81, 0xC3, 0x80];
pub const IXF_FILE_RECORD_LENGTH: usize = 20;
pub const IXF_FILE_NULL_RECORD: &[u8] = &[0u8; IXF_FILE_RECORD_LENGTH];

#[derive(Debug, PartialEq)]
pub struct IXFFile {
    pub records: Vec<IXFRecord>,
}

#[derive(Debug, PartialEq)]
pub struct IXFRecord {
    pub type_id: u32,
    pub group_id: u32,
    pub instance_id: u32,
    pub body: Vec<u8>,
}

impl IXFFile {

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
                body: data[address..address + length].to_vec(),
            });
        }

        Ok(IXFFile {
            records: records,
        })
    }

    pub fn as_vec(&self) -> Result<Vec<u8>> {
        let mut cursor = Cursor::new(Vec::new());

        cursor.write_all(IXF_FILE_HEADER_IDENTIFIER)?;

        let mut offset = IXF_FILE_HEADER_IDENTIFIER.len() + IXF_FILE_RECORD_LENGTH * (self.records.len() + 1);

        for elem in self.records.iter() {
            cursor.write_u32::<LE>(elem.type_id)?;
            cursor.write_u32::<LE>(elem.group_id)?;
            cursor.write_u32::<LE>(elem.instance_id)?;
            cursor.write_u32::<LE>(offset as u32)?;
            cursor.write_u32::<LE>(elem.body.len() as u32)?;

            offset += elem.body.len();
        }

        cursor.write_all(IXF_FILE_NULL_RECORD)?;

        for elem in self.records.iter() {
            cursor.write_all(&elem.body)?;
        }

        Ok(cursor.into_inner())
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

    #[test]
    fn reencode() {
        let data = [
            0xD7, 0x81, 0xC3, 0x80,
            0x12, 0x34, 0x56, 0x78,
            0x9A, 0xBC, 0xDE, 0xF0,
            0x29, 0x99, 0x79, 0x24,
            0x2C, 0x00, 0x00, 0x00,
            0x04, 0x00, 0x00, 0x00,

            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            // Yes, I missing this one. Still, they can be overlapped. I think the last 2*4 bytes of null record
            // can be removed.
            0x00, 0x00, 0x00, 0x00,

            0xBE, 0xEF, 0xCA, 0xCE,
        ];

        let parsed = IXFFile::parse(&data, false).unwrap();

        assert_eq!(parsed.as_vec().unwrap(), data.to_vec());
    }
}
