use std::{fs};

use iced::{Background, Color, Element, Length, alignment, Task, Event, Subscription, event, window};
use iced::widget::{Button, Text, button, column, container, row};

use rfd::FileDialog;

fn main() -> iced::Result {
    iced::application(new, update, view)
    .title("Blahajs for everyone >:3")
    .subscription(subscription)
    .run()
}

#[derive(Debug, Clone)]
enum Message {
    None,
    FileDropped(String),
    File,
    Unfile
}

#[derive(Debug)]
struct ANSTable {
    freq: Vec<u32>,       // frequency of each symbol
    cumul: Vec<u32>,      // cumulative frequency (where each symbol starts)
    total: u32,           // sum of all frequencies

    slot_to_sym: Vec<u8>,
}

#[derive(Debug)]
struct Apy {
    file_path: String,
    table: ANSTable
}

#[derive(Debug)]
struct ANSEncoder {
    state: u64,
    output: Vec<u8>
}
#[derive(Debug)]
struct ANSDecoder {
    state: u64,
    input: Vec<u8>,
    pos: usize
}

const L: u64 = 1 << 23;

fn ANSCreateTable(freqs: Vec<u32>) -> ANSTable {
    let total: u32 = freqs.iter().sum();

    let mut cumul =  vec![0u32; freqs.len() + 1];
    for i in 0..freqs.len() {
        cumul[i+1] = cumul[i] + freqs[i];
    }

    let mut slots = vec![0u8; total as usize];
    for i in 0..freqs.len() {
        for sym in cumul[i]..cumul[i+1] {
            slots[sym as usize] = i as u8;
        }
    }

    ANSTable { freq: freqs, cumul, total, slot_to_sym: slots }
}
fn ANSEncoding(symbol: u8, encoder: &mut ANSEncoder, table: &ANSTable) {
    let m = table.total as u64;
    let freq = table.freq[symbol as usize] as u64;
    let cumul = table.cumul[symbol as usize] as u64;

    // Normalize: shrink state until it's in the valid range
    let upper = freq << 8; // i.e. freq * b, where b=256
    while encoder.state >= upper {
        encoder.output.push((encoder.state & 0xFF) as u8);
        encoder.state >>= 8;
    }
    // Encode symbol into state
    encoder.state = (encoder.state / freq) * m + cumul + (encoder.state % freq);
}

fn ANSCloseEncoding(encoder: &mut ANSEncoder) {
    for _ in 0..4 {
        encoder.output.push((encoder.state & 0xFF) as u8);
        encoder.state >>= 8;
    }
}

fn ANSDecoding(decoder: &mut ANSDecoder, table: &ANSTable) -> Vec<u8> {
    let input = &decoder.input;
    let m = table.total as u64;

    // Load initial state from the last 4 bytes (little-endian)
    let n = input.len();
    decoder.state = 0u64;
    for i in 0..4 {
        decoder.state |= (input[n - 4 + i] as u64) << (i * 8);
    }
    decoder.pos = n - 5; // next byte to read, going backwards

    let mut output = vec![0u8; m as usize];
    for item in &mut output {
        let slot = decoder.state % m;
        *item = table.slot_to_sym[slot as usize];
        let freq = table.freq[*item as usize] as u64;
        let cumul = table.cumul[*item as usize] as u64;

        decoder.state = freq * (decoder.state / m) + slot - cumul;

        while decoder.state < m {
            decoder.state = (decoder.state << 8) | (input[decoder.pos] as u64);
            if decoder.pos > 0 { decoder.pos -= 1; }
            else { break; }
        }
    }

    output.reverse(); // ANS decodes in reverse symbol order
    output
}

fn new() -> (Apy, Task<Message>) {
    (
        Apy {
            file_path: String::new(),
            table: ANSTable { freq: Vec::new(), cumul: Vec::new(), total: 0, slot_to_sym: Vec::new() }
        },
        Task::none(),
    )
}


//ˇˇˇˇˇˇold down there
fn decode_file(file_path: String, table: &ANSTable) {

    let file_data: Vec<u8> = fs::read(&file_path).expect("failed to read file");

    let mut decoder = ANSDecoder { state: 0, input: file_data, pos: 0};

    println!("{} table total, {} table slots", table.total, table.slot_to_sym[0]);

    let output = ANSDecoding(&mut decoder, &table);

    let output_path: String = format!("{}unlolied.jpg", file_path.split('.').next().unwrap());
    fs::write(output_path, output).expect("cant write decoded data");
/*

    //we check if its a compressed file
    let mut buf = [0u8; 4];
    for i in 0..4 {
        buf[i] = heap.pop_front().unwrap();
    }

    if &buf != b"LOLI" {
        println!("Got bamboozled, the file is not even a loli");
        return
    }

    //we retrive the bwt encoding index
    let mut buf = [0u8; 8];
    for i in 0..8 {
        buf[i] = heap.pop_front().unwrap();
    }
    let bwt_index = u64::from_le_bytes(buf) as usize;

    //we get the huffman coding map
    for _i in 0..8 {
        let encoded = heap.pop_front().unwrap();
        map.insert(heap.pop_front().unwrap(), encoded);
    }

    for item in &map {
        println!("map entry: {};{}", item.0, item.1);
    }

    //we get the not used bits count
    for _i in 0..3 { not_used.push(heap.pop_front().unwrap()) }

    //we get the size of the data chunks
    for _i in 0..3 { 
        let mut buf = [0u8; 8];
        for i in 0..8 {
            buf[i] = heap.pop_front().unwrap();
        }
        let cur_len = u64::from_le_bytes(buf) as usize;
        data_len.push(cur_len as usize);
    }

    let main_chunk: Vec<u8> = heap.drain(..data_len[0]).collect();
    let layer_chunk: Vec<u8> = heap.drain(..data_len[1]).collect();
    let mut leftover_chunk:  VecDeque<_> = heap.drain(..data_len[2]).collect();

    let mut original: Vec<u8> = Vec::new();
    let mut layer_index: usize = 0;
    let mut layer_bit: u8 = 0;

    let total_bits = main_chunk.len() * 8 - not_used[0] as usize;

    let mut bits_read = 0;

    //undoing huffman coding
    for byte in &main_chunk {
        for i in (0..8).rev() {
            if bits_read == total_bits {
                break;
            }

            let bit = (byte >> i) & 1;
            bits_read += 1;

            if bit == 1 { original.push(map[&get_prefix(&layer_chunk, &mut layer_index, &mut layer_bit)]) } 
            else { original.push(leftover_chunk.pop_front().unwrap()) };
        }
    }

    original = rle_decode(&original);
    original = mtf_decode(&original);
    original = bwt_decode(&original, bwt_index);


    let output_path: String = format!("{}unlolied.gif", file_path.split('.').next().unwrap());
    fs::write(output_path, original).expect("cant write decoded data");
*/
}

fn encode_file(file_path: String) -> ANSTable {
    let file_data: Vec<u8> = fs::read(&file_path).expect("failed to read file");

    let mut freq_table = vec![0u32; 256];
    
    for item in &file_data{
        freq_table[*item as usize] = freq_table[*item as usize] +1;
    }

    let table = ANSCreateTable(freq_table);

    let mut encoder = ANSEncoder { state: table.total as u64, output: Vec::new() };
    
    for item in &file_data {
        ANSEncoding(*item, &mut encoder, &table);
    }

    ANSCloseEncoding(&mut encoder);

    println!("{} table total, {} file size", table.total, file_data.len());

    let output_path: String = format!("{}.loli", file_path.split('.').next().unwrap());
    fs::write(output_path, encoder.output).expect("cant write encoded data");

    table
    
    //outputing
/*
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
*/
}


// ui stuff

fn update(api: &mut Apy, message: Message) {
    match message {
        Message::FileDropped(path) => {
            api.file_path = path;
            if api.file_path.split('.').last().unwrap() == "loli" {
                decode_file(api.file_path.clone(), &api.table);
            } else {
                api.table = encode_file(api.file_path.clone());
            }
        }
        Message::File => {
            if let Some(path) = FileDialog::new().pick_file() {
                api.file_path = path.to_string_lossy().to_string();
                api.table = encode_file(api.file_path.clone());
                println!("{} total freqs", api.table.total);
            }
        }
        Message::Unfile => {
            if let Some(path) = FileDialog::new().pick_file() {
                api.file_path = path.to_string_lossy().to_string();
                decode_file(api.file_path.clone(), &api.table);
            }
        }
        _ => {}
    }
}
fn view(api: &Apy) -> Element<'_, Message> {
    let text = Text::new(&api.file_path);
    let button_text = Text::new("Ola amigo");
    let decode_text: Text = Text::new("si senyore");
    let button = Button::new(button_text).on_press(Message::File);
    let d_button = Button::new(decode_text).on_press(Message::Unfile);
    let children = [
        container(
            column([text.into(), button.into(), d_button.into()]))
            .width(Length::Fill)
            .height(Length::FillPortion(4))
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center)
            .style(|_theme| container::Style::default().background(Background::Color(Color::from_rgb(0.0, 0.0, 0.0))).color(Color::from_rgb(1.0, 0.0, 1.0)))
            .into(),
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
