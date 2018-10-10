use error::*;
use std::io::{Cursor, Write};
use byteorder::{ReadBytesExt, WriteBytesExt, BE};

pub const REFPACK_COMPRESSION_ID: u16 = 0x10FB;

#[derive(Debug)]
pub struct RefPackCompression {
    pub data: Vec<u8>,
    pub instructions: Vec<RefPackCompressionInstruction>,
    pub uncompressed_len: usize,
}

#[derive(Debug)]
pub struct RefPackCompressionInstruction {
    pub append_offset: usize,
    pub append_len: usize,
    pub decoded_copy_offset: usize,
    pub decoded_copy_len: usize,
}

/** References:
 * http://www.wiki.sc4devotion.com/index.php?title=QFS_compression
 * http://wiki.niotso.org/RefPack
 */

impl RefPackCompression {

    pub fn parse(data: &[u8]) -> Result<RefPackCompression> {
        let mut cursor = Cursor::new(data);
        let ident = cursor.read_u16::<BE>()?;

        if ident != REFPACK_COMPRESSION_ID {
            return Err(Error::RefPackCompression(format!("invalid identifier: 0x{:04X?}", ident)))
        }

        let mut ret = RefPackCompression {
            data: data.to_vec(),
            instructions: Vec::new(),
            uncompressed_len: cursor.read_u24::<BE>()? as usize,
        };

        let mut i = 9;
        while i < data.len() {
            let control_0 = cursor.read_u8()? as usize;
            let insn;

            print!("{:X?} ", control_0);

            match control_0 {
                0x00 ... 0x7F => {
                    let control_1 = cursor.read_u8()? as usize;
                    i += 2;

                    insn = RefPackCompressionInstruction {
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

                    insn = RefPackCompressionInstruction {
                        append_offset: i,
                        append_len: ((control_1 & 0xC0) >> 6) & 0x03,
                        decoded_copy_offset: ((control_1 & 0x3F) << 8) + control_2 + 1,
                        decoded_copy_len: (control_0 & 0x3F) + 4,
                    };
                },
                0xC0 ... 0xDF => {
                    // TODO: determine what format SC3K uses, the same as The Sims 2 or SC4?
                    let control_1 = cursor.read_u8()? as usize;
                    let control_2 = cursor.read_u8()? as usize;
                    let control_3 = cursor.read_u8()? as usize;
                    i += 4;

                    // TS2
                    /*insn = RefPackCompressionInstruction {
                        append_offset: i,
                        append_len: control_0 & 0x03,
                        decoded_copy_offset: ((control_0 & 0x10) << 12) + (control_1 << 8 ) + control_2 + 1,
                        decoded_copy_len: ((control_0 & 0x0C) << 6)  + control_3 + 5,
                    };*/

                    // SC4?
                    insn = RefPackCompressionInstruction {
                        append_offset: i,
                        append_len: control_0 & 0x03,
                        decoded_copy_offset: (control_1 << 8) + control_2,
                        decoded_copy_len: ((control_0 & 0x1C) << 6)  + control_3 + 5,
                    };
                },
                0xE0 ... 0xFB => {
                    i += 1;
                    insn = RefPackCompressionInstruction {
                        append_offset: i,
                        append_len: ((control_0 & 0x1F) << 2) + 4,
                        decoded_copy_offset: 0,
                        decoded_copy_len: 0,
                    };
                },
                0xFC ... 0xFF => {
                    i += 1;
                    insn = RefPackCompressionInstruction {
                        append_offset: i,
                        append_len: control_0 & 0x03,
                        decoded_copy_offset: 0,
                        decoded_copy_len: 0,
                    };
                },
                _ => return Err(Error::RefPackCompression(format!("unknown control code: 0x{:X?}", control_0)))
            }

            if insn.append_len > 0 {
                cursor.set_position((insn.append_offset + insn.append_len) as u64);
                i += insn.append_len;
            }

            ret.instructions.push(insn);
        }
        println!();

        Ok(ret)
    }

    pub fn uncompress(&self) -> Result<Vec<u8>> {
        let mut decoded = Vec::new();

        for elem in self.instructions.iter() {
            if elem.append_len > 0 {
                decoded.extend_from_slice(&self.data[elem.append_offset..elem.append_offset + elem.append_len]);
            }

            if elem.decoded_copy_len <= 0 {
                continue
            }

            if elem.decoded_copy_offset >= decoded.len() {
                return Err(Error::RefPackCompression(format!("decompression start index out of bounds: len ({}) <= {}",
                    decoded.len(), elem.decoded_copy_offset)))
            }

            let start = decoded.len() - elem.decoded_copy_offset - 1;

            for i in start..start + elem.decoded_copy_len {
                let b = decoded[i];
                decoded.push(b);
            }
        }

        if decoded.len() != self.uncompressed_len {
            return Err(Error::RefPackCompression(format!("uncompressed length mismatched: {} != {}", decoded.len(),
                self.uncompressed_len)));
        }

        Ok(decoded)
    }

    pub fn uncompress_direct(data: &[u8]) -> Result<Vec<u8>> {
        let mut cursor = Cursor::new(data);
        let ident = cursor.read_u16::<BE>()?;

        if ident != REFPACK_COMPRESSION_ID {
            return Err(Error::RefPackCompression(format!("invalid identifier: 0x{:04X?}", ident)))
        }

        let uncompressed_len = cursor.read_u24::<BE>()? as usize;

        let mut decoded = Vec::new();
        let mut stop_command = false;

        while cursor.position() < data.len() as u64 && !stop_command {
            let b0 = cursor.read_u8()? as usize;
            let append_len;
            let copy_offset;
            let copy_len;

            match b0 {
                0x00 ... 0x7F => {
                    let b1 = cursor.read_u8()? as usize;

                    append_len = b0 & 0x03;
                    copy_offset = ((b0 & 0x60) << 3) + b1 + 1;
                    copy_len = ((b0 & 0x1C) >> 2) + 3;
                },
                0x80 ... 0xBF => {
                    let b1 = cursor.read_u8()? as usize;
                    let b2 = cursor.read_u8()? as usize;

                    append_len = ((b1 & 0xC0) >> 6) & 0x03;
                    copy_offset = ((b1 & 0x3F) << 8) + b2 + 1;
                    copy_len = (b0 & 0x3F) + 4;
                },
                0xC0 ... 0xDF => {
                    let b1 = cursor.read_u8()? as usize;
                    let b2 = cursor.read_u8()? as usize;
                    let b3 = cursor.read_u8()? as usize;

                    append_len = b0 & 0x03;
                    copy_offset = (b1 << 8) + b2;
                    copy_len = ((b0 & 0x1C) << 6) + b3 + 5;
                },
                0xE0 ... 0xFB => {
                    append_len = ((b0 & 0x1F) << 2) + 4;
                    copy_offset = 0;
                    copy_len = 0;
                },
                0xFC ... 0xFF => {
                    append_len = b0 & 0x03;
                    copy_offset = 0;
                    copy_len = 0;
                    stop_command = true;
                },
                _ => return Err(Error::RefPackCompression(format!("unknown control code: 0x{:X?}", b0)))
            }

            for _ in 0..append_len {
                decoded.push(cursor.read_u8()?);
            }

            if copy_len <= 0 {
                continue
            }

            if copy_offset >= decoded.len() {
                return Err(Error::RefPackCompression(format!("decompression start index out of bounds: len ({}) <= {}",
                    decoded.len(), copy_offset)))
            }

            let start = decoded.len() - copy_offset - 1;

            for i in start..start + copy_len {
                let b = decoded[i];
                decoded.push(b);
            }
        }

        if decoded.len() != uncompressed_len {
            return Err(Error::RefPackCompression(format!("uncompressed length mismatched: {} != {}", decoded.len(),
                uncompressed_len)));
        }

        Ok(decoded)
    }

    pub fn compress(data: &[u8]) -> RefPackCompression {
        unimplemented!()
    }

    pub fn compress_bad(data: &[u8]) -> Result<Vec<u8>> {
        let mut cursor = Cursor::new(Vec::new());

        cursor.write_u16::<BE>(REFPACK_COMPRESSION_ID)?;
        cursor.write_u24::<BE>(data.len() as u32)?;

        let mut remaining = data.len();

        while remaining >= 112 {
            let off = data.len() - remaining;
            cursor.write_u8(0xFB)?;
            cursor.write_all(&data[off..off + 112])?;
            remaining -= 112;
        }

        assert!(remaining < 112);

        let left_bits = ((remaining - 4) / 4) as u8;
        let left = left_bits as usize * 4 + 4;
        let off = data.len() - remaining;

        remaining -= left;

        cursor.write_u8(0b11100000u8 | left_bits)?;
        cursor.write_all(&data[off..off + left])?;

        assert!(remaining <= 3);

        cursor.write_u8(0b11111100u8 | remaining as u8)?;
        cursor.write_all(&data[data.len() - remaining..])?;

        Ok(cursor.into_inner())
    }
}
