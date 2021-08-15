mod search_encoding_tree;

extern crate image;

use std::{error::Error, u32};

use image::{DynamicImage, GenericImageView, GrayImage, ImageBuffer, ImageFormat, Luma, io::Reader as ImageReader};
use search_encoding_tree::{generate_transition_function_table, SearchNode, MASK};

use crate::search_encoding_tree::{SearchHistory, BLOCK_SIZE};

fn get_image_bytes_encoding_size(image: &GrayImage) -> u32 {
    let (width, height) = image.dimensions();
    ((width * height) as f32 / 8.0).floor() as u32
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
    println!("{:?}", probabilities);
    probabilities
}

fn calculate_weights(probabilities: &Vec<f32>, step: usize) -> (Vec<f32>, Vec<f32>) {
    let mut zero_bit_weights = vec![0.0_f32; 8];
    let mut one_bit_weights = vec![0.0_f32; 8];
    for i in 0..8 {
        zero_bit_weights[i] = -(1.0 - probabilities[8 * step + i]).ln();
        one_bit_weights[i] = -(probabilities[8 * step + i]).ln();
    }
    println!("{:?} {:?} -- {}", zero_bit_weights, one_bit_weights, step);

    (zero_bit_weights, one_bit_weights)
}

fn calculate_possible_byte_weights(probabilities: &Vec<f32>, step: usize) -> Vec<f32> {
    let weights = calculate_weights(&probabilities, step as usize);
    let mut all_byte_weights: Vec<f32> = vec![0.0_f32; 256];

    for byte in 0..256 {
        all_byte_weights[byte] = (0..8)
            .into_iter()
            .map(|index| {
                if (byte & (1 << index)) == 0 {
                    weights.0[index]
                } else {
                    weights.1[index]
                }
            })
            .sum::<f32>();
    }
    all_byte_weights
}

fn calculate_minmax_weight(prev_search: &Vec<SearchNode>) -> (f32, f32) {
    let weights: Vec<f32> = prev_search.iter().map(|node| node.weight).collect();
    let mut max_weight = weights.iter().cloned().fold(0. / 0., f32::max);
    let min_weight = weights.iter().cloned().fold(0. / 0., f32::min);

    // TODO: is the maxactive >> F needed?
    // mxactive >> freedom_bits_count
    if prev_search.len() > (10_000 >> 2) {
        let interpolation_coeff = 1000.0 / (max_weight - min_weight);
        let mut bucket = [0; 1001];
        for node in prev_search {
            let interpolated_index = (interpolation_coeff * (node.weight - min_weight)).floor() as usize;
            bucket[interpolated_index] += 1;
        }
        let mut pred = 0;
        let mut i: i32 = 0;
        loop {
            if i >= 1000 || pred >= (10_000 >> 2) {
                break;
            }
            pred += bucket[i as usize];
            i += 1;
        }
        max_weight = min_weight + (i - 2) as f32 * (max_weight - min_weight) / 1000.0;
    }

    (max_weight, min_weight)
}

// Find which byte from message to fetch
//      ...and then fetch the exact bit value
fn extract_ith_bit(message: &Vec<u8>, i: usize) -> u8 {
    (message[i / 8] & (1 << (i % 8))) >> (i % 8)
}

fn encode(
    image: &GrayImage,
    message: &Vec<u8>,
    freedom_bit_count: u8,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let probabilities = load_probabilities(image, 50, 50);
    let encoding_byte_size = get_image_bytes_encoding_size(image);

    let mut msg_bit_index = 0;
    let mut search_history = SearchHistory::new();
    // Assign root node with default values
    let mut prev_search = vec![SearchNode::new(None, 0, 0, 0.0_f32)];

    let mut max_weight = 1.0_f32;
    let mut min_weight = 0.0_f32;

    for step in 0..encoding_byte_size {
        let weights = calculate_possible_byte_weights(&probabilities, step as usize);
        search_history.set_weights(weights);

        // Store `8 - freedom_bit_count` in MS positions in `bits_to_encode`
        let mut bits_to_encode = 0;
        for i in freedom_bit_count..8 {
            // TODO: Proper error handling - can go past the message boundary
            let bit = extract_ith_bit(message, msg_bit_index) << i;
            bits_to_encode += bit;
            msg_bit_index += 1;
        }
        prev_search = search_history.expand_search(
            &prev_search,
            max_weight,
            freedom_bit_count,
            bits_to_encode,
        );

        let (new_max_weight, new_min_weight) = calculate_minmax_weight(&prev_search);
        max_weight = new_max_weight;
        min_weight = new_min_weight;
    }

    let maybe_last_node = prev_search.iter().find(|node| node.weight == min_weight);

    let mut encoded_message = vec![];

    if let Some(last_node) = maybe_last_node {
        encoded_message.push(last_node.encoded_byte);
        println!("{:?}", last_node);
        let mut maybe_next_index = last_node.parent_index;
        for _ in 0..encoding_byte_size {
            if let Some(next_index) = maybe_next_index {
                let node = search_history.get(next_index);
                encoded_message.push(node.encoded_byte);
                maybe_next_index = node.parent_index;
            } else {
                break;
            }
        }
        encoded_message.reverse();
    }

    Ok(encoded_message)
}

fn decode(image: &GrayImage, freedom_bit_count: u8) -> Vec<u8> {
    let mut state: u64 = 0;
    
    let (t_table, r_table) = generate_transition_function_table();

    let mut x_pos = 50;
    let mut y_pos = 50;
    let mut current_bit = 0;
    let (width, height) = image.dimensions();

    let mut decoded = vec![0; get_image_bytes_encoding_size(image) as usize];

    let mut byte = 0;
    let mut encoded_bit_pos = 0;
    for i in 0..(width * height) {
        // TODO: Why is this needed?
        if i % width == 0 {
            x_pos += 1;
        }
        x_pos = (x_pos + 19) % width;
        y_pos = (y_pos + 29) % height;
        
        if image.get_pixel(x_pos, y_pos).0[0] > 128 {
            byte = byte | (1 << encoded_bit_pos);
        }
        
        encoded_bit_pos += 1;

        if encoded_bit_pos == 8 {
            print!("{} ", byte);
            let index = (byte ^ state as u8) & MASK;
            let decoded_byte: u8 = r_table[index as usize] as u8;
            let new_state = state ^ t_table[decoded_byte as usize];
            state = new_state.rotate_right(BLOCK_SIZE as u32);

            byte = 0;
            encoded_bit_pos = 0;

            for i in freedom_bit_count..8 {
                let byte_index = current_bit / 8;
                let bit_index = current_bit % 8;
                decoded[byte_index] = decoded[byte_index] | ((decoded_byte >> i) & 1) << bit_index;
                current_bit += 1;
            }
        };
    }
    
    decoded
}

fn main() -> Result<(), Box<dyn Error>> {
    let image = ImageReader::open("picture.bmp")?.decode()?;
    let text = r#"
    Imagine a basic situation: we have a source of symbols of known probability distribution and we would like to design an entropy coder transforming it into a bit sequence, which would be simple and very close to the capacity (Shannon entropy). Prefix codes are the basic method, defining "symbol$\rightarrow$bit sequence" set of rules, usually found using Huffman algorithm. They theoretically allow to reduce the distance from the capacity ($\Delta H$) down to zero, but the cost grows rapidly. We will discuss improving it by replacing this memoryless coder with an automate having some small set of internal states: defined by "(symbol, state)$\rightarrow$(bit sequence, new state)" set of rules. The natural question is the minimal number of states to achieve given performance, what is especially important for simple high throughput hardware coders. Arithmetic coding can be seen this way, but it requires relatively large number of states (possible ranges). We will discuss asymmetric numeral systems (ANS) for this purpose, which can be seen as asymmetrization of numeral systems. Less than 20 states will be usually sufficient to achieve $\Delta H\approx 0.001$ bits/symbol for a small alphabet. $\Delta H$ generally decreases approximately like 1/(the number of states$)^{2}$ and increases proportionally to the size of alphabet. Huge freedom of choosing the exact coding and chaotic behavior of state make it also perfect to simultaneously encrypt the data.
\end{abstract}
\section{Introduction}
Electronics we use is usually based on the binary numeral system, which is perfect for handling integer number of bits of information. However, generally event/symbol of probability $p$ contains $\lg(1/p)$ bits of information ($\lg\equiv \log_2$), which is not necessarily integer in real applications. If all probabilities would be integer powers of $3$ instead, we could optimally use base 3 numeral system and analogously for larger bases. In more complex situations we need to use more sophisticated methods: entropy coders, translating between symbol sequence of some probability distribution and bit sequence. For fixed numbers of different symbols in the sequence, we can enumerate all possibilities (combinations) and encode given one by its number - this approach is called enumerative coding \cite{enum}. More practical are prefix codes, like Huffman coding \cite{huf}, which is computationally inexpensive, but approximates probabilities with powers of 2, what reduces the capacity. We can improve it by grouping a few symbols together, but as we can see in Fig. \ref{huffman}, it is relatively expensive to get really close to the capacity (Shannon entropy). Precise analysis can be found in \cite{huff}.

\begin{figure}[t!]
    \centering
        \includegraphics{huffman.jpg}\
        \caption{Left: construction of Huffman coding while grouping two symbols from $(1,1,1)/3$ probability distribution: we have 
    "#;

    let result = encode(&image.to_luma8(), &text.as_bytes().to_vec(), 2);
    if let Ok(seq) = &result {
        println!("{:?}", seq);
        let (width, height) = image.dimensions();
        let mut new_image: GrayImage = ImageBuffer::new(width, height);

        let mut x_pos = 50;
        let mut y_pos = 50;
        let mut current_bit = 0;
        for i in 0..(width * height) {
            // TODO: Why is this needed?
            if i % width == 0 {
                x_pos += 1;
            }
            x_pos = (x_pos + 19) % width;
            y_pos = (y_pos + 29) % height;
            let pixel = if (seq[current_bit / 8] & (1 << (current_bit % 8))) > 0 {
                Luma([255])
            } else {
                Luma([0])
            };
            current_bit += 1;
            new_image.put_pixel(x_pos, y_pos, pixel);
        }
        new_image.save("coded.png").unwrap();
    }
    println!("{:?}", String::from_utf8_lossy(&decode(&ImageReader::open("coded.png")?.decode()?.to_luma8(), 2)));

    Ok(())
}
