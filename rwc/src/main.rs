use std::env;
use std::fmt;
use std::fs;
use std::io;
use std::io::{BufRead, BufReader, IsTerminal, Read, Write, stdin};

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

    let stdout = io::stdout();
    let mut lock = stdout.lock();

    print_out(&mut lock, &counts, &command);
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

impl std::error::Error for ParseError {}

#[derive(Debug)]
enum AppError {
    EmptyFilePath,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::EmptyFilePath => {
                write!(f, "Empty File Path")
            }
        }
    }
}

impl std::error::Error for AppError {}

impl From<AppError> for io::Error {
    fn from(err: AppError) -> Self {
        let kind = match err {
            AppError::EmptyFilePath => io::ErrorKind::Other,
        };
        io::Error::new(kind, err)
    }
}

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

#[derive(Default, Debug, Clone)]
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

fn read_file<T: AsRef<str>>(paths: &[T]) -> Vec<io::Result<Count>> {
    let mut buffer = String::with_capacity(4096);

    let mut result: Vec<io::Result<Count>> =
        Vec::with_capacity(if paths.len() > 1 { paths.len() + 1 } else { 1 });

    if !stdin().is_terminal() && paths.len().eq(&0) {
        result.push(count_word(stdin(), &mut buffer));
    } else {
        if paths.len().eq(&0) {
            result.push(Err(io::Error::from(AppError::EmptyFilePath)));
        }

        for path in paths.iter() {
            let path_ref = path.as_ref();
            match fs::File::open(path_ref) {
                Ok(value) => result.push(count_word(value, &mut buffer)),
                Err(err) => result.push(Err(err)),
            };
        }
    }

    result
}

fn print_out(writer: &mut impl Write, results: &[io::Result<Count>], command: &Command) {
    let digit_max = max_digit(results);

    for (i, result_union) in results.iter().enumerate() {
        match result_union {
            Err(err) => writeln!(writer, " {}", err).unwrap(),
            Ok(count) => {
                if command.is_line {
                    write!(writer, "{:1$}", count.line, digit_max).unwrap();
                }
                if command.is_word {
                    write!(writer, "{:1$}", count.word, digit_max + 1).unwrap();
                }
                if command.is_byte {
                    write!(writer, "{:1$}", count.byte, digit_max + 1).unwrap();
                }

                // No Flags
                if !command.is_line && !command.is_word && !command.is_byte {
                    write!(writer, "{:1$}", count.line, digit_max).unwrap();
                    write!(writer, "{:1$}", count.word, digit_max + 1).unwrap();
                    write!(writer, "{:1$}", count.byte, digit_max + 1).unwrap();
                }

                match command.paths.get(i) {
                    Some(value) => writeln!(writer, " {}", value).unwrap(),
                    None => writeln!(writer).unwrap(),
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
    use core::panic;
    use std::io::Write;

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

    #[test]
    fn test_fail_read_file() {
        let mut got = read_file::<&str>(&[]);
        assert_eq!(got.len(), 1);

        if let Some(first) = got.pop() {
            let io_err = first.unwrap_err();
            if let Some(inner_err) = io_err.into_inner() {
                match inner_err.downcast::<AppError>() {
                    Ok(my_err) => {
                        assert!(matches!(*my_err, AppError::EmptyFilePath));
                    }
                    Err(_) => {
                        panic!("The inner error was not an AppError type");
                    }
                }
            }
        } else {
            panic!("Could not get the first item");
        }
    }

    #[test]
    fn test_succeed_read_file() {
        let file_name = "./test.ts";
        {
            let mut file = fs::File::create(file_name).expect("Could not to create the file");
            file.write_all(b"console.log('Hello, world');\n")
                .expect("Could not to write data to the file");
            file.flush().expect("Could not to flush the data");
        }

        let mut got = read_file(&[file_name, "notFound.file"]);
        assert_eq!(got.len(), 2);

        let second = got.pop().unwrap();
        assert!(second.is_err());

        let first = got.pop().unwrap();
        match first {
            Ok(item) => {
                assert_eq!(item.line, 1);
                assert_eq!(item.word, 2);
                assert_eq!(item.byte, 29);
            }
            Err(err) => {
                panic!("the second item is an error: {}", err);
            }
        }

        fs::remove_file(file_name).expect("Could not delete the file");
    }

    #[test]
    fn test_print_out() {
        let mut buffer = Vec::new();
        let command = Command {
            paths: ["test.ts", "try.ts", "notFound.file"]
                .into_iter()
                .map(String::from)
                .collect(),
            is_help: false,
            is_line: false,
            is_word: false,
            is_byte: false,
        };

        let resutls: [io::Result<Count>; 3] = [
            Ok(Count {
                line: 1,
                word: 12,
                byte: 123,
            }),
            Ok(Count {
                line: 1,
                word: 12,
                byte: 123,
            }),
            Err(io::Error::from(AppError::EmptyFilePath)),
        ];
        print_out(&mut buffer, &resutls, &command);

        let want = "  1  12 123 test.ts\n  1  12 123 try.ts\n Empty File Path\n";
        let got = String::from_utf8(buffer).expect("Could not get string from buffer");
        assert_eq!(got, want);
    }
}
