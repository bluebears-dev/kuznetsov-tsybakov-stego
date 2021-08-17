mod encoder;
mod image_encoder;

extern crate image;

use std::{error::Error};

use encoder::{KTEncoder, StandardKTEncoder};
use image::{io::Reader as ImageReader, GenericImageView};

use image_encoder::ImageEncoder;

use crate::image_encoder::{get_image_encoding_capacity, modulo_traversing_encoder::ModuloTraversingEncoder, random_traversing_encoder::RandomTraversingEncoder};

fn main() -> Result<(), Box<dyn Error>> {
    let image = ImageReader::open("picture.png")?.decode()?;
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
    let gray_image = &image.to_luma8();
    let freedom_bit_count = 2;
    let encoding_capacity = get_image_encoding_capacity(gray_image);
    // let image_encoder = RandomTraversingEncoder::new((encoding_capacity * 8) as usize, 13371);
    let image_encoder = ModuloTraversingEncoder {};
    let probabilities = image_encoder.get_probabilities(gray_image, 50, 50);
    let coder = StandardKTEncoder::new(5);

    let data = coder.encode(
        &text.as_bytes().to_vec(),
        &probabilities,
        encoding_capacity,
        freedom_bit_count,
    );
    let encoded_image = image_encoder.encode_into_image(&gray_image, &data);
    encoded_image.save("code.bmp").unwrap();

    let image_with_data = ImageReader::open("code.bmp")?.decode()?.to_luma8();
    let raw_data = image_encoder.decode_from_image(&image_with_data);
    let decoded_message = coder.decode(&raw_data, encoding_capacity, freedom_bit_count);

    let message = String::from_utf8_lossy(&decoded_message);
    println!("{:?}", message.trim_matches(char::from(0)));

    Ok(())
}
