pub mod modulo_traversing_encoder;
pub mod random_traversing_encoder;

use bitvec::prelude::*;
use image::{GrayImage, ImageBuffer, Luma};

use crate::encoder::ByteEncodingCapacity;

const BUCKET_SIZE: u8 = 2;

fn bucket_get_bit(pixel: &Luma<u8>) -> bool {
    (pixel.0[0] / BUCKET_SIZE) % 2 == 1
}

fn bucket_get_pixel(bit: bool, original_pixel: &Luma<u8>) -> Luma<u8> {
    let bucket_index = original_pixel.0[0] / BUCKET_SIZE;

    if bit != bucket_get_bit(original_pixel) {
        if bucket_index == 0 {
            Luma([BUCKET_SIZE])
        } else {
            Luma([(bucket_index * BUCKET_SIZE) - 1])
        }
    } else {
        Luma([(bucket_index * BUCKET_SIZE) + 1])
    }
}

pub trait ImageEncoder {
    fn get_next_pixel_pos(
        &self,
        pos: (u32, u32),
        dimension: (u32, u32),
        index: usize,
    ) -> Option<(u32, u32)>;

    fn decode_from_image(&self, image: &GrayImage) -> Vec<u8> {
        let mut x_pos = 50;
        let mut y_pos = 50;

        let dimensions = image.dimensions();

        let mut fetched_bits = bitvec![u8, Lsb0; 0; image.len() as usize];

        let mut encoded_bit_pos = 0;
        for i in 0..image.len() as usize {
            if let Some((x, y)) = self.get_next_pixel_pos((x_pos, y_pos), dimensions, i) {
                x_pos = x;
                y_pos = y;
            } else {
                break;
            }

            fetched_bits.set(
                encoded_bit_pos,
                bucket_get_bit(image.get_pixel(x_pos, y_pos)),
            );
            encoded_bit_pos += 1;
        }

        fetched_bits.into()
    }

    fn encode_into_image(&self, image: &GrayImage, data: &Vec<u8>) -> GrayImage {
        let mut new_image: GrayImage = ImageBuffer::new(image.width(), image.height());
        let dimensions = new_image.dimensions();

        let mut x_pos = 50;
        let mut y_pos = 50;

        let data_bit_vec = data.view_bits::<Lsb0>();

        for i in 0..image.len() as usize {
            if let Some((x, y)) = self.get_next_pixel_pos((x_pos, y_pos), dimensions, i) {
                x_pos = x;
                y_pos = y;
            } else {
                break;
            }

            let maybe_pixel = data_bit_vec
                .get(i as usize)
                .map(|bit| bucket_get_pixel(*bit, image.get_pixel(x_pos, y_pos)));

            if let Some(pixel) = maybe_pixel {
                new_image.put_pixel(x_pos, y_pos, pixel);
            }
        }

        new_image
    }

    fn get_probabilities(&self, image: &GrayImage, x_start: u32, y_start: u32) -> Vec<f32> {
        let dimensions = image.dimensions();
        let mut probabilities = vec![0.0; 10 + image.len() as usize];

        let mut x_pos = x_start;
        let mut y_pos = y_start;
        for i in 0..image.len() as usize {
            if let Some((x, y)) = self.get_next_pixel_pos((x_pos, y_pos), dimensions, i) {
                x_pos = x;
                y_pos = y;
            } else {
                break;
            }

            let pixel = image.get_pixel(x_pos, y_pos);
            // Prevent log 0 by scaling the value
            probabilities[i as usize] = ((pixel.0[0] as f32) / 256.0 + 0.0001) * 0.9998;
        }
        probabilities
    }
}

pub fn get_image_encoding_capacity(image: &GrayImage) -> ByteEncodingCapacity {
    let (width, height) = image.dimensions();
    ((width * height) as f32 / 8.0).floor() as ByteEncodingCapacity
}
