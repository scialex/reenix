#pragma once

#include "types.h"

#include "test/kshell/kshell.h"

typedef enum kshell_token_type {
        KTT_WORD,
        KTT_REDIRECT_IN,         /* '<' */
        KTT_REDIRECT_OUT,        /* '>' */
        KTT_REDIRECT_OUT_APPEND, /* '>>' */
        KTT_EOL,

        KTT_MAX /* Number of token types */
} kshell_token_type_t;

typedef struct kshell_token {
        kshell_token_type_t kt_type;
        char *kt_text;
        size_t kt_textlen;
} kshell_token_t;

/**
 * Finds the next token in the input line.
 *
 * Note: To find multiple tokens from the same line, you increment the
 * line pointer by the number of bytes processed before the next call
 * to kshell_next token.
 *
 * @param ksh the kshell
 * @param line the input line to tokenize
 * @param token out parameter containing the next token found
 * @return 0 if no more tokens, otherwise, number of bytes processed
 */
int kshell_next_token(kshell_t *ksh, char *line, kshell_token_t *token);

const char *kshell_token_type_str(kshell_token_type_t type);
