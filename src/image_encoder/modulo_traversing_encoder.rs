use super::ImageEncoder;

pub struct ModuloTraversingEncoder;

// Original implementation - works only for square images
impl ImageEncoder for ModuloTraversingEncoder {
    fn get_next_pixel_pos(&self, (x, y): (u32, u32), (w, h): (u32, u32), index: usize) -> Option<(u32, u32)> {
        // if index / 8 >= (w * h) as usize {
        //     return None;
        // }

        let mut x_pos = x;
        let mut y_pos = y;
        if index % w as usize == 0 {
            x_pos += 1;
        }
        x_pos = (x_pos + 19) % w;
        y_pos = (y_pos + 29) % h;

        Some((x_pos, y_pos))
    }
}
