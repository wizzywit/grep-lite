use clap::Parser;
use colored::*;
use regex::{Regex, RegexBuilder};
use std::{
    collections::VecDeque,
    fs::File,
    io::{self, BufRead, BufReader, Result},
};
use walkdir::WalkDir;

#[derive(Parser)]
#[clap(
    name = "grep-lite",
    version = "0.1",
    about = "searches for patterns in files"
)]
struct Grep {
    pattern: String,

    #[arg(required = false)]
    inputs: Vec<String>,

    #[arg(required = false, short, long)]
    ignore_case: bool,

    #[arg(short = 'v', long)]
    invert_match: bool,

    #[arg(short, long)]
    count: bool,

    #[arg(short, long)]
    recursive: bool,

    #[arg(short = 'A', long = "after", default_value = "0")]
    after_context: usize,

    #[arg(short = 'B', long = "before", default_value = "0")]
    before_context: usize,

    #[arg(short = 'C', default_value = "0")]
    context: usize,
}

fn main() -> Result<()> {
    let args = Grep::parse();
    let re = RegexBuilder::new(&args.pattern)
        .case_insensitive(args.ignore_case)
        .build()
        .unwrap();

    let inputs = &args.inputs;
    let is_multiple_files = inputs.len() > 1;

    let before_context = if args.context > 0 {
        args.context
    } else {
        args.before_context
    };

    let after_context = if args.context > 0 {
        args.context
    } else {
        args.after_context
    };

    if inputs.is_empty() {
        let stdin = io::stdin();
        let reader = stdin.lock();
        process_line(
            reader,
            &re,
            args.invert_match,
            args.count,
            false,
            "-",
            after_context,
            before_context,
        )?;
    }

    for input in inputs {
        if args.recursive {
            for entry in WalkDir::new(input).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file() {
                    process_file(
                        entry.path().to_str().unwrap(),
                        &re,
                        &args,
                        is_multiple_files,
                        after_context,
                        before_context,
                    )?;
                }
            }
        } else {
            process_file(
                input,
                &re,
                &args,
                is_multiple_files,
                after_context,
                before_context,
            )?;
        }
    }
    Ok(())
}

// Process a single file
fn process_file(
    file_name: &str,
    re: &Regex,
    args: &Grep,
    is_multiple_files: bool,
    after_context: usize,
    before_context: usize,
) -> Result<()> {
    let file = File::open(file_name)?;
    let reader = BufReader::new(file);
    process_line(
        reader,
        re,
        args.invert_match,
        args.count,
        is_multiple_files,
        file_name,
        after_context,
        before_context,
    )?;
    Ok(())
}

fn process_line<T: BufRead + Sized>(
    reader: T,
    re: &Regex,
    invert_match: bool,
    count: bool,
    is_multiple_files: bool,
    file_name: &str,
    after_context: usize,
    before_context: usize,
) -> Result<()> {
    let mut current_count = 0;

    let mut before_buffer: VecDeque<(usize, String)> = VecDeque::new();
    let mut after_countdown = 0;

    for (index, line) in reader.lines().enumerate() {
        let line = match line {
            Ok(line) => line,
            Err(e) => {
                println!("{}: Error reading file '{}'", file_name, e);
                break;
            }
        };
        let match_found = re.is_match(&line) != invert_match;

        if match_found {
            for (before_index, before_line) in before_buffer.iter() {
                print_line_with_highlighted_text(
                    count,
                    &mut current_count,
                    before_line,
                    *before_index,
                    is_multiple_files,
                    &file_name,
                    &re,
                    invert_match,
                )?;
            }
            before_buffer.clear();

            print_line_with_highlighted_text(
                count,
                &mut current_count,
                &line,
                index,
                is_multiple_files,
                &file_name,
                &re,
                invert_match,
            )?;
            after_countdown = after_context;
        } else if after_countdown > 0 {
            print_line_with_highlighted_text(
                count,
                &mut current_count,
                &line,
                index,
                is_multiple_files,
                &file_name,
                &re,
                invert_match,
            )?;
            after_countdown -= 1;
        } else {
            before_buffer.push_back((index, line));
            if before_buffer.len() > before_context {
                before_buffer.pop_front();
            }
        }
    }

    if count {
        if is_multiple_files {
            println!("{}: {}", file_name, current_count);
        } else {
            println!("{}", current_count);
        }
    }

    Ok(())
}

fn print_line_with_highlighted_text(
    count: bool,
    current_count: &mut i32,
    line: &str,
    index: usize,
    is_multiple_files: bool,
    file_name: &str,
    highlight_regex: &Regex,
    invert_match: bool,
) -> Result<()> {
    if count {
        *current_count += 1;
    } else {
        let highlighted_line = if invert_match {
            line.to_string()
        } else {
            highlight_regex
                .replace_all(line, |caps: &regex::Captures| {
                    caps[0].bright_red().bold().to_string()
                })
                .to_string()
        };

        if is_multiple_files {
            print!("{}:{}: ", file_name, index + 1);
        } else {
            print!("{}: ", index + 1);
        }
        println!("{}", highlighted_line);
    }

    Ok(())
}
