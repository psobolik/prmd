use std::fs;
use std::path::PathBuf;
use clap::Parser;

#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// Print the file without ANSI formatting
    #[arg(short, long)]
    plain: bool,
    /// The file to print
    file: PathBuf,
}

fn main() {
    let args = Args::parse();

    match fs::read_to_string(&args.file) {
        Ok(content) => {
            print!("{}", prmd::markdown_to_text(content.as_str(), args.plain))
        },
        Err(error) => {
            eprintln!("Failed to read file {:?}: {}", args.file, error);
        }
    }
}
