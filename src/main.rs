mod encoder;
mod image_encoder;

extern crate image;

use std::{
    error::Error,
    fs::{self, File},
};

use encoder::{KTEncoder, StandardKTEncoder};
use image::{io::Reader as ImageReader, GenericImageView};

use image_encoder::ImageEncoder;

use crate::image_encoder::{
    get_image_encoding_capacity, modulo_traversing_encoder::ModuloTraversingEncoder,
    random_traversing_encoder::RandomTraversingEncoder,
};

fn main() -> Result<(), Box<dyn Error>> {
    let image = ImageReader::open("borzoi.jpg")?.decode()?;
    let text = fs::read_to_string("data")?;
    let gray_image = &image.to_luma8();
    let freedom_bit_count = 2;
    let encoding_capacity = get_image_encoding_capacity(gray_image);
    println!("Image and data loaded");
    // let image_encoder = RandomTraversingEncoder::new((encoding_capacity * 8) as usize, 13371);
    let image_encoder = RandomTraversingEncoder::new(gray_image.len(), 10);
    let probabilities = image_encoder.get_probabilities(gray_image, 50, 50);
    let coder = StandardKTEncoder::new(5);
    println!("Encoder ready");

    let data = coder.encode(
        &text.as_bytes().to_vec(),
        &probabilities,
        encoding_capacity,
        freedom_bit_count,
    );
    println!("KT encoding found");

    let encoded_image = image_encoder.encode_into_image(gray_image, &data);
    println!("Encoded into image");

    encoded_image.save("code.bmp").unwrap();
    println!("Saved the image");

    let image_with_data = ImageReader::open("code.bmp")?.decode()?.to_luma8();
    println!("Encoded image loaded");

    let raw_data = image_encoder.decode_from_image(&image_with_data);
    let decoded_message = coder.decode(&raw_data, encoding_capacity, freedom_bit_count);
    println!("Decoded the data from the image");

    let message = String::from_utf8_lossy(&decoded_message).to_string();
    fs::write("decoded-data", &message.trim_end_matches(char::from(0)))?;
    println!("Decoded data saved");

    Ok(())
}
