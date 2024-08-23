use std::fs;
use std::path::PathBuf;
use clap::Parser;

#[derive(Parser)]
#[command(version, about)]
struct Args {
    #[arg(short, long)]
    file: PathBuf,
}

fn main() {
    let args = Args::parse();

    let buffer = fs::read_to_string(args.file)
        .expect("Couldn't read file");
    print!("{}", prmd::markdown_to_text(buffer.as_str()))
}
