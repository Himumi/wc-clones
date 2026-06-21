use std::env;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader, IsTerminal, Read, stdin};

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut command = match parse(&args) {
        Ok(value) => value,
        Err(err) => return println!(" {}", err),
    };

    if command.is_help {
        return println!("{}", HELP_MESSAGE);
    }

    let mut counts = read_file(&command.paths);
    if counts.len() > 1 {
        command.paths.push(String::from("total"));
        counts.push(io::Result::Ok(total_count(&counts)));
    }

    print_out(&counts, &command);
}

#[derive(Default)]
struct Command {
    paths: Vec<String>,
    is_help: bool,
    is_line: bool,
    is_word: bool,
    is_byte: bool,
}

#[derive(Debug)]
enum ParseError {
    UnknownFlag(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::UnknownFlag(flag) => {
                write!(f, "An Unknown Flag Provided: {}.", flag)
            }
        }
    }
}

impl Error for ParseError {}

fn parse(args: &[String]) -> Result<Command, ParseError> {
    let mut command = Command::default();

    for arg in args.iter().skip(1) {
        if !arg.starts_with("-") {
            command.paths.push(arg.to_string());
        } else if arg == "--help" {
            command.is_help = true;
        } else if arg == "-l" || arg == "--line" {
            command.is_line = true;
        } else if arg == "-w" || arg == "--word" {
            command.is_word = true;
        } else if arg == "-c" || arg == "--byte" {
            command.is_byte = true;
        } else {
            return Err(ParseError::UnknownFlag(arg.to_string()));
        }
    }

    Ok(command)
}

#[derive(Default)]
struct Count {
    line: usize,
    word: usize,
    byte: usize,
}

fn count_word<R: Read>(input: R, buffer: &mut String) -> io::Result<Count> {
    let mut reader = BufReader::new(input);
    let mut count = Count::default();

    while reader.read_line(buffer)? > 0 {
        count.line += 1;
        count.byte += buffer.len();

        let mut appeared = false;
        for c in buffer.chars() {
            if (c == ' ' || c == '\n') && appeared {
                count.word += 1;
                appeared = false;
            }

            if c != ' ' && !appeared {
                appeared = true;
            }
        }

        buffer.clear();
    }

    Ok(count)
}

fn total_count(counts: &[io::Result<Count>]) -> Count {
    let mut total = Count::default();

    for count in counts.iter().flatten() {
        total.line += count.line;
        total.word += count.word;
        total.byte += count.byte;
    }

    total
}

fn read_file(paths: &[String]) -> Vec<io::Result<Count>> {
    let mut buffer = String::with_capacity(4096);

    let mut result: Vec<io::Result<Count>> =
        Vec::with_capacity(if paths.len() > 1 { paths.len() + 1 } else { 1 });

    if !stdin().is_terminal() && paths.len().eq(&0) {
        result.push(count_word(stdin(), &mut buffer));
    } else {
        if paths.len().eq(&0) {
            result.push(Err(io::Error::other(String::from("Empty File Path"))));
        }

        for path in paths.iter() {
            match File::open(path) {
                Ok(value) => result.push(count_word(value, &mut buffer)),
                Err(err) => result.push(Err(err)),
            };
        }
    }

    result
}

fn print_out(results: &[io::Result<Count>], command: &Command) {
    let digit_max = max_digit(results);

    for (i, result_union) in results.iter().enumerate() {
        match result_union {
            Err(err) => println!(" {}", err),
            Ok(count) => {
                if command.is_line {
                    print!("{:1$}", count.line, digit_max);
                }
                if command.is_word {
                    print!("{:1$}", count.word, digit_max + 1);
                }
                if command.is_byte {
                    print!("{:1$}", count.byte, digit_max + 1);
                }

                // No Flags
                if !command.is_line && !command.is_word && !command.is_byte {
                    print!("{:1$}", count.line, digit_max);
                    print!("{:1$}", count.word, digit_max + 1);
                    print!("{:1$}", count.byte, digit_max + 1);
                }

                match command.paths.get(i) {
                    Some(value) => println!(" {}", value),
                    None => println!(),
                }
            }
        }
    }
}

fn max_digit(results: &[io::Result<Count>]) -> usize {
    let mut max: usize = 0;

    for count in results.iter().flatten() {
        max = max.max(count_digit(count.line));
        max = max.max(count_digit(count.word));
        max = max.max(count_digit(count.byte));
    }

    max
}

fn count_digit(n: usize) -> usize {
    if n < 10 {
        return 1;
    }
    (n as f64).log10().ceil() as usize
}

const HELP_MESSAGE: &str = "Usage: rwc [OPTION]... [FILE]...

Options:
    -c, --byte
        print the byte counts
    -w, --word
        print the word counts
    -l, --line
        print the newline counts
    --help
        display this help and exit
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_digit() {
        struct Case {
            num: usize,
            want: usize,
        }

        let cases: [Case; 3] = [
            Case { num: 8, want: 1 },
            Case { num: 88, want: 2 },
            Case { num: 888, want: 3 },
        ];

        for case in cases {
            assert_eq!(count_digit(case.num), case.want);
        }
    }

    #[test]
    fn test_max_digit() {
        let results: [io::Result<Count>; 3] = [
            io::Result::Ok(Count {
                line: 1,
                word: 12,
                byte: 123,
            }),
            io::Result::Ok(Count {
                line: 1,
                word: 12,
                byte: 123,
            }),
            io::Result::Err(io::Error::other(String::from("Empty File Path"))),
        ];

        let digit = max_digit(&results);
        assert_eq!(digit, 3);
    }

    #[test]
    fn test_count_total() {
        let results: [io::Result<Count>; 3] = [
            io::Result::Ok(Count {
                line: 1,
                word: 12,
                byte: 123,
            }),
            io::Result::Ok(Count {
                line: 1,
                word: 12,
                byte: 123,
            }),
            io::Result::Err(io::Error::other(String::from("Empty File Path"))),
        ];

        let total = total_count(&results);

        assert_eq!(total.line, 2);
        assert_eq!(total.word, 24);
        assert_eq!(total.byte, 246);
    }

    #[test]
    fn test_parse() {
        struct CommandCase {
            paths: &'static [&'static str],
            is_help: bool,
            is_line: bool,
            is_word: bool,
            is_byte: bool,
        }

        struct Case {
            args: &'static [&'static str],
            want: CommandCase,
        }

        let cases: [Case; 5] = [
            Case {
                args: &["rwc", "main.ts"],
                want: CommandCase {
                    paths: &["main.ts"],
                    is_help: false,
                    is_line: false,
                    is_word: false,
                    is_byte: false,
                },
            },
            Case {
                args: &["rwc", "-l", "-w", "-c", "main.ts"],
                want: CommandCase {
                    paths: &["main.ts"],
                    is_help: false,
                    is_line: true,
                    is_word: true,
                    is_byte: true,
                },
            },
            Case {
                args: &["rwc", "--line", "--word", "--byte", "main.ts"],
                want: CommandCase {
                    paths: &["main.ts"],
                    is_help: false,
                    is_line: true,
                    is_word: true,
                    is_byte: true,
                },
            },
            Case {
                args: &["rwc", "--help", "main.ts"],
                want: CommandCase {
                    paths: &["main.ts"],
                    is_help: true,
                    is_line: false,
                    is_word: false,
                    is_byte: false,
                },
            },
            Case {
                args: &["rwc", "main.ts", "try.ts"],
                want: CommandCase {
                    paths: &["main.ts", "try.ts"],
                    is_help: false,
                    is_line: false,
                    is_word: false,
                    is_byte: false,
                },
            },
        ];

        for case in cases {
            let args: Vec<String> = case.args.iter().map(|&s| s.to_string()).collect();
            let got = parse(&args).unwrap();

            assert_eq!(got.paths.len(), case.want.paths.len());

            for (got_path, want_path) in got.paths.iter().zip(case.want.paths.iter()) {
                assert_eq!(got_path, want_path);
            }

            assert_eq!(got.is_help, case.want.is_help);
            assert_eq!(got.is_line, case.want.is_line);
            assert_eq!(got.is_word, case.want.is_word);
            assert_eq!(got.is_byte, case.want.is_byte);
        }
    }
}
