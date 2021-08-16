use rand::prelude::SliceRandom;
use rand_pcg::Pcg64;
use rand::SeedableRng;

use super::ImageEncoder;

pub struct RandomTraversingEncoder {
    indices: Vec<usize>
}

impl RandomTraversingEncoder {
    pub fn new(pixels_amount: usize, seed: u64) -> Self {
        let mut indices: Vec<usize> = (0..pixels_amount).collect();
        indices.shuffle(&mut Pcg64::seed_from_u64(seed));

        RandomTraversingEncoder {
            indices,
        }
    }
}

impl ImageEncoder for RandomTraversingEncoder {
    fn get_next_pixel_pos(&self, _: (u32, u32), (w, _): (u32, u32), index: usize) -> Option<(u32, u32)> {
        self.indices.get(index).map(|i| (*i as u32 % w, *i as u32 / w))
    }
}
