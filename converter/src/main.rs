use clap::Parser;
use memchr::memmem::find_iter;
use std::path::PathBuf;

#[derive(Parser, Debug)]
struct Args {
	input: PathBuf,
	output: PathBuf,
}

fn main() {
	let args = Args::parse();
	let mut file = std::fs::read(args.input).unwrap();

	let pattern = b"_external\0";

	let indices = find_iter(&file, pattern).collect::<Vec<_>>();
	for index in indices {
		file[index] = b'\0';
	}

	std::fs::write(args.output, file).unwrap();
}
