pub mod binary;
mod encoder;
mod kt_search_tree;

extern crate image;

use std::{error::Error, u32};

use bit_vec::BitVec;
use encoder::StandardKTEncoder;
use image::{io::Reader as ImageReader, GenericImageView, GrayImage, ImageBuffer, Luma};
use kt_search_tree::{Config, SearchTree};

use crate::encoder::KTEncoder;

const RNG_SEED: u64 = 5;

fn get_image_bytes_encoding_size(image: &GrayImage) -> u64 {
    let (width, height) = image.dimensions();
    ((width * height) as f32 / 8.0).floor() as u64
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

fn encode(image: &GrayImage, message: &Vec<u8>, freedom_bit_count: u8) -> Vec<u8> {
    let probabilities = load_probabilities(image, 50, 50);
    let encoding_capacity = get_image_bytes_encoding_size(image);

    let mut search_tree = SearchTree::new(Config {
        probabilities,
        encoding_capacity,
        freedom_bit_count,
        encoding_method: Box::new(StandardKTEncoder::new(RNG_SEED)),
    });

    search_tree.find_best_encoding(message)
}

fn decode(image: &GrayImage, freedom_bit_count: u8) -> Vec<u8> {
    let mut state: u64 = 0;

    let decoder = StandardKTEncoder::new(RNG_SEED);

    let mut x_pos = 50;
    let mut y_pos = 50;
    let mut current_bit = 0;
    let (width, height) = image.dimensions();
    let encoding_capacity = get_image_bytes_encoding_size(&image);

    let mut decoded = BitVec::from_elem((encoding_capacity * 8) as usize, false);

    let mut byte_bit_vec = BitVec::from_elem(8, false);
    let mut encoded_bit_pos = 0;
    for i in 0..(width * height) {
        // TODO: Why is this needed?
        if i % width == 0 {
            x_pos += 1;
        }
        x_pos = (x_pos + 19) % width;
        y_pos = (y_pos + 29) % height;

        if image.get_pixel(x_pos, y_pos).0[0] > 128 {
            byte_bit_vec.set(encoded_bit_pos, true);
        }

        encoded_bit_pos += 1;

        if encoded_bit_pos == 8 {
            let byte = byte_bit_vec.to_bytes()[0];
            let (decoded_byte, new_state) = decoder.decode(byte, state);
            state = new_state;

            byte_bit_vec.set_all();
            byte_bit_vec.negate();

            encoded_bit_pos = 0;

            // TODO: Write remaining bits too
            let decoded_bit_vec = BitVec::from_bytes(&[decoded_byte]);
            for i in 0..8 - freedom_bit_count {
                decoded.set(current_bit, decoded_bit_vec.get(i.into()).unwrap());
                current_bit += 1;
            }
        };
    }

    decoded.to_bytes()
}

fn main() -> Result<(), Box<dyn Error>> {
    let image = ImageReader::open("picture.bmp")?.decode()?;
    let text = r#"twoja stara
    Imagine a basic situation: we have a source of symbols of known probability distribution and we would like to design an entropy coder transforming it into a bit sequence, which would be simple and very close to the capacity (Shannon entropy). Prefix codes are the basic method, defining "symbol$\rightarrow$bit sequence" set of rules, usually found using Huffman algorithm. They theoretically allow to reduce the distance from the capacity ($\Delta H$) down to zero, but the cost grows rapidly. We will discuss improving it by replacing this memoryless coder with an automate having some small set of internal states: defined by "(symbol, state)$\rightarrow$(bit sequence, new state)" set of rules. The natural question is the minimal number of states to achieve given performance, what is especially important for simple high throughput hardware coders. Arithmetic coding can be seen this way, but it requires relatively large number of states (possible ranges). We will discuss asymmetric numeral systems (ANS) for this purpose, which can be seen as asymmetrization of numeral systems. Less than 20 states will be usually sufficient to achieve $\Delta H\approx 0.001$ bits/symbol for a small alphabet. $\Delta H$ generally decreases approximately like 1/(the number of states$)^{2}$ and increases proportionally to the size of alphabet. Huge freedom of choosing the exact coding and chaotic behavior of state make it also perfect to simultaneously encrypt the data.
\end{abstract}
\section{Introduction}
Electronics we use is usually based on the binary numeral system, which is perfect for handling integer number of bits of information. However, generally event/symbol of probability $p$ contains $\lg(1/p)$ bits of information ($\lg\equiv \log_2$), which is not necessarily integer in real applications. If all probabilities would be integer powers of $3$ instead, we could optimally use base 3 numeral system and analogously for larger bases. In more complex situations we need to use more sophisticated methods: entropy coders, translating between symbol sequence of some probability distribution and bit sequence. For fixed numbers of different symbols in the sequence, we can enumerate all possibilities (combinations) and encode given one by its number - this approach is called enumerative coding \cite{enum}. More practical are prefix codes, like Huffman coding \cite{huf}, which is computationally inexpensive, but approximates probabilities with powers of 2, what reduces the capacity. We can improve it by grouping a few symbols together, but as we can see in Fig. \ref{huffman}, it is relatively expensive to get really close to the capacity (Shannon entropy). Precise analysis can be found in \cite{huff}.

\begin{figure}[t!]
    \centering
        \includegraphics{huffman.jpg}\
        \caption{Left: construction of Huffman coding while grouping two symbols from $(1,1,1)/3$ probability distribution: we have 
    "#;

    let encoded_seq = encode(&image.to_luma8(), &text.as_bytes().to_vec(), 2);

    let (width, height) = image.dimensions();
    let mut new_image: GrayImage = ImageBuffer::new(width, height);

    let mut x_pos = 50;
    let mut y_pos = 50;

    let message_bit_vec = BitVec::from_bytes(&encoded_seq);

    for i in 0..(width * height) as u64 {
        if i / 8 >= get_image_bytes_encoding_size(&new_image) {
            break;
        }
        // TODO: Why is this needed?
        if i % width as u64 == 0 {
            x_pos += 1;
        }
        x_pos = (x_pos + 19) % width;
        y_pos = (y_pos + 29) % height;

        let maybe_pixel =
            message_bit_vec
                .get(i as usize)
                .map(|bit| if bit { Luma([255]) } else { Luma([0]) });

        if let Some(pixel) = maybe_pixel {
            new_image.put_pixel(x_pos, y_pos, pixel);
        } else {
            break;
        }
    }
    new_image.save("code.bmp").unwrap();

    println!(
        "{:?}",
        String::from_utf8_lossy(&decode(
            &ImageReader::open("code.bmp")?.decode()?.to_luma8(),
            2
        ))
    );

    Ok(())
}
