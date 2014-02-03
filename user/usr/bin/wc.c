#include <ctype.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#define BUFFER_SIZE 1024

typedef struct count_results {
    unsigned long long        n_chars;
    unsigned long long        n_words;
    unsigned long long        n_lines;
} count_results_t;

char buf[BUFFER_SIZE];

void
print_counts(count_results_t *results, char *name)
{
    if (name) {
        printf("%10llu %10llu %10llu %10s\n",
               results->n_lines,
               results->n_words,
               results->n_chars,
               name);
    } else {
        printf("%10llu %10llu %10llu\n",
               results->n_lines,
               results->n_words,
               results->n_chars);
    }

}

void
count(int fd, char *name, count_results_t *results)
{
    size_t bytes_read;
    unsigned int in_word, i;

    in_word = 0;
    while ((bytes_read = read(fd, buf, BUFFER_SIZE)) > 0)
    {
        for (i = 0; i < bytes_read; ++i) {
            if (isspace(buf[i])) {
                if (in_word) {
                    results->n_words++;
                    in_word = 0;
                }
            } else {
                in_word = 1;
            }

            if (buf[i] == '\n')
                results->n_lines++;
        }

        results->n_chars += bytes_read;
    }

    print_counts(results, name);
}

int
main(int argc, char **argv)
{
    int f, fd;
    count_results_t total_counts = { .n_chars = 0, .n_words = 0, .n_lines = 0 };
    count_results_t local_counts = { .n_chars = 0, .n_words = 0, .n_lines = 0 };

    if (argc == 1)
    {
        /* Reading from standard input. */
        count(0, 0, &total_counts);
    } else {
        /* Reading files, not standard input. */
        for (f = 1; f < argc; ++f)
        {
            fd = open(argv[f], O_RDONLY, 0);
            if (fd < 0) {
                /* Error opening file. */
                fprintf(stderr, "wc: %s: open: %s\n", argv[f], strerror(errno));
            } else {
                /* Opened the file. */
                count(fd, argv[f], &local_counts);

                total_counts.n_chars += local_counts.n_chars;
                total_counts.n_words += local_counts.n_words;
                total_counts.n_lines += local_counts.n_lines;

                /* Reset the local counts. */
                local_counts.n_chars = local_counts.n_words = local_counts.n_lines = 0;

                close(fd);
            }
        }

        if (argc > 2) {
            /* They provided multiple files. We should print the total too. */
            print_counts(&total_counts, "total");
        }
    }

    return 0;
}
