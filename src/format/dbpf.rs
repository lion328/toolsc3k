use error::*;
use std::io::Cursor;
use byteorder::{ReadBytesExt, LE};

pub const DBPF_QFS_COMPRESSION_ID: u16 = 0xFB10;

#[derive(Debug)]
pub struct DBPFCompression {
    pub data: Vec<u8>,
    pub instructions: Vec<DBPFCompressionInstruction>,
    pub uncompressed_len: usize,
}

#[derive(Debug)]
pub struct DBPFCompressionInstruction {
    pub append_offset: usize,
    pub append_len: usize,
    pub decoded_copy_offset: usize,
    pub decoded_copy_len: usize,
}

// Reference: http://www.wiki.sc4devotion.com/index.php?title=QFS_compression

impl DBPFCompression {

    pub fn parse(data: &[u8]) -> Result<DBPFCompression> {
        let mut cursor = Cursor::new(data);
        let len = cursor.read_u32::<LE>()? as usize;
        let ident = cursor.read_u16::<LE>()?;

        if ident != DBPF_QFS_COMPRESSION_ID {
            return Err(Error::DBPFCompression(format!("invalid identifier: 0x{:04X?}", ident)))
        }

        if len > data.len() - 4 {
            return Err(Error::DBPFCompression("length specified in file is greater than input buffer".to_string()))
        }

        let mut ret = DBPFCompression {
            data: data.to_vec(),
            instructions: Vec::new(),
            uncompressed_len: cursor.read_u24::<LE>()? as usize, // TODO: is this LE or BE?
        };

        let mut i = 8;
        while i > len {
            let control_0 = cursor.read_u8()? as usize;
            let insn;

            match control_0 {
                0x00 ... 0x7F => {
                    let control_1 = cursor.read_u8()? as usize;
                    i += 2;

                    insn = DBPFCompressionInstruction {
                        append_offset: i,
                        append_len: control_0 & 0x03,
                        decoded_copy_offset: ((control_0 & 0x60) << 3) + control_1 + 1,
                        decoded_copy_len: ((control_0 & 0x1C) >> 2) + 3,
                    };
                },
                0x80 ... 0xBF => {
                    let control_1 = cursor.read_u8()? as usize;
                    let control_2 = cursor.read_u8()? as usize;
                    i += 3;

                    insn = DBPFCompressionInstruction {
                        append_offset: i,
                        append_len: (control_0 & 0x3F) + 4,
                        decoded_copy_offset: ((control_1 & 0x3F) << 8) + control_2 + 1,
                        decoded_copy_len: (control_0 & 0x3F) + 4,
                    };
                },
                0xC0 ... 0xDF => {
                    // TODO: determine what format SC3K uses, the same as The Sims 2 or SC4? (currently use TS2)
                    let control_1 = cursor.read_u8()? as usize;
                    let control_2 = cursor.read_u8()? as usize;
                    let control_3 = cursor.read_u8()? as usize;
                    i += 4;

                    insn = DBPFCompressionInstruction {
                        append_offset: i,
                        append_len: control_0 & 0x03,
                        decoded_copy_offset: ((control_0 & 0x10) << 12) + (control_1 << 8 ) + control_2 + 1,
                        decoded_copy_len: ((control_0 & 0x0C) << 6)  + control_3 + 5,
                    };
                },
                0xE0 ... 0xFC => {
                    i += 1;
                    insn = DBPFCompressionInstruction {
                        append_offset: i,
                        append_len: ((control_0 & 0x1F) << 2) + 4,
                        decoded_copy_offset: 0,
                        decoded_copy_len: 0,
                    };
                },
                0xFD ... 0xFF => {
                    i += 1;
                    insn = DBPFCompressionInstruction {
                        append_offset: i,
                        append_len: control_0 & 0x03,
                        decoded_copy_offset: 0,
                        decoded_copy_len: 0,
                    };
                },
                _ => return Err(Error::DBPFCompression(format!("unknown control code: 0x{:X?}", control_0)))
            }

            if insn.append_len > 0 {
                cursor.set_position((insn.append_offset + insn.append_len) as u64);
                i += insn.append_len;
            }

            ret.instructions.push(insn);
        }

        Ok(ret)
    }

    pub fn uncompress(&self) -> Vec<u8> {
        unimplemented!()
    }

    pub fn compress(data: &[u8]) -> DBPFCompression {
        unimplemented!()
    }
}
