use std::collections::{BinaryHeap, HashMap};
use std::fmt::Binary;
use std::collections::VecDeque;
use std::{fs};
use std::cmp::{Ordering};

use iced::{Background, Color, Element, Length, alignment, Task, Event, Subscription, event, window};
use iced::widget::{column, container, Text};

use libsais::{BwtConstruction, ThreadCount};

use rayon::prelude::*;

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
}

#[derive(Debug)]
struct Chunk {
    block: Vec<u8>,

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

fn mtf_encode(bwt: &Vec<u8>) -> Vec<u8> {
    let mut symbols: [u8; 256] = [0; 256];
    let mut positions: [u8; 256] = [0; 256];
    for i in 0..=255 {
        symbols[i] = i as u8;
        positions[i] = i as u8;
    }

    let mut output = Vec::with_capacity(bwt.len());

    for &byte in bwt {
        let pos = positions[byte as usize];
        output.push(pos);

        // move byte to front
        for i in (0..pos as usize).rev() {
            let s = symbols[i];
            symbols[i + 1] = s;
            positions[s as usize] += 1;
        }
        symbols[0] = byte;
        positions[byte as usize] = 0;
    }

    output
}

fn rle_encode(mtf: &Vec<u8>) -> Vec<u8> {
    let mut output = Vec::new();
    let mut zero_run = 0;

    for &val in mtf {
        if val == 0 {
            zero_run += 1;
        } else {
            while zero_run > 0 {
                let run = zero_run.min(255);
                output.push(0);
                output.push(run as u8);
                zero_run -= run;
            }

            output.push(val);
        }
    }

    while zero_run > 0 {
        let run = zero_run.min(255);
        output.push(0);
        output.push(run as u8);
        zero_run -= run;
    }

    output
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

fn decode_file(file_data: Vec<u8>) {
    let mut heap: VecDeque<_> = file_data.into();
    let mut map: HashMap<u8, u16> = HashMap::new();

    let mut cur_data: u16 = 0;
    let mut not_used: u8 = 0;
    let mut data_len: u64 = 0;
    for _i in 0..7 {
        cur_data = heap.pop_front().unwrap() as u16;
        cur_data <<= 8;
        cur_data |= heap.pop_front().unwrap() as u16;
        map.insert(heap.pop_front().unwrap(), cur_data);
        not_used = heap.pop_front().unwrap();

        for _y in 0..8 {
            data_len <<= 8;
            data_len |= heap.pop_front().unwrap() as u64;
        }
    } 
}

fn encode_file(api: &mut Apy) {
    let file_data: Vec<u8> = fs::read(&api.file_path).expect("failed to read file");

    let (bwt, idx) = bwt_block(&file_data);
    println!("did bwt");
    let mtf = mtf_encode(&bwt);
    println!("did mtf");
    let rle = rle_encode(&mtf);
    println!("did rle");

    //let block_size = 1024;
    //let blocks: Vec<&[u8]> = file_data.chunks(block_size).collect();
    
/*
    let results: Vec<(Vec<u8>, usize)> = blocks
    .par_iter()
    .map(|block| {
        let (bwt, idx) = bwt_block(block);
        let mtf = mtf_encode(&bwt);
        let rle = rle_encode(&mtf);
        (rle, idx)

    })
    .collect();
*/

    println!("got data: btw-{} rle-{}", bwt.len(), rle.len());


    let mut map: HashMap<u8, u64> = HashMap::new();
    for item in &rle {
        *map.entry(*item).or_insert(0) += 1;
    }
    println!("got map: {}", map.len());
    let mut heap = BinaryHeap::new();
    
    for (byte, freq) in map {
        heap.push(Box::new(HuffData{ byte, freq }));
    }
    println!("got heap: {}", heap.len());
    

    let mut current_count: u64 = 0;
    let mut layer: Layer = Layer {
            encoded: HashMap::new(),
            chunk: Chunk { block: Vec::new(), buffer: 0, bit_count: 0 },
    };
    current_count = fill_layer(&mut layer, &mut heap);

    println!("sum:{}", current_count);

    let mut main_chunk: Chunk = Chunk { block:Vec::new(), buffer: 0, bit_count: 0 };
    let mut left_over: Chunk = Chunk { block:Vec::new(), buffer: 0, bit_count: 0 };

    for byte in &rle {
        if layer.encoded.contains_key(byte) {
            main_chunk.buffer = (main_chunk.buffer << 1) | 1;
            main_chunk.bit_count += 1;
            if main_chunk.bit_count == 8 {
                main_chunk.block.push(main_chunk.buffer);
                main_chunk.buffer = 0;
                main_chunk.bit_count = 0;
            }
            
            encode_onto_layer(&mut layer, byte);
        }
        else {
            main_chunk.buffer = main_chunk.buffer << 1;
            main_chunk.bit_count += 1;
            if main_chunk.bit_count == 8 {
                main_chunk.block.push(main_chunk.buffer);
                main_chunk.buffer = 0;
                main_chunk.bit_count = 0;
            }
            left_over.block.push(*byte);
        }
    }
    
    //outputing

    let mut output: Vec<u8> = Vec::new();
    output.extend_from_slice("LOLI".as_bytes());
     
    for item in &layer.encoded {
        output.push(*item.0);
        output.push(*item.1);
    }
    let not_used: u8 = 8 - layer.chunk.bit_count;
    if layer.chunk.bit_count > 0 {
        layer.chunk.block.push(layer.chunk.buffer << not_used);
    }
    output.push(not_used % 8);
    output.extend_from_slice(&(layer.chunk.block.len() as u64).to_le_bytes());

    let mut not_used: u8 = 8 - main_chunk.bit_count;
    if main_chunk.bit_count > 0 {
        main_chunk.block.push(main_chunk.buffer << not_used);
    }
    output.push(not_used % 8);

    not_used = 8 - left_over.bit_count;
    if left_over.bit_count > 0 {
        left_over.block.push(left_over.buffer << not_used);
    }
    output.push(not_used % 8);

    output.extend_from_slice(&main_chunk.block);

    output.extend_from_slice(&layer.chunk.block);
    
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

fn encode_onto_layer(layer: &mut Layer, data: &u8) {
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