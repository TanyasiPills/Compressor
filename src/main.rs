use std::collections::{BinaryHeap, HashMap};
use std::{fs};
use std::cmp::{Ordering};

use iced::{Background, Color, Element, Length, alignment, Task, Event, Subscription, event, window};
use iced::widget::{column, container, Text};

use libsais::{BwtConstruction, ThreadCount};

fn main() -> iced::Result {
    iced::application(new, update, view)
    .title("Blahajs for everyone >:3")
    .subscription(subscription)
    .run()
}

#[derive(Debug, Clone)]
enum Message {
    None,
    FileDropped(String)
}

#[derive(Debug)]
struct HuffData {
    byte: u16,
    freq: u64,
}

#[derive(Debug)]
struct Layer {
    encoded : HashMap<u16, u8>, //(incoded, prefix)
    chunk: Chunk,
}

#[derive(Debug)]
struct Chunk {
    block: Vec<u8>,
    unused: u8, //unused bits on the end of chunk

    buffer: u8,
    bit_count: u8
}

impl PartialEq for HuffData {
    fn eq(&self, other: &Self) -> bool {
        self.freq == other.freq
    }
}
impl Eq for HuffData {}
impl PartialOrd for HuffData {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.freq.cmp(&other.freq))
    }
}
impl Ord for HuffData {
    fn cmp(&self, other: &Self) -> Ordering {
        self.freq.cmp(&other.freq)
    }
}

fn bwt_block(data: &[u8]) -> (Vec<u8>, usize) {
    let res = BwtConstruction::for_text(data)
        .with_owned_temporary_array_buffer32()
        .multi_threaded(ThreadCount::openmp_default())                  
        .run()
        .expect("BWT failed");

    let bwt: Vec<u8> = res.bwt().to_vec();
    let orig_idx = res.primary_index();

    (bwt, orig_idx)
}

#[derive(Debug, Default)]
struct Apy {
    file_path: String,
}

fn new() -> (Apy, Task<Message>) {
    (
        Apy {
            file_path: String::new()
        },
        Task::none(),
    )
}

fn encode_file(api: &mut Apy) {
    let file_data: Vec<u8> = fs::read(&api.file_path).expect("failed to read file");

    let (bwt, idx) = bwt_block(&file_data);
    println!("got bwt: {} {}", bwt.len(), idx);
    let base: u8 = *bwt.first().unwrap();

    let previous = base;
    let mut bytes: Vec<u8> = Vec::new();

    for item in bwt {
        bytes.push((item as i16 - previous as i16 + 128) as u8);
    }
    println!("got bytes: {}", bytes.len());

    let mut map: HashMap<u16, u64> = HashMap::new();
    let mut bit_len = 0;
    let mut bits: u16 = 0;
    for item in &bytes {
        bits <<= 8;
        bits |= *item as u16;
        bit_len += 8;
        if bit_len == 16 {
            *map.entry(bits).or_insert(0) += 1;
            bit_len = 0;
            bits = 0;
        }
    }
    println!("got map: {}", map.len());
    let mut heap = BinaryHeap::new();
    
    for (byte, freq) in map {
        heap.push(Box::new(HuffData{ byte, freq }));
    }
    println!("got heap: {}", heap.len());
    

    let mut current_count: u64 = 0;
    let mut layers: Vec<Layer> = Vec::new();
    while layers.len() < 7 {
        let mut layer = Layer {
            encoded: HashMap::new(),
            chunk: Chunk { block: Vec::new(), unused: 0, buffer: 0, bit_count: 0 },
        };

        current_count += fill_layer(&mut layer, &mut heap);
        layers.push(layer);
    }
    println!("sum:{}", current_count);

    let mut main_chunk: Chunk = Chunk { block:Vec::new(), unused: 0, buffer: 0, bit_count: 0 };
    let mut left_over: Chunk = Chunk { block:Vec::new(), unused: 0, buffer: 0, bit_count: 0 };

    bits = 0;
    bit_len = 0;
    for byte in &bytes {
        bits = bits << 8 | *byte as u16;
        bit_len += 8;

        if bit_len == 16 {
            if let Some((index, layer)) = layers.iter_mut().enumerate().find(|layer| layer.1.encoded.contains_key(&bits)) {
                let main_index: u8 = (index+1) as u8; //+1 cause 000 is the main pool
                for i in (0..3).rev() {
                    main_chunk.buffer = (main_chunk.buffer << 1) | ((main_index >> i) & 1);
                    main_chunk.bit_count += 1;
                    if main_chunk.bit_count == 8 {
                        main_chunk.block.push(main_chunk.buffer);
                        main_chunk.buffer = 0;
                        main_chunk.bit_count = 0;
                    }
                }
                encode_onto_layer(layer, &bits);
            }
            else {
                for _i in (0..3).rev() {
                    main_chunk.buffer = main_chunk.buffer << 1;
                    main_chunk.bit_count += 1;
                    if main_chunk.bit_count == 8 {
                        main_chunk.block.push(main_chunk.buffer);
                        main_chunk.buffer = 0;
                        main_chunk.bit_count = 0;
                    }
                }

                left_over.block.push((bits >> 8) as u8);
                left_over.block.push(bits as u8);
            }
            bits = 0;
            bit_len = 0;
        }

    }
    
    //outputing

    let mut output: Vec<u8> = Vec::new();
    output.extend_from_slice("LOLI 1".as_bytes());
     
    for layer in &layers {
        for item in &layer.encoded {
            output.push((*item.0 >> 8) as u8);
            output.push(*item.0 as u8);
            output.push(*item.1);
        }

        output.extend_from_slice(&layer.chunk.block);
    }

    output.extend_from_slice(&left_over.block);

    let output_path: String = format!("{}.loli", api.file_path.split('.').next().unwrap());
    fs::write(output_path, output).expect("cant write encoded data");
}

fn fill_layer(layer: &mut Layer, heap: &mut BinaryHeap<Box<HuffData>>) -> u64{
    let mut freq_sum = 0;
    for i in 1..=8 {
        let node: HuffData = *heap.pop().unwrap();
        print!("{}; ", node.freq);
        layer.encoded.insert(node.byte, i);
        freq_sum += node.freq;
    }
    freq_sum
}

fn encode_onto_layer(layer: &mut Layer, data: &u16) {
    let prefix = layer.encoded[data];
    for i in (0..3).rev() {
        layer.chunk.buffer = (layer.chunk.buffer << 1) | ((prefix >> i) & 1);
        layer.chunk.bit_count += 1;
        if layer.chunk.bit_count == 8 {
            layer.chunk.block.push(layer.chunk.buffer);
            layer.chunk.buffer = 0;
            layer.chunk.bit_count = 0;
        }
    }
}



// ui stuff

fn update(api: &mut Apy, message: Message) {
    match message {
        Message::FileDropped(path) => {
            api.file_path = path;
            encode_file(api);
        }
        Message::None => {}
    }
}
fn view(api: &Apy) -> Element<'_, Message> {
    let text = Text::new(&api.file_path);
    let children = [
        container(text)
            .width(Length::Fill)
            .height(Length::FillPortion(4))
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center)
            .style(|_theme| container::Style::default().background(Background::Color(Color::from_rgb(0.0, 0.0, 0.0))).color(Color::from_rgb(1.0, 0.0, 1.0)))
            .into()
    ];
    column(children).into()
}

fn subscription(_api: &Apy) -> Subscription<Message> {
    event::listen().map(|event| match event {
        Event::Window(window::Event::FileDropped(path)) => {
            Message::FileDropped(path.to_string_lossy().to_string())
        }
        _ => Message::None
    })
}