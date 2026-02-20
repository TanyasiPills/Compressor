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
    byte: u8,
    freq: u64,
}

#[derive(Debug)]
struct Layer {
    encoded : HashMap<u8, u8>, //(incoded, prefix)
    chunk: Chunk,

    buffer: u8,
    bit_count: u8
}

#[derive(Debug)]
struct Chunk {
    block: Vec<u8>,
    unused: u8 //unused bits on the end of chunk
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

    // get primary index (needed to invert BWT)
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
    let bytes: Vec<u8> = fs::read(&api.file_path).expect("failed to read file");
    let mut map: HashMap<u8, u64> = HashMap::new();

    for item in &bytes {
        *map.entry(*item).or_insert(0) += 1;
    }

    let (bwt, idx) = bwt_block(&bytes[..1024]);
    println!("got bwt: {} {}", bwt.len(), idx);

    let mut heap = BinaryHeap::new();
    
    for (byte, freq) in map {
        heap.push(Box::new(HuffData{ byte, freq }));
    }

    let mut current_count: u64 = 0;
    let mut layers: Vec<Layer> = Vec::new();
    println!("{}", bytes.len());
    while layers.len() < 1 {
        let mut layer = Layer {
            encoded: HashMap::new(),
            chunk: Chunk { block: Vec::new(), unused: 0 },
            buffer: 0,
            bit_count: 0
        };

        current_count += fill_layer(&mut layer, &mut heap);
        layers.push(layer);
    }
    println!("sum:{}", current_count);

    let mut left_over: Chunk = Chunk { block:Vec::new(), unused: 0 };


    for encode in &layers.first().unwrap().encoded {
        println!("{:16b} {:8b}", encode.0, encode.1);
    }

    for byte in &bytes {
            encode_onto_layer(&mut layers, 0, byte, &mut left_over);
    }

    
    let mut output: Vec<u8> = Vec::new();
    output.extend_from_slice("LOLI 1".as_bytes());
    /* 
    for layer in &layers {
    }*/
        for item in &layers.first().unwrap().encoded {
            output.push(*item.0);
            output.push(*item.1);
        }
        output.extend_from_slice(&layers.first().unwrap().chunk.block);
    output.extend_from_slice(&left_over.block);

    let output_path: String = format!("{}.loli", api.file_path.split('.').next().unwrap());
    fs::write(output_path, output).expect("cant write encoded data");
}

fn fill_layer(layer: &mut Layer, heap: &mut BinaryHeap<Box<HuffData>>) -> u64{
    let mut freq_sum = 0;
    for i in 1..8 {
        let node = *heap.pop().unwrap();
        print!("{}; ", node.freq);
        layer.encoded.insert(node.byte, i);
        freq_sum += node.freq;
    }
    freq_sum
}

fn encode_onto_layer(layers: &mut Vec<Layer>, index: u8, data: &u8, left_over: &mut Chunk) {
    let layer = &mut layers[index as usize];
    if layer.encoded.contains_key(data) {
        let prefix = layer.encoded[data];
        for i in (0..3).rev() {
            layer.buffer = (layer.buffer << 1) | ((prefix >> i) & 1);
            layer.bit_count += 1;
            if layer.bit_count == 8 {
                layer.chunk.block.push(layer.buffer);
                layer.buffer = 0;
                layer.bit_count = 0;
            }
        }
    } else {
        for _i in 0..3 {
            layer.buffer <<= 1;
            layer.bit_count += 1;
            if layer.bit_count == 8 {
                layer.chunk.block.push(layer.buffer);
                layer.buffer = 0;
                layer.bit_count = 0;
            }
        }
        if layers.len() -1 == index as usize {
            left_over.block.push(*data);
        } else {
            //encode_onto_layer(layers, index+1, data, left_over);
            left_over.block.push(*data);
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