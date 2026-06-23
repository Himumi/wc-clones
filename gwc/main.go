package main

import (
	"bufio"
	"errors"
	"fmt"
	"io"
	"math"
	"os"
	"strings"

	"golang.org/x/term"
)

func main() {
	command, err := Parse(os.Args)
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		return
	}

	if command.IsHelp {
		fmt.Println("HELP")
		return
	}

	counts := Read(command.Paths)
	if len(counts) > 1 {
		counts = append(counts, Total(counts))
		command.Paths = append(command.Paths, "total")
	}

	if err := PrintOut(os.Stdout, counts, command); err != nil {
		fmt.Fprintln(os.Stderr, err)
		return
	}
}

func Read(paths []string) []Count {
	countLen := len(paths)
	if countLen == 0 {
		countLen = 1
	}

	counts := make([]Count, 0, countLen)
	reader := bufio.NewReader(nil)

	if !term.IsTerminal(int(os.Stdin.Fd())) && len(paths) == 0 {
		reader.Reset(os.Stdin)

		c, err := CountWord(reader)
		if err != nil {
			c.Error = err
		}

		counts = append(counts, c)
	} else {
		if len(paths) == 0 {
			err := Count{Error: errors.New("empty file path")}
			counts = append(counts, err)
		} else {
			for _, path := range paths {
				var c Count

				if f, err := os.Open(path); err != nil {
					c.Error = err
				} else {
					reader.Reset(f)
					defer f.Close()

					if c, err = CountWord(reader); err != nil {
						c.Error = err
					}
				}
				counts = append(counts, c)
			}
		}
	}

	return counts
}

type Command struct {
	Paths  []string
	IsHelp bool
	IsLine bool
	IsWord bool
	IsByte bool
}

func Parse(args []string) (Command, error) {
	c := Command{
		Paths: make([]string, 0, 1),
	}

	for _, arg := range args[1:] {
		if !strings.HasPrefix(arg, "-") {
			c.Paths = append(c.Paths, arg)
		} else if arg == "--help" {
			c.IsHelp = true
		} else if arg == "-l" || arg == "--line" {
			c.IsLine = true
		} else if arg == "w" || arg == "--word" {
			c.IsWord = true
		} else if arg == "-c" || arg == "--byte" {
			c.IsByte = true
		} else {
			return c, errors.New("parse: unknown flag")
		}
	}

	return c, nil
}

type Count struct {
	Error error
	Line  int
	Word  int
	Byte  int
}

func CountWord(reader *bufio.Reader) (Count, error) {
	c := Count{}
	for {
		line, err := reader.ReadString('\n')
		if err != nil {
			if err == io.EOF {
				break
			}
			return c, err
		}

		c.Line += 1
		c.Byte += len(line)
		c.Word += word(line)
	}

	return c, nil
}

func word(line string) int {
	var total int
	var appeared bool

	for _, char := range line {
		if (char == ' ' || char == '\n') && appeared {
			total += 1
			appeared = false
		}

		if char != ' ' && !appeared {
			appeared = true
		}
	}

	return total
}

func Total(counts []Count) Count {
	total := Count{}

	for _, count := range counts {
		if count.Error == nil {
			total.Line += count.Line
			total.Word += count.Word
			total.Byte += count.Byte

		}
	}

	return total
}

func PrintOut(base io.Writer, counts []Count, command Command) error {
	w := bufio.NewWriter(base)
	max := maxDigit(counts)

	for i, c := range counts {
		if c.Error != nil {
			if _, err := fmt.Fprintf(w, "%s\n", c.Error); err != nil {
				return err
			}
		} else {
			if command.IsLine {
				if _, err := fmt.Fprintf(w, "%*d", max, c.Line); err != nil {
					return err
				}
			}
			if command.IsWord {
				if _, err := fmt.Fprintf(w, "%*d", max+1, c.Word); err != nil {
					return err
				}
			}
			if command.IsByte {
				if _, err := fmt.Fprintf(w, "%*d", max+1, c.Byte); err != nil {
					return err
				}
			}

			if !command.IsLine && !command.IsWord && !command.IsByte {
				if _, err := fmt.Fprintf(w, "%*d", max, c.Line); err != nil {
					return err
				}
				if _, err := fmt.Fprintf(w, "%*d", max+1, c.Word); err != nil {
					return err
				}
				if _, err := fmt.Fprintf(w, "%*d", max+1, c.Byte); err != nil {
					return err
				}
			}

			path := ""
			if i < len(command.Paths) {
				path = command.Paths[i]
			}

			if _, err := fmt.Fprintf(w, " %s\n", path); err != nil {
				return err
			}
		}
	}

	if err := w.Flush(); err != nil {
		return err
	}

	return nil
}

func digit(num int) float64 {
	if num == 0 || num > 0 && num < 10 {
		return 1
	}

	log := math.Log10(float64(num))
	return math.Ceil(log)
}

func maxDigit(counts []Count) int {
	var max float64
	for _, count := range counts {
		max = math.Max(max, digit(count.Line))
		max = math.Max(max, digit(count.Word))
		max = math.Max(max, digit(count.Byte))
	}

	return int(max)
}
