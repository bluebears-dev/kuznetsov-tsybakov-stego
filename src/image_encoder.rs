use bit_vec::BitVec;
use image::{GrayImage, ImageBuffer, Luma};

use crate::{
    encoder::StandardKTEncoder,
    kt_search_tree::{ByteEncodingCapacity, Config, SearchTree},
};

pub trait ImageEncoder {
    fn traversal_method(pos: (u32, u32), dimension: (u32, u32), index: u64) -> (u32, u32);

    fn encode(image: &GrayImage, message: &Vec<u8>, freedom_bit_count: u8) -> Vec<u8> {
        let probabilities = load_probabilities(image, 50, 50);
        let encoding_capacity = get_image_encoding_capacity(image);

        let mut search_tree = SearchTree::new(Config {
            probabilities,
            encoding_capacity,
            freedom_bit_count,
            encoding_method: Box::new(StandardKTEncoder::new(RNG_SEED)),
        });

        search_tree.find_best_encoding(message)
    }

    fn save_into_image((width, height): (u32, u32), data: &Vec<u8>) -> GrayImage {
        let mut new_image: GrayImage = ImageBuffer::new(width, height);
        let dimension = new_image.dimensions();
        let encoding_capacity = get_image_encoding_capacity(&new_image);

        let mut x_pos = 50;
        let mut y_pos = 50;

        let data_bit_vec = BitVec::from_bytes(&data);

        for i in 0..(width * height) as u64 {
            let (x, y) = Self::traversal_method((x_pos, y_pos), dimension, i);
            x_pos = x;
            y_pos = y;

            let maybe_pixel =
                data_bit_vec
                    .get(i as usize)
                    .map(|bit| if bit { Luma([255]) } else { Luma([0]) });

            if let Some(pixel) = maybe_pixel {
                new_image.put_pixel(x_pos, y_pos, pixel);
            } else {
                break;
            }
        }

        new_image
    }
}

const RNG_SEED: u64 = 5;

type ImageTraversalMethod = dyn Fn(u32, u32, u64, dyn Fn(u64) -> Luma<u8>) -> (u32, u32);

fn get_image_encoding_capacity(image: &GrayImage) -> ByteEncodingCapacity {
    let (width, height) = image.dimensions();
    ((width * height) as f32 / 8.0).floor() as ByteEncodingCapacity
}

fn load_probabilities(image: &GrayImage, start_x: u32, start_y: u32) -> Vec<f32> {
    let (width, height) = image.dimensions();
    let mut probabilities = vec![0.0; 10 + (width * height) as usize];

    let mut x_pos = start_x;
    let mut y_pos = start_y;
    for i in 0..(width * height) {
        // TODO: Why is this needed?
        if i % width == 0 {
            x_pos += 1;
        }
        x_pos = (x_pos + 19) % width;
        y_pos = (y_pos + 29) % height;
        let pixel = *image.get_pixel(x_pos, y_pos);
        // These will be later passed to logarithm - prevent log 0 by scaling
        probabilities[i as usize] = ((pixel.0[0] as f32) / 256.0 + 0.0001) * 0.9998;
    }
    probabilities
}

pub struct ModuloTraversingImageEncoder;

impl ImageEncoder for ModuloTraversingImageEncoder {
    fn traversal_method((x, y): (u32, u32), (w, h): (u32, u32), index: u64) -> (u32, u32) {
        // if i / 8 >= encoding_capacity {
        // break;
        // }
        // TODO: Why is this needed?
        let mut x_pos = x;
        let mut y_pos = y;
        if index % w as u64 == 0 {
            x_pos += 1;
        }
        x_pos = (x_pos + 19) % w;
        y_pos = (y_pos + 29) % h;

        (x_pos, y_pos)
    }
}
