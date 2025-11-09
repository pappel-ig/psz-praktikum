use clap::Parser;
use clap_derive::Parser;
use std::fmt::Debug;
use std::fs::File;
use std::io::{stdin, BufReader, Read};
use std::str;
use str::from_utf8;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct ProgramArgs {
    #[arg(
        help = "File to read"
    )]
    file_name: Option<String>,

    #[arg(
        short = 'c',
        help = "show byte count for file",
        default_value = "false"
    )]
    bytes: bool,

    #[arg(
        short = 'l',
        help = "show line count for file",
        default_value = "false"
    )]
    lines: bool,

    #[arg(
        short = 'w',
        help = "show word count for file",
        default_value = "false"
    )]
    words: bool,

    #[arg(
        short = 'm',
        help = "show character count for file",
        default_value = "false"
    )]
    characters: bool
}

const BUF_SIZE: usize = 65536;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = ProgramArgs::parse();

    // overwrite default settings if no option is present
    if !(args.bytes || args.lines || args.words || args.characters) {
        args = ProgramArgs {
            file_name: args.file_name,
            bytes: true,
            lines: true,
            words: true,
            characters: false,
        }
    }

    // check if file name is provided, else read from stdin
    if let Some(file) = args.file_name.clone() {
        parse(args, Box::new(BufReader::with_capacity(BUF_SIZE, File::open(&file)?)))?;
    } else {
        parse(args, Box::new(BufReader::new(stdin())))?
    }
    Ok(())
}

fn parse(args: ProgramArgs, mut reader: Box<BufReader<dyn Read>>) -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = 0;
    let mut lines = 0;
    let mut words = 0;
    let mut chars = 0;

    let mut buf = [0; BUF_SIZE];
    let mut in_word = false;
    loop {
        let bytes_read = reader.read(&mut buf)?;
        if bytes_read == 0 {
            break;
        }
        if args.bytes {
            bytes += bytes_read as u64;
        }
        if args.lines {
            lines += bytecount::count(&buf[..bytes_read], b'\n');
        }
        if args.words {
            for &b in &buf[..bytes_read] {
                if b.is_ascii_whitespace() {
                    in_word = false;
                } else if !in_word {
                    in_word = true;
                    words += 1;
                }
            }
        }
        if args.characters {
            let mut leftover = Vec::new();
            let chunk = if leftover.is_empty() {
                &buf[..bytes_read]
            } else {
                leftover.extend_from_slice(&buf[..bytes_read]);
                &leftover
            };

            match from_utf8(chunk) {
                Ok(valid_str) => {
                    chars += valid_str.chars().count() as u64;
                    leftover.clear();
                }
                Err(e) => {
                    let valid_up_to = e.valid_up_to();
                    let valid_part = &chunk[..valid_up_to];
                    chars += from_utf8(valid_part).unwrap().chars().count() as u64;
                    leftover = chunk[valid_up_to..].to_vec();
                }
            }
        }
    }
    if args.lines {
        print!("\t{}", lines);
    }
    if args.words {
        print!("\t{}", words);
    }
    if args.bytes {
        print!("\t{}", bytes);
    }
    if args.characters {
        print!("\t{}", chars);
    }
    print!("\n");

    Ok(())
}
