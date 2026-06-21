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

	if command.Flags.IsHelp {
		fmt.Println("HELP")
		return
	}

	countLen := len(command.Paths)
	if countLen == 0 {
		countLen = 1
	}
	counts := make([]Count, 0, countLen)

	if !term.IsTerminal(int(os.Stdin.Fd())) && len(command.Paths) == 0 {
		c, err := CountWord(os.Stdin, "")
		if err != nil {
			c.Error = err
		}
		counts = append(counts, c)
	} else {
		if len(command.Paths) == 0 {
			fmt.Fprintln(os.Stderr, "file: empty file path")
			return
		}

		for _, path := range command.Paths {
			var c Count

			if f, err := os.Open(path); err != nil {
				c.Error = err
			} else {
				if c, err = CountWord(f, path); err != nil {
					c.Error = err
				}

				if err := f.Close(); err != nil {
					fmt.Fprintln(os.Stderr, err)
					return
				}
			}
			counts = append(counts, c)
		}
	}

	if len(counts) > 1 {
		total := Total(counts)
		counts = append(counts, total)
	}

	message, err := Print(counts, command)
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		return
	}

	fmt.Print(message)
}

type Command struct {
	Paths []string
	Flags Flags
}

type Flags struct {
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
		} else if strings.EqualFold(arg, "--help") {
			c.Flags.IsHelp = true
		} else if strings.EqualFold(arg, "-l") || strings.EqualFold(arg, "--line") {
			c.Flags.IsLine = true
		} else if strings.EqualFold(arg, "-w") || strings.EqualFold(arg, "--word") {
			c.Flags.IsWord = true
		} else if strings.EqualFold(arg, "-c") || strings.EqualFold(arg, "--byte") {
			c.Flags.IsByte = true
		} else {
			return c, errors.New("parse: unknown flag")
		}
	}

	return c, nil
}

type Count struct {
	Path  string
	Error error
	Line  int
	Word  int
	Byte  int
}

func (c *Count) Len(command Command, max int) int {
	const (
		space     = 1
		lineBreak = 1
	)

	var total int
	if c.Error != nil {
		total += space + len(c.Error.Error()) + lineBreak
		return total
	}

	if command.Flags.IsLine {
		total += max
	}

	if command.Flags.IsWord {
		total += space + max
	}

	if command.Flags.IsByte {
		total += space + max
	}

	if !command.Flags.IsLine &&
		!command.Flags.IsWord &&
		!command.Flags.IsByte {
		total += max*3 + space*2
	}
	total += space + len(c.Path) + lineBreak

	return total
}

func CountWord(f *os.File, path string) (Count, error) {
	c := Count{
		Path: path,
	}
	reader := bufio.NewReaderSize(f, 1024)

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
	total := Count{Path: "total"}

	for _, count := range counts {
		if count.Error == nil {
			total.Line += count.Line
			total.Word += count.Word
			total.Byte += count.Byte

		}
	}

	return total
}

const fmtString = " %s\n"

func Print(counts []Count, command Command) (string, error) {
	max := maxDigit(counts)
	length := lenCounts(counts, command, max)

	var s strings.Builder
	s.Grow(length)

	for _, c := range counts {
		if c.Error != nil {
			if _, err := fmt.Fprintf(&s, fmtString, c.Error); err != nil {
				return "", err
			}
		} else {
			countStr, err := print(c, command, max)
			if err != nil {
				return "", err
			}

			if _, err := s.WriteString(countStr); err != nil {
				return "", err
			}
		}
	}

	return s.String(), nil
}

func print(count Count, command Command, max int) (string, error) {
	length := count.Len(command, max)

	line := fmt.Sprintf(genNumFmt(count.Line, max), count.Line)
	word := fmt.Sprintf(genNumFmt(count.Word, max+1), count.Word)
	byte := fmt.Sprintf(genNumFmt(count.Byte, max+1), count.Byte)

	var s strings.Builder
	s.Grow(length)

	if command.Flags.IsLine {
		if _, err := s.WriteString(line); err != nil {
			return "", err
		}
	}

	if command.Flags.IsWord {
		if _, err := s.WriteString(word); err != nil {
			return "", err
		}
	}

	if command.Flags.IsByte {
		if _, err := s.WriteString(byte); err != nil {
			return "", err
		}
	}

	if !command.Flags.IsLine && !command.Flags.IsWord && !command.Flags.IsByte {
		if _, err := s.WriteString(line); err != nil {
			return "", err
		}
		if _, err := s.WriteString(word); err != nil {
			return "", err
		}
		if _, err := s.WriteString(byte); err != nil {
			return "", err
		}
	}

	if _, err := fmt.Fprintf(&s, fmtString, count.Path); err != nil {
		return "", err
	}

	return s.String(), nil
}

func genNumFmt(num int, max int) string {
	d := int(digit(num))
	length := max - d + 1

	var s strings.Builder
	s.Grow(length)

	for range max - d {
		s.WriteString(" ")
	}
	s.WriteString("%d")

	return s.String()
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

func lenCounts(cs []Count, c Command, max int) int {
	var total int
	for _, count := range cs {
		total += count.Len(c, max)
	}

	return total
}
