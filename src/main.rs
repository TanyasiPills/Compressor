use std::collections::{BinaryHeap, HashMap};
use std::hint::black_box;
use std::{fs, vec};
use std::cmp::{Ordering, Reverse};

use iced::alignment::Vertical::Bottom;
use iced::{Background, Color, Element, Length, alignment, Task, Event, Subscription, event, window};
use iced::widget::{column, container, Text};

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

    let mut heap = BinaryHeap::new();
    
    for (byte, freq) in map {
        heap.push(Box::new(HuffData{ byte, freq }));
    }

    let mut layer1 = Layer {
        encoded: HashMap::new(),
        chunk: Chunk { block: Vec::new(), unused: 0 },
    };

    fill_layer(&mut layer1, &mut heap);

    let mut buffer: u8 = 0;
    let mut bit_count: u8 = 0;
    let mut output: Vec<u8> = Vec::new();

    let mut left_over: Chunk = Chunk { block:Vec::new(), unused: 0 };

    for byte in &bytes {
        if layer1.encoded.contains_key(byte) {
            let prefix = layer1.encoded[byte];
            for i in (0..3).rev() {
                buffer = (buffer << 1) | (prefix >> i & 1);
                bit_count += 1;

                if bit_count == 8 {
                    layer1.chunk.block.push(buffer);
                    buffer = 0;
                    bit_count = 0;
                }
            }
        }
        else {
            for _i in 0..3 {
                buffer <<= 1;
                bit_count += 1;
                if bit_count == 8 {
                    layer1.chunk.block.push(buffer);
                    buffer = 0;
                    bit_count = 0;
                }
            }
            left_over.block.push(*byte);
            //this is where recursion can accure with multiple layers
        }
        if bit_count > 0 {
            layer1.chunk.unused = 8 - bit_count;
            layer1.chunk.block.push(buffer <<  layer1.chunk.unused);
        }



    }

    let output_path: String = format!("{}.loli", api.file_path.split('.').next().unwrap());
    fs::write(output_path, output).expect("cant write encoded data");
}

fn fill_layer(layer: &mut Layer, heap: &mut BinaryHeap<Box<HuffData>>){
    for i in 1..8 {
        let node = *heap.pop().unwrap();
        layer.encoded.insert(node.byte, i);
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
