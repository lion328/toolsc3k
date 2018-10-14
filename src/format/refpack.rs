use error::*;
use std::io::{Cursor, Write};
use byteorder::{ReadBytesExt, WriteBytesExt, BE};

pub const REFPACK_COMPRESSION_ID: u16 = 0x10FB;

pub struct RefPackCompression;

/** References:
 * http://www.wiki.sc4devotion.com/index.php?title=QFS_compression
 * http://wiki.niotso.org/RefPack
 */

impl RefPackCompression {

    pub fn uncompress(data: &[u8]) -> Result<Vec<u8>> {
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
                0xC0 ... 0xCF => {
                    let b1 = cursor.read_u8()? as usize;
                    let b2 = cursor.read_u8()? as usize;
                    let b3 = cursor.read_u8()? as usize;

                    append_len = b0 & 0x03;
                    copy_offset = (b1 << 8) + b2 + 1;
                    copy_len = ((b0 & 0x1C) << 6) + b3 + 5;
                },
                0xD0 ... 0xDF => {
                    let b1 = cursor.read_u8()? as usize;
                    let b2 = cursor.read_u8()? as usize;
                    let b3 = cursor.read_u8()? as usize;

                    append_len = b0 & 0x03;
                    copy_offset = ((b0 & 0x10) << 12) + (b1 << 8) + b2 + 1;
                    copy_len = ((b0 & 0x0C) << 6) + b3 + 5;
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

            if copy_offset > decoded.len() {
                return Err(Error::RefPackCompression(format!("decompression start index out of bounds: len ({}) < {}",
                    decoded.len(), copy_offset)))
            }

            let start = decoded.len() - copy_offset;

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

    pub fn compress(data: &[u8]) -> Result<Vec<u8>> {
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
