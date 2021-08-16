pub mod modulo_traversing_encoder;

use bitvec::prelude::*;
use image::{GrayImage, ImageBuffer, Luma};

use crate::encoder::ByteEncodingCapacity;

pub trait ImageEncoder {
    fn get_next_pixel_pos(pos: (u32, u32), dimension: (u32, u32), index: u64)
        -> Option<(u32, u32)>;

    fn decode_from_image(image: &GrayImage) -> Vec<u8> {
        let mut x_pos = 50;
        let mut y_pos = 50;

        let dimensions = image.dimensions();
        let (width, height) = dimensions;

        let encoding_capacity = get_image_encoding_capacity(&image);

        let mut fetched_bits = bitvec![Lsb0, u8; 0; (encoding_capacity * 8) as usize];

        let mut encoded_bit_pos = 0;
        for i in 0..(width * height) as u64 {
            if let Some((x, y)) = Self::get_next_pixel_pos((x_pos, y_pos), dimensions, i) {
                x_pos = x;
                y_pos = y;

                if image.get_pixel(x_pos, y_pos).0[0] > 128 {
                    fetched_bits.set(encoded_bit_pos, true);
                }
                encoded_bit_pos += 1;
            } else {
                break;
            }
        }

        fetched_bits.into()
    }

    fn encode_into_image((width, height): (u32, u32), data: &Vec<u8>) -> GrayImage {
        let mut new_image: GrayImage = ImageBuffer::new(width, height);
        let dimension = new_image.dimensions();

        let mut x_pos = 50;
        let mut y_pos = 50;

        let data_bit_vec = data.view_bits::<Lsb0>();

        for i in 0..(width * height) as u64 {
            if let Some((x, y)) = Self::get_next_pixel_pos((x_pos, y_pos), dimension, i) {
                x_pos = x;
                y_pos = y;

                let maybe_pixel =
                    data_bit_vec
                        .get(i as usize)
                        .map(|bit| if *bit { Luma([255]) } else { Luma([0]) });

                if let Some(pixel) = maybe_pixel {
                    new_image.put_pixel(x_pos, y_pos, pixel);
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        new_image
    }

    fn get_probabilities(image: &GrayImage, x_start: u32, y_start: u32) -> Vec<f32> {
        let dimensions = image.dimensions();
        let (width, height) = dimensions;
        let mut probabilities = vec![0.0; 10 + (width * height) as usize];

        let mut x_pos = x_start;
        let mut y_pos = y_start;
        for i in 0..(width * height) as u64 {
            if let Some((x, y)) = Self::get_next_pixel_pos((x_pos, y_pos), dimensions, i) {
                x_pos = x;
                y_pos = y;

                let pixel = *image.get_pixel(x_pos, y_pos);
                // Prevent log 0 by scaling the value
                probabilities[i as usize] = ((pixel.0[0] as f32) / 256.0 + 0.0001) * 0.9998;
            } else {
                break;
            }
        }
        probabilities
    }
}

pub fn get_image_encoding_capacity(image: &GrayImage) -> ByteEncodingCapacity {
    let (width, height) = image.dimensions();
    ((width * height) as f32 / 8.0).floor() as ByteEncodingCapacity
}
