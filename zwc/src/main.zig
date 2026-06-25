const std = @import("std");
const Allocator = std.mem.Allocator;
const ArrayList = std.ArrayList;
const Io = std.Io;
const Dir = Io.Dir;
const File = Io.File;

const eql = std.mem.eql;
const expectEqual = std.testing.expectEqual;
const expectError = std.testing.expectError;
const expectEqualSlices = std.testing.expectEqualSlices;
const startsWith = std.mem.startsWith;

const S_ERROR_FORMAT = "Error: {}\n";
const AppError = error{EmptyFilePath};
const DelimiterError = Io.Reader.DelimiterError;
const ParserError = error{UnknownFlag};

pub fn main(init: std.process.Init) !void {
    const io = init.io;
    const arena = init.arena.allocator();

    const stdout_buffer = try arena.alloc(u8, 1024);

    var stdout_writer = File.stdout().writer(io, stdout_buffer);
    const stdout = &stdout_writer.interface;

    // Start App
    const z_args = try init.minimal.args.toSlice(arena);
    const o_command = parse(z_args[1..]) catch |err|
        return std.debug.print(S_ERROR_FORMAT, .{err});

    if (o_command.is_help)
        return {
            try stdout.writeAll(S_HELP_MESSAGE);
            try stdout.flush();
        };

    var o_path_iter: Path = .init(z_args[1..]);
    var o_results = try read(arena, io, &o_path_iter);

    if (o_results.items.len > 1) {
        try o_results.append(arena, countTotal(o_results.items));
    }

    try printOut(stdout, o_results.items, o_command, &o_path_iter);
}

const Path = struct {
    z_args: []const []const u8,
    n_current: usize,

    pub fn init(source: []const []const u8) Path {
        return .{ .z_args = source, .n_current = 0 };
    }

    fn isValid(_: Path, path: []const u8) bool {
        return !startsWith(u8, path, "-");
    }

    pub fn len(self: *Path) usize {
        var count: usize = 0;
        while (self.next()) |_| {
            count += 1;
        }
        self.n_current = 0;
        return count;
    }

    pub fn next(self: *Path) ?[]const u8 {
        if (self.n_current >= self.z_args.len) {
            return null;
        }

        var path = self.z_args[self.n_current];
        if (self.isValid(path)) {
            self.n_current += 1;
            return path;
        }

        while (true) {
            self.n_current += 1;

            if (self.n_current >= self.z_args.len) {
                return null;
            }

            path = self.z_args[self.n_current];
            if (self.isValid(path)) {
                self.n_current += 1;
                return path;
            }
        }
    }

    pub fn reset(self: *Path) void {
        self.n_current = 0;
    }
};

const Command = struct {
    is_help: bool,
    is_line: bool,
    is_word: bool,
    is_byte: bool,

    pub const empty = Command{
        .is_help = false,
        .is_line = false,
        .is_word = false,
        .is_byte = false,
    };
};

fn parse(z_args: []const []const u8) ParserError!Command {
    var o_command: Command = .empty;

    for (z_args) |s_arg| {
        if (!startsWith(u8, s_arg, "-")) {
            continue;
        } else if (eql(u8, s_arg, "--help")) {
            o_command.is_help = true;
        } else if (eql(u8, s_arg, "-l") or eql(u8, s_arg, "--line")) {
            o_command.is_line = true;
        } else if (eql(u8, s_arg, "-w") or eql(u8, s_arg, "--word")) {
            o_command.is_word = true;
        } else if (eql(u8, s_arg, "-c") or eql(u8, s_arg, "--byte")) {
            o_command.is_byte = true;
        } else {
            return ParserError.UnknownFlag;
        }
    }

    return o_command;
}

test "parse" {
    const Case = struct {
        a_args: []const []const u8,
        o_want: Command,
    };

    const normal_command: Command = .{
        .is_help = false,
        .is_line = true,
        .is_word = true,
        .is_byte = true,
    };

    const help_command: Command = .{
        .is_help = true,
        .is_line = false,
        .is_word = false,
        .is_byte = false,
    };

    const a_cases = [_]Case{
        .{
            .a_args = &.{ "zwc", "main.ts" },
            .o_want = .empty,
        },
        .{
            .a_args = &.{ "zwc", "-l", "-w", "-c", "main.ts" },
            .o_want = normal_command,
        },
        .{
            .a_args = &.{ "zwc", "--line", "--word", "--byte", "main.ts" },
            .o_want = normal_command,
        },
        .{
            .a_args = &.{ "zwc", "--help", "main.ts", "--help" },
            .o_want = help_command,
        },
        .{
            .a_args = &.{ "zwc", "main.ts", "try.ts" },
            .o_want = .empty,
        },
    };

    // Succeeded Cases
    for (a_cases) |o_case| {
        const got = try parse(o_case.a_args);

        inline for (std.meta.fields(Command)) |field| {
            if (comptime !eql(u8, field.name, "z_paths")) {
                try expectEqual(
                    @field(o_case.o_want, field.name),
                    @field(got, field.name),
                );
            }
        }
    }

    // An Error Case
    const e_err = parse(&.{ "zwc", "-a", "main.ts" });
    try expectError(ParserError.UnknownFlag, e_err);
}

fn countPaths(z_args: []const []const u8) usize {
    var n_paths: usize = 0;

    for (z_args) |s_arg| {
        if (!startsWith(u8, s_arg, "-")) {
            n_paths += 1;
        }
    }

    return n_paths;
}

test "sizeFilePath" {
    const a_args: []const []const u8 = &.{
        "-l",
        "-w",
        "main.ts",
        "try.ts",
        "maybe.file",
    };

    const got = countPaths(a_args);
    try expectEqual(a_args[2..].len, got);
}

const Count = struct {
    n_line: usize,
    n_word: usize,
    n_byte: usize,

    pub const empty: Count = .{
        .n_line = 0,
        .n_word = 0,
        .n_byte = 0,
    };
};

fn countWord(io: Io, z_buffer: []u8, o_file: File) DelimiterError!Count {
    var o_reader = o_file.reader(io, z_buffer);
    return try readCount(&o_reader.interface);
}

fn readCount(o_interface: *Io.Reader) DelimiterError!Count {
    var o_count: Count = .empty;

    while (true) {
        const s_line = o_interface.takeDelimiterInclusive('\n') catch |err|
            switch (err) {
                error.EndOfStream => break,
                else => return err,
            };

        o_count.n_line += 1;
        o_count.n_byte += s_line.len;

        var appeared = false;
        for (s_line) |c_line| {
            if ((c_line == ' ' or c_line == '\n') and appeared) {
                o_count.n_word += 1;
                appeared = false;
            }

            if (c_line != ' ' and !appeared) {
                appeared = true;
            }
        }
    }

    return o_count;
}

test "countWord" {
    const s_file =
        \\  console.log("hello, world");
        \\  function greet(name?: string) {
        \\    if (name === undefined) {
        \\      name = "world";
        \\    }
        \\    console.log(`Hello, ${name}`);
        \\  }
        \\
        \\ greet();
        \\ greet("John");
        \\
    ;

    var reader = Io.Reader.fixed(&s_file.*);
    const got = try readCount(&reader);
    const want: Count = .{
        .n_line = 10,
        .n_word = 20,
        .n_byte = 189,
    };

    inline for (std.meta.fields(Count)) |field| {
        try expectEqual(
            @field(want, field.name),
            @field(got, field.name),
        );
    }
}

fn countTotal(z_results: []anyerror!Count) Count {
    var o_total: Count = .empty;

    for (z_results) |e_result| {
        const o_count = e_result catch continue;
        o_total.n_line += o_count.n_line;
        o_total.n_word += o_count.n_word;
        o_total.n_byte += o_count.n_byte;
    }

    return o_total;
}

test "countTotal" {
    var z_results = [_]anyerror!Count{
        ParserError.UnknownFlag,
        Count{ .n_line = 1, .n_word = 2, .n_byte = 10 },
        Count{ .n_line = 1, .n_word = 2, .n_byte = 10 },
    };

    const got = countTotal(&z_results);
    const want: Count = .{
        .n_line = 2,
        .n_word = 4,
        .n_byte = 20,
    };

    inline for (std.meta.fields(Count)) |field| {
        try expectEqual(
            @field(want, field.name),
            @field(got, field.name),
        );
    }
}

fn read(
    gpa: Allocator,
    io: Io,
    o_path: *Path,
) (Allocator.Error || Io.Cancelable)!ArrayList(anyerror!Count) {
    const n_paths = o_path.len();

    var o_list: ArrayList(anyerror!Count) = try .initCapacity(
        gpa,
        if (n_paths > 1) n_paths + 1 else 1,
    );
    errdefer o_list.deinit(gpa);

    const z_buffer = try gpa.alloc(u8, 4096);
    defer gpa.free(z_buffer);

    if (!try File.stdin().isTty(io) and n_paths == 0) {
        try o_list.append(gpa, countWord(io, z_buffer, File.stdin()));
    } else if (n_paths == 0) {
        try o_list.append(gpa, AppError.EmptyFilePath);
    } else {
        // Read files by iterating paths
        const cwd = Dir.cwd();
        while (o_path.next()) |s_path| {
            const o_file = cwd.openFile(io, s_path, .{}) catch |err| {
                try o_list.append(gpa, err);
                continue;
            };
            defer o_file.close(io);
            try o_list.append(gpa, countWord(io, z_buffer, o_file));
        }
        o_path.reset(); // Reset path tracer.
    }

    return o_list;
}

test "read - empty paths error when stdin is a Tty" {
    const gpa = std.testing.allocator;
    const io = std.testing.io;

    var path: Path = .init(&.{});

    var result = try read(gpa, io, &path);
    defer result.deinit(gpa);

    try expectEqual(@as(usize, 1), result.items.len);

    const expected_err = error.EmptyFilePath;
    try expectError(expected_err, result.items[0]);
}

test "read - valid file iteration and error handling" {
    const gpa = std.testing.allocator;
    const io = std.testing.io;

    const test_filename = "test_temp_file.txt";
    const cwd = std.Io.Dir.cwd();

    const file = try cwd.createFile(io, test_filename, .{});
    try file.writeStreamingAll(io, "Hello Zig world!\n");
    file.close(io);

    defer cwd.deleteFile(io, test_filename) catch {};

    var path: Path = .init(&.{ test_filename, "non_existent_file.xyz" });

    var result = try read(gpa, io, &path);
    defer result.deinit(gpa);

    try std.testing.expectEqual(2, result.items.len);

    const first = try result.items[0];
    const want: Count = .{ .n_line = 1, .n_word = 3, .n_byte = 17 };
    inline for (std.meta.fields(Count)) |field| {
        try expectEqual(@field(want, field.name), @field(first, field.name));
    }

    const second = result.items[1];
    try expectError(error.FileNotFound, second);
}

fn printOut(
    writer: *Io.Writer,
    z_results: []anyerror!Count,
    o_command: Command,
    o_path: *Path,
) error{WriteFailed}!void {
    const s_fmt = "{:>[1]}";
    const n_max = maxDigit(z_results);
    const s_default = if (z_results.len > 1) "total" else "";

    for (z_results) |e_result| {
        const s_path = o_path.next() orelse s_default;
        if (e_result) |o_count| {
            const t_line = .{ o_count.n_line, n_max };
            const t_word = .{ o_count.n_word, n_max + 1 };
            const t_byte = .{ o_count.n_byte, n_max + 1 };

            if (o_command.is_line) try writer.print(s_fmt, t_line);
            if (o_command.is_word) try writer.print(s_fmt, t_word);
            if (o_command.is_byte) try writer.print(s_fmt, t_byte);

            // No Flags
            if (!o_command.is_line and
                !o_command.is_word and
                !o_command.is_byte)
            {
                try writer.print(s_fmt, t_line);
                try writer.print(s_fmt, t_word);
                try writer.print(s_fmt, t_byte);
            }

            try writer.print(" {s}\n", .{s_path});
        } else |err| {
            try writer.print(S_ERROR_FORMAT, .{err});
        }
    }

    try writer.flush();
}

test "printOut" {
    const gpa = std.testing.allocator;

    const o_command: Command = .empty;
    var path: Path = .init(&.{
        "main.ts",
        "try.ts",
        "notFound.file",
        "total",
    });

    var a_results = [_]anyerror!Count{
        Count{ .n_line = 1, .n_word = 12, .n_byte = 123 },
        Count{ .n_line = 1, .n_word = 12, .n_byte = 123 },
        error.FileNotFound,
        Count{ .n_line = 2, .n_word = 24, .n_byte = 246 },
    };

    var stream: Io.Writer.Allocating = try .initCapacity(gpa, 1024);
    defer stream.deinit();

    try printOut(&stream.writer, &a_results, o_command, &path);

    const written: []u8 = stream.written();
    const s_want: []const u8 =
        \\  1  12 123 main.ts
        \\  1  12 123 try.ts
        \\Error: error.FileNotFound
        \\  2  24 246 total
        \\
    ;
    try expectEqualSlices(u8, s_want, written);
}

fn countDigit(n_num: usize) usize {
    if (n_num < 10) return 1;

    const f_log: f64 = @log10(@floatFromInt(n_num));
    return @ceil(f_log);
}

test "countDigit" {
    const Case = struct {
        n_num: usize,
        n_want: usize,
    };

    const a_cases = &[_]Case{
        Case{ .n_num = 1, .n_want = 1 },
        Case{ .n_num = 12, .n_want = 2 },
        Case{ .n_num = 123, .n_want = 3 },
    };

    for (a_cases) |o_case| {
        try expectEqual(o_case.n_want, countDigit(o_case.n_num));
    }
}

fn maxDigit(z_resutls: []anyerror!Count) usize {
    var max: usize = 0;

    for (z_resutls) |e_result| {
        const o_count = e_result catch continue;
        max = @max(max, countDigit(o_count.n_line));
        max = @max(max, countDigit(o_count.n_word));
        max = @max(max, countDigit(o_count.n_byte));
    }

    return max;
}

test "maxDigit" {
    var a_results = [_]anyerror!Count{
        ParserError.UnknownFlag,
        Count{ .n_line = 1, .n_word = 12, .n_byte = 123 },
    };
    try expectEqual(3, maxDigit(&a_results));
}

const S_HELP_MESSAGE =
    \\ Usage: zwc [OPTION]... [FILE]...
    \\
    \\ Options:
    \\    -c, --byte
    \\        print the byte counts
    \\    -w, --word
    \\        print the word counts
    \\    -l, --line
    \\        print the newline counts
    \\    --help
    \\        display this help and exit
    \\
    \\
;
