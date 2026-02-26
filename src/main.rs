use std::collections::{BinaryHeap, HashMap};
use std::fmt::Binary;
use std::collections::VecDeque;
use std::{fs};
use std::cmp::{Ordering};

use iced::{Background, Color, Element, Length, alignment, Task, Event, Subscription, event, window};
use iced::widget::{column, container, Text};

use libsais::bwt::Bwt;
use libsais::{BwtConstruction, ThreadCount, UnBwt};

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

fn bwt_encode(data: &[u8]) -> (Vec<u8>, usize) {
    let res = BwtConstruction::for_text(data)
        .with_owned_temporary_array_buffer32()
        .multi_threaded(ThreadCount::openmp_default())                  
        .run()
        .expect("BWT failed");

    let test = res.into_parts();

    (test.0, test.1)
}

fn bwt_decode(mtf: &[u8], main_index: usize) -> Vec<u8> {
    let mut output: Vec<u8> = Vec::new();
    unsafe {
        let mut data= 
             Bwt::<u8, _>::from_parts(mtf, main_index)
            .unbwt()
            .with_owned_temporary_array_buffer32()
            .multi_threaded(ThreadCount::openmp_default())
            .run()
            .unwrap();

        output = data.as_slice().to_vec();
    }

    output
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

fn mtf_decode(rle: &Vec<u8>) -> Vec<u8> {
    let mut symbols = [0u8; 256];
    for i in 0..=255 {
        symbols[i] = i as u8;
    }

    let mut output = Vec::with_capacity(rle.len());

    for &pos in rle {
        let idx = pos as usize;
        let byte = symbols[idx];
        output.push(byte);

        // move to front
        for i in (1..=idx).rev() {
            symbols[i] = symbols[i - 1];
        }
        symbols[0] = byte;
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

fn rle_decode(data: &Vec<u8>) -> Vec<u8> {
    let mut output = Vec::new();
    let mut i = 0;

    while i < data.len() {
        if data[i] == 0 {
            let run = data[i + 1] as usize;
            output.extend(std::iter::repeat(0).take(run));
            i += 2;
        } else {
            output.push(data[i]);
            i += 1;
        }
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

fn get_prefix(data: &Vec<u8>, index: &mut usize, bit: &mut u8) -> u8 {
    let mut count: u8 = 0;
    let mut prefix: u8 = 0;

    let mut byte = data[*index];

    while count != 3 {
        prefix = (prefix << 1) | (byte & 1);
        count += 1;

        *bit += 1;
        if *bit == 8 {
            *index += 1;
            *bit = 0;
            if count != 3 { byte = data[*index] }
        }
    }

    return prefix;
}

fn decode_file(file_path: String) {

    let file_data: Vec<u8> = fs::read(&file_path).expect("failed to read file");

    let mut heap: VecDeque<_> = file_data.into();
    let mut map: HashMap<u8, u8> = HashMap::new();

    let mut not_used: Vec<u8> = Vec::new();
    let mut data_len: Vec<usize> = Vec::new();


    //we check if its a compressed file
    let mut buf = [0u8; 4];
    for i in 0..4 {
        buf[i] = heap.pop_front().unwrap();
    }

    if &buf != b"loli" {
        println!("Got bamboozled, the file is not even a loli");
        return
    }

    //we retrive the bwt encoding index
    let bwt_index = heap.pop_front();

    //we get the huffman coding map
    for _i in 0..8 {
        let encoded = heap.pop_front().unwrap();
        map.insert(heap.pop_front().unwrap(), encoded);
    }

    //we get the not used bits count
    for _i in 0..3 { not_used.push(heap.pop_front().unwrap()) }

    //we get the size of the data chunks
    for _i in 0..3 { 
        let mut cur_len: usize = 0;
        for _y in 0..8 {
            cur_len = (cur_len << 8) | (heap.pop_front().unwrap() as usize);
        }
        data_len.push(cur_len as usize);
    }

    let main_chunk: Vec<u8> = heap.drain(..data_len[0]).collect();
    let layer_chunk: Vec<u8> = heap.drain(..data_len[1]).collect();
    let mut leftover_chunk:  VecDeque<_> = heap.drain(..data_len[2]).collect();

    let mut original: Vec<u8> = Vec::new();
    let mut layer_index: usize = 0;
    let mut layer_bit: u8 = 0;

    //undoing huffman coding
    for byte in &main_chunk {
        for i in (0..8).rev() {
            let bit = (byte >> i) & 1;
            if bit == 1 { original.push(map[&get_prefix(&layer_chunk, &mut layer_index, &mut layer_bit)]) } 
            else { original.push(leftover_chunk.pop_front().unwrap()) };
        }
    }




    let output_path: String = format!("{}.gif", file_path.split('.').next().unwrap());
    fs::write(output_path, original).expect("cant write decoded data");

}

fn encode_file(file_path: String) {
    let file_data: Vec<u8> = fs::read(&file_path).expect("failed to read file");

    let (bwt, idx) = bwt_encode(&file_data);
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

    //original index for bwt decoding
    output.extend_from_slice(&(idx as u64).to_le_bytes());
     
    for item in &layer.encoded {
        output.push(*item.0);
        output.push(*item.1);
    }

    //pushes not used bits for the three chunk
    let mut not_used: u8 = 8 - main_chunk.bit_count;
    if main_chunk.bit_count > 0 { main_chunk.block.push(main_chunk.buffer << not_used) }
    output.push(not_used % 8);

    not_used = 8 - layer.chunk.bit_count;
    if layer.chunk.bit_count > 0 { layer.chunk.block.push(layer.chunk.buffer << not_used) }
    output.push(not_used % 8);

    not_used = 8 - left_over.bit_count;
    if left_over.bit_count > 0 { left_over.block.push(left_over.buffer << not_used) }
    output.push(not_used % 8);

    //append chunksize and chunks
    output.extend_from_slice(&(main_chunk.block.len() as u64).to_le_bytes());
    output.extend_from_slice(&(layer.chunk.block.len() as u64).to_le_bytes());
    output.extend_from_slice(&(left_over.block.len() as u64).to_le_bytes());

    output.extend_from_slice(&main_chunk.block);
    output.extend_from_slice(&layer.chunk.block);
    output.extend_from_slice(&left_over.block);

    let output_path: String = format!("{}.loli", file_path.split('.').next().unwrap());
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
            if api.file_path.split('.').last().unwrap() == "loli" {
                decode_file(api.file_path.clone());
            } else {
                encode_file(api.file_path.clone());
            }
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