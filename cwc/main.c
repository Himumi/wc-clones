#include <iso646.h>
#include <math.h>
#include <stdatomic.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

const char *ERR_EMPTY_FILE_PATH = "error: empty file path";

typedef struct Command {
  char **z_paths;
  int n_paths;
  bool is_help;
  bool is_line;
  bool is_word;
  bool is_byte;
} Command;

bool start_with(const char *str, const char *prefix) {
  return strncmp(str, prefix, strlen(prefix)) == 0;
}

char *errMessage(int n_err) {
  switch (n_err) {
  case -1:
    return "Error: empty file path";
  case -2:
    return "Error: file not found";
  case -3:
    return "Error: could not duplicate a string";
  case -4:
    return "Error: could not reallocate memory";
  case -5:
    return "Error: could not print the result";
  default:
    return "";
  }
}

int lenErrMessage(int n_err) { return strlen(errMessage(n_err)); }

// I need to write cleanup part.
int parse(int n_argc, char *z_argv[], Command *o_command) {
  int n_status = 0;

  for (int i = 1; i < n_argc; i++) {
    const char *s_arg = z_argv[i];
    if (!start_with(s_arg, "-")) {
      char *s_path = strdup(s_arg);
      if (s_path == NULL) {
        n_status = -3;
        goto parse_cleanup;
      }

      char **tmp;
      if (o_command->n_paths == 0) {
        tmp = malloc(sizeof(char *));
      } else {
        tmp = realloc(o_command->z_paths,
                      (o_command->n_paths + 1) * sizeof(char *));
      }

      if (tmp == NULL) {
        free(s_path);
        n_status = -4;
        goto parse_cleanup;
      }
      o_command->z_paths = tmp;
      o_command->z_paths[o_command->n_paths] = s_path;
      o_command->n_paths += 1;
    } else if (strcmp(s_arg, "--help") == 0) {
      o_command->is_help = true;
    } else if (strcmp(s_arg, "-l") == 0 || strcmp(s_arg, "--line") == 0) {
      o_command->is_line = true;
    } else if (strcmp(s_arg, "-w") == 0 || strcmp(s_arg, "--word") == 0) {
      o_command->is_word = true;
    } else if (strcmp(s_arg, "-c") == 0 || strcmp(s_arg, "--byte") == 0) {
      o_command->is_byte = true;
    } else {
      n_status = -1;
      goto parse_cleanup;
    }
  }

  return 0;

parse_cleanup:
  if (o_command->z_paths != NULL) {
    for (int i = 0; i < o_command->n_paths; i++) {
      free(o_command->z_paths[i]);
    }
    free(o_command->z_paths);
    o_command->z_paths = NULL;
    o_command->n_paths = 0;
  }

  return n_status;
}

typedef struct Count {
  int n_err;
  int n_line;
  int n_word;
  int n_byte;
} Count;

Count countWord(FILE *o_file) {
  size_t n_len = 0;
  char *s_line = NULL; // let the getline handles memory safely.

  const char C_SPACE = ' ';
  const char C_LINE_BREAK = '\n';

  Count o_count = {
      .n_err = 0,
      .n_line = 0,
      .n_word = 0,
      .n_byte = 0,
  };

  while (true) {
    ssize_t n_size = getline(&s_line, &n_len, o_file);
    if (n_size == -1) {
      break;
    }

    o_count.n_line += 1;
    o_count.n_byte += n_size;

    bool is_appeared = false;
    for (int i = 0; i < n_size; i++) {
      const char c_line = s_line[i];

      if ((c_line == C_SPACE || c_line == C_LINE_BREAK) && is_appeared) {
        o_count.n_word += 1;
        is_appeared = false;
      }

      if (c_line != C_SPACE && !is_appeared)
        is_appeared = true;
    }
  }

  free(s_line);
  return o_count;
}

int digit(int n_num) {
  if (n_num < 10) {
    return 1;
  }

  return (int)ceil(log10((double)n_num + 0.1));
}

int mathMax(int a, int b) { return (a > b) ? a : b; }

int maxDigit(Count *z_counts, int n_len) {
  int max = 0;

  for (int i = 0; i < n_len; i++) {
    Count o_count = z_counts[i];
    if (o_count.n_err != 0) {
      continue;
    } else {
      max = mathMax(max, digit(o_count.n_line));
      max = mathMax(max, digit(o_count.n_word));
      max = mathMax(max, digit(o_count.n_byte));
    }
  }

  return max;
}

int lenCounts(Count *z_counts, int n_counts, Command o_command, int n_max) {
  const int n_space = 1;
  const int n_line_break = 1;

  bool is_line = o_command.is_line;
  bool is_word = o_command.is_word;
  bool is_byte = o_command.is_byte;
  bool is_default = !is_line && !is_word && !is_byte;

  int n_total = 0;

  for (int i = 0; i < n_counts; i++) {
    Count o_count = z_counts[i];
    if (o_count.n_err < 0) {
      n_total += lenErrMessage(o_count.n_err) + n_line_break;
    } else {
      n_total += (is_line) ? n_max : 0;
      n_total += (is_word) ? n_space + n_max : 0;
      n_total += (is_byte) ? n_space + n_max : 0;

      n_total += (is_default) ? n_max * 3 + n_space * 2 : 0;

      int n_path = (i < o_command.n_paths) ? strlen(o_command.z_paths[i]) : 0;
      n_total += n_space + n_path + n_line_break;
    }
  }

  return n_total;
}

Count total(Count *z_counts, int n_counts) {
  Count o_total = {.n_err = 0, .n_line = 0, .n_word = 0, .n_byte = 0};

  for (int i = 0; i < n_counts; i++) {
    Count o_count = z_counts[i];
    if (o_count.n_err == 0) {
      o_total.n_line += o_count.n_line;
      o_total.n_word += o_count.n_word;
      o_total.n_byte += o_count.n_byte;
    }
  }

  return o_total;
}

int printNumber(char *s_message, int n_width, int n_num) {
  return sprintf(s_message, "%*d", n_width, n_num);
}

int printFilename(char *s_message, char *s_filename) {
  return sprintf(s_message, " %s\n", s_filename);
}

int printError(char *s_message, int n_err) {
  return sprintf(s_message, "%s\n", errMessage(n_err));
}

char *print(Count *z_counts, int n_counts, Command o_command) {
  int n_max = maxDigit(z_counts, n_counts);

  bool is_line = o_command.is_line;
  bool is_word = o_command.is_word;
  bool is_byte = o_command.is_byte;
  bool is_default = !is_line && !is_word && !is_byte;

  int n_start = 0;
  int n_len = lenCounts(z_counts, n_counts, o_command, n_max) + 1;

  char *s_message = malloc(sizeof(char) * n_len);
  if (s_message == NULL) {
    return s_message;
  }
  s_message[n_len - 1] = '\0'; // add null terminator.

  for (int i = 0; i < n_counts; i++) {
    Count o_count = z_counts[i];
    if (o_count.n_err < 0) {
      n_start += printError(s_message + n_start, o_count.n_err);
    } else {
      if (is_line)
        n_start += printNumber(s_message + n_start, n_max, o_count.n_line);
      if (is_word)
        n_start += printNumber(s_message + n_start, n_max + 1, o_count.n_word);
      if (is_byte)
        n_start += printNumber(s_message + n_start, n_max + 1, o_count.n_byte);

      if (is_default) {
        n_start += printNumber(s_message + n_start, n_max, o_count.n_line);
        n_start += printNumber(s_message + n_start, n_max + 1, o_count.n_word);
        n_start += printNumber(s_message + n_start, n_max + 1, o_count.n_byte);
      }

      char *s_filename = (i < o_command.n_paths) ? o_command.z_paths[i] : "";
      n_start += printFilename(s_message + n_start, s_filename);
    }
  }

  return s_message;
}

int printOut(Count *z_counts, int n_counts, Command o_command) {
  setvbuf(stdout, NULL, _IOFBF, 1024);
  int n_max = maxDigit(z_counts, n_counts);

  bool is_line = o_command.is_line;
  bool is_word = o_command.is_word;
  bool is_byte = o_command.is_byte;
  bool is_default = !is_line && !is_word && !is_byte;

  for (int i = 0; i < n_counts; i++) {
    Count o_count = z_counts[i];
    if (o_count.n_err < 0) {
      printf("%s\n", errMessage(o_count.n_err));
    } else {
      if (is_line)
        printf("%*d", n_max, o_count.n_line);
      if (is_word)
        printf("%*d", n_max + 1, o_count.n_word);
      if (is_byte)
        printf("%*d", n_max + 1, o_count.n_byte);

      if (is_default) {
        printf("%*d", n_max, o_count.n_line);
        printf("%*d", n_max + 1, o_count.n_word);
        printf("%*d", n_max + 1, o_count.n_byte);
      }

      char *s_filename = (i < o_command.n_paths) ? o_command.z_paths[i] : "";
      printf(" %s\n", s_filename);
    }
  }

  fflush(stdout);
  return 0;
}

int readFile(Count *z_counts, int n_counts, Command o_command) {
  if (!isatty(fileno(stdin)) && o_command.n_paths == 0) {
    Count o_count = countWord(stdin);
    z_counts[0] = o_count;
  } else {
    if (n_counts == 0) {
      return -1;
    }

    for (int i = 0; i < o_command.n_paths; i++) {
      Count o_count;

      FILE *f = fopen(o_command.z_paths[i], "r");
      if (f == NULL) {
        o_count.n_err = -2;
      } else {
        o_count = countWord(f);
        fclose(f);
      }

      z_counts[i] = o_count;
    }
  }

  return 0;
}

int main(int argc, char *argv[]) {
  Count *z_counts = NULL;
  char *s_message = NULL;
  int n_status = 0;
  Command o_command = {
      .z_paths = NULL,
      .n_paths = 0,
      .is_line = false,
      .is_word = false,
      .is_byte = false,
  };

  int n_err = parse(argc, argv, &o_command);
  if (n_err != 0) {
    n_status = n_err;
    goto cleanup;
  }

  if (o_command.is_help) {
    printf("HELP MESSAGE\n");
    n_status = 0;
    goto cleanup;
  }

  int n_counts = (o_command.n_paths > 1) ? o_command.n_paths + 1 : 1;

  z_counts = malloc(sizeof(Count) * n_counts);
  if (z_counts == NULL) {
    n_status = -2;
    goto cleanup;
  }

  n_err = readFile(z_counts, n_counts, o_command);
  if (n_err != 0) {
    n_status = n_err;
    goto cleanup;
  }

  if (n_counts > 1) {
    Count o_total = total(z_counts, n_counts);
    z_counts[n_counts - 1] = o_total;

    char *s_total = strdup("total");
    if (s_total == NULL) {
      n_status = -3;
      goto cleanup;
    }

    char **tmp =
        realloc(o_command.z_paths, (o_command.n_paths + 1) * sizeof(char *));
    if (tmp == NULL) {
      free(s_total);
      n_status = -2;
      goto cleanup;
    }
    o_command.z_paths = tmp;

    o_command.z_paths[o_command.n_paths] = s_total;
    o_command.n_paths += 1;
  }

  printOut(z_counts, n_counts, o_command);
  goto cleanup;

cleanup:
  if (o_command.z_paths != NULL) {
    for (int i = 0; i < o_command.n_paths; i++) {
      free(o_command.z_paths[i]);
    }
    free(o_command.z_paths);
  }

  if (s_message != NULL) {
    free(s_message);
  }

  if (z_counts != NULL) {
    free(z_counts);
  }

  return n_status;
}
