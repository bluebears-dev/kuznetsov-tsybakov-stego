use super::ImageEncoder;

pub struct ModuloTraversingEncoder;

impl ImageEncoder for ModuloTraversingEncoder {
    fn get_next_pixel_pos((x, y): (u32, u32), (w, h): (u32, u32), index: u64) -> Option<(u32, u32)> {
        if index / 8 >= (w * h) as u64 {
            return None;
        }
        // TODO: Why is this needed?
        let mut x_pos = x;
        let mut y_pos = y;
        if index % w as u64 == 0 {
            x_pos += 1;
        }
        x_pos = (x_pos + 19) % w;
        y_pos = (y_pos + 29) % h;

        Some((x_pos, y_pos))
    }
}
