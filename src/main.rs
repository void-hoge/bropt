use bropt::brainfuck::{compile, unsafe_run, get_offset};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "bropt")]
#[command(about = "An optimizing brainfuck interpreter")]
struct Args {
    /// Path to the Brainfuck program file to execute
    #[arg(value_name = "FILE")]
    file: String,

    /// Number of cells in the memory tape
    #[arg(short, long, default_value_t = 65536)]
    length: usize,

    /// Flush stdout after each . instruction
    #[arg(short, long, action = clap::ArgAction::SetTrue)]
    flush: bool,
}

fn main() {
    let args = Args::parse();
    let code = std::fs::read_to_string(&args.file).expect("Failed to read the file.");
    let prog = match compile(&code) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };
    let offset = get_offset(&prog);
    if args.flush {
        unsafe_run::<true>(prog, args.length, offset);
    } else {
        unsafe_run::<false>(prog, args.length, offset);
    }
}
