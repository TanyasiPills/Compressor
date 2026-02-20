use std::collections::{BinaryHeap, HashMap};
use std::fs;
use std::cmp::{Ordering, Reverse};

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
struct HuffNode {
    byte: Option<u8>,
    freq: u64,
    left: Option<Box<HuffNode>>,
    right: Option<Box<HuffNode>>
}

impl PartialEq for HuffNode {
    fn eq(&self, other: &Self) -> bool {
        self.freq == other.freq
    }
}

impl Eq for HuffNode {}

impl PartialOrd for HuffNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.freq.cmp(&other.freq))
    }
}

impl Ord for HuffNode {
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
        heap.push(Reverse(Box::new(HuffNode { byte: Some(byte), freq, left: None, right: None })));
    }

    while heap.len() > 1 {
        let Reverse(left) = heap.pop().unwrap();
        let Reverse(right) = heap.pop().unwrap();

        let morg = Box::new(HuffNode {
            byte: None,
            freq: left.freq + right.freq,
            left: Some(left),
            right: Some(right),
        });

        heap.push(Reverse(morg));
    }

    let mut codes = HashMap::new();
    let root = heap.pop().unwrap().0;
    println!("{}", root.freq);
    build_codes_bits(&root, 0, 0, &mut codes);

    let mut buffer: u8 = 0;
    let mut bit_count: u8 = 0;
    let mut output: Vec<u8> = Vec::new();

    for byte in &bytes {
        let (prefix, length) = codes[byte];
        for i in (0..length).rev()  {
            let bit = ((prefix >> i) & 1) as u8;
            buffer = (buffer << 1) | bit;
            bit_count += 1;
            if bit_count == 8 {
                output.push(buffer);
                buffer = 0;
                bit_count = 0;
            }
        }
    }
    if bit_count > 0 {
        let left_over: u8 = 8-bit_count;
        buffer <<= left_over;
        output.push(buffer);
    }

    let output_path: String = format!("{}.loli", api.file_path.split('.').next().unwrap());
    fs::write(output_path, output).expect("cant write encoded data");
}

fn build_codes_bits(node: &Box<HuffNode>, prefix: u8, length: u8, codes: &mut HashMap<u8, (u8, u8)>) {
    if let Some(byte) = node.byte {
        println!("{:8b}; {:8b}; {}", byte, prefix, length.max(1));
        codes.insert(byte, (prefix, length.max(1)));
    } else {
        if let Some(ref l) = node.left {
            build_codes_bits(l, prefix << 1 | 0, length + 1, codes);
        }
        if let Some(ref r) = node.right {
            build_codes_bits(r, prefix << 1 | 1, length + 1, codes);
        }
    }
}

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