use error::*;

#[derive(Debug)]
pub struct Image {
    image_type: ImageType,
    data: Vec<u8>,
    width: usize,
    height: usize,
}

impl Image {

    pub fn new(image_type: ImageType, width: usize, height: usize, data: Vec<u8>) -> Result<Image> {
        if data.len() != width * height * 2 {
            return Err(Error::Image(
                format!(
                    "size of the game image data is not matched for an image with (width, height) = ({}, {})",
                    width, height
                )
            ));
        }

        Ok(Image {
            image_type: image_type,
            data: data,
            width: width,
            height: height,
        })
    }

    pub fn from_rgb8(raw: &[u8], width: usize, height: usize, image_type: ImageType) -> Result<Image> {
        if raw.len() % 3 != 0 {
            return Err(Error::Image("invalid raw RGB8 pixels (length % 3 != 0)".into()));
        }

        let px_count = raw.len() / 3;

        if width * height != px_count {
            return Err(Error::Image(format!(
                    "size of the raw RGB pixels data is not matched for an image with (width, height) = ({}, {})",
                    width, height
            )));
        }

        let mut buffer = Vec::with_capacity(px_count * 2);

        for i in 0..px_count {
            let r = raw[i * 3 + 0] as u16;
            let g = raw[i * 3 + 1] as u16;
            let b = raw[i * 3 + 2] as u16;

            let v = match image_type {
                ImageType::G1R5G5B5 => {
                    ((g & 0x04) << 13) | ((r & 0xF8) << 7) | ((g & 0xF8) << 2) | (b >> 3)
                },
                ImageType::R5G6B5 => {
                    ((r & 0xF8) << 8) | ((g & 0xFC) << 3) | (b >> 3)
                },
            };

            buffer.push((v & 0xFF) as u8);
            buffer.push((v >> 8) as u8);
        }

        Ok(Image {
            image_type: image_type,
            data: buffer,
            width: width,
            height: height,
        })
    }

    pub fn image_type(&self) -> ImageType {
        self.image_type
    }

    pub fn into_inner(self) -> Vec<u8> {
        self.data
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn to_rgb8(&self) -> Vec<u8> {
        assert!(self.data.len() % 2 == 0);

        let len_half = self.data.len() / 2;
        let mut buffer = vec![0u8; len_half * 3];

        for i in 0..len_half {
            let color = self.data[i * 2] as u16 | ((self.data[i * 2 + 1] as u16) << 8);

            // I actually want to move these conversion code to an ImageType function, but I will keep it here first
            // since I'm not sure how other formats behave.
            let (r, g, b) = match self.image_type {
                ImageType::G1R5G5B5 => (
                    ((color >> 10) & 0x1F) * 255 / 31,
                    (((color >> 4) & 0x3E) | (color >> 15)) * 255 / 63,
                    (color & 0x1F) * 255 / 31
                ),
                ImageType::R5G6B5 => (
                    (color >> 11) * 255 / 31,
                    ((color >> 5) & 0x3F) * 255 / 63,
                    (color & 0x1F) * 255 / 31
                ),
            };

            buffer[i * 3 + 0] = r as u8;
            buffer[i * 3 + 1] = g as u8;
            buffer[i * 3 + 2] = b as u8;
        }

        buffer
    }

    pub fn convert_to(self, image_type: ImageType) -> Image {
        if self.image_type == image_type {
            return self;
        }

        Image::from_rgb8(&self.to_rgb8(), self.width, self.height, image_type).unwrap()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImageType {
    G1R5G5B5,
    R5G6B5,
}

impl ImageType {

    pub fn as_game_value(self) -> u32 {
        match self {
            ImageType::G1R5G5B5 => 0x05,
            ImageType::R5G6B5 => 0x07,
        }
    }

    pub fn from_game_value(v: u32) -> Result<ImageType> {
        Ok(match v {
            0x05 => ImageType::G1R5G5B5,
            0x07 => ImageType::R5G6B5,
            _ => return Err(Error::Image(format!("unsupported image type: {}", v)))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DATA_0: [u8; 27] = [
        0xFF, 0x00, 0x00,
        0x00, 0xFF, 0x00,
        0x00, 0x00, 0xFF,

        0x00, 0x00, 0x00,
        0xFF, 0xFF, 0xFF,
        0xFF, 0xFF, 0x00,
            
        0x00, 0xFF, 0xFF,
        0xFF, 0x00, 0xFF,
        0x12, 0x34, 0x56,
    ];
    const DATA_0_WIDTH: usize = 3;
    const DATA_0_HEIGHT: usize = DATA_0_WIDTH;

    const DATA_0_R5G6B5: [u8; 18] = [
        0b000_00000, 0b11111_000,
        0b111_00000, 0b00000_111,
        0b000_11111, 0b00000_000,

        0, 0,
        0xFF, 0xFF,
        0b111_00000, 0b11111_111,

        0b111_11111, 0b00000_111,
        0b000_11111, 0b11111_000,
        0b101_01010, 0b00010_001,
    ];

    const DATA_0_G1R5G5B5: [u8; 18] = [
        0b000_00000, 0b0_11111_00,
        0b111_00000, 0b1_00000_11,
        0b000_11111, 0b0_00000_00,

        0, 0,
        0xFF, 0xFF,
        0b111_00000, 0b1_11111_11,

        0b111_11111, 0b1_00000_11,
        0b000_11111, 0b0_11111_00,
        0b110_01010, 0b1_00010_00,
    ];

    const DATA_0_CONVERSION_LOSS: [u8; 27] = [
        0xFF, 0x00, 0x00,
        0x00, 0xFF, 0x00,
        0x00, 0x00, 0xFF,

        0x00, 0x00, 0x00,
        0xFF, 0xFF, 0xFF,
        0xFF, 0xFF, 0x00,
            
        0x00, 0xFF, 0xFF,
        0xFF, 0x00, 0xFF,
        0x10, 0x34, 0x52,
    ];

    #[test]
    fn to_game_format() {
        let image = Image::from_rgb8(&DATA_0, DATA_0_WIDTH, DATA_0_HEIGHT, ImageType::R5G6B5).unwrap();

        assert_eq!(image.width(), DATA_0_WIDTH);
        assert_eq!(image.height(), DATA_0_HEIGHT);
        assert_eq!(image.into_inner(), DATA_0_R5G6B5);

        let image = Image::from_rgb8(&DATA_0, DATA_0_WIDTH, DATA_0_HEIGHT, ImageType::G1R5G5B5).unwrap();

        assert_eq!(image.width(), DATA_0_WIDTH);
        assert_eq!(image.height(), DATA_0_HEIGHT);
        assert_eq!(image.into_inner(), DATA_0_G1R5G5B5);
    }

    #[test]
    fn from_game_format() {
        let image = Image::new(ImageType::R5G6B5, DATA_0_WIDTH, DATA_0_HEIGHT, DATA_0_R5G6B5.to_vec()).unwrap();
        assert_eq!(image.width(), DATA_0_WIDTH);
        assert_eq!(image.height(), DATA_0_HEIGHT);
        assert_eq!(&image.to_rgb8(), &DATA_0_CONVERSION_LOSS);
        
        let image = Image::new(ImageType::G1R5G5B5, DATA_0_WIDTH, DATA_0_HEIGHT, DATA_0_G1R5G5B5.to_vec()).unwrap();
        assert_eq!(image.width(), DATA_0_WIDTH);
        assert_eq!(image.height(), DATA_0_HEIGHT);
        assert_eq!(&image.to_rgb8(), &DATA_0_CONVERSION_LOSS);
    }

    #[test]
    fn convert() {
        let image = Image::new(ImageType::R5G6B5, DATA_0_WIDTH, DATA_0_HEIGHT, DATA_0_R5G6B5.to_vec())
            .unwrap().convert_to(ImageType::R5G6B5);
        assert_eq!(image.into_inner(), &DATA_0_R5G6B5);

        let image = Image::new(ImageType::R5G6B5, DATA_0_WIDTH, DATA_0_HEIGHT, DATA_0_R5G6B5.to_vec())
            .unwrap().convert_to(ImageType::G1R5G5B5);
        assert_eq!(image.into_inner(), &DATA_0_G1R5G5B5);

        let image = Image::new(ImageType::G1R5G5B5, DATA_0_WIDTH, DATA_0_HEIGHT, DATA_0_G1R5G5B5.to_vec())
            .unwrap().convert_to(ImageType::R5G6B5);
        assert_eq!(image.into_inner(), &DATA_0_R5G6B5);
    }
}
