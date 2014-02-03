#pragma once
#include "types.h"

/**
 * Initialize the screen subsystem.
 */
void screen_init(void);

/**
 * Move the cursor to the given (x,y) coords (measured from top left).
 *
 * @param x the x coordinate
 * @param y the y coordinate
 */
void screen_move_cursor(uint8_t x, uint8_t y);

/**
 * Writes a character to the screen at the given position.
 *
 * @param c the character to display on the screen
 * @param x the x coordinate
 * @param y the y coordinate
 */
void screen_putchar(char c, uint8_t x, uint8_t y);

/**
 * Writes a character to the screen at a given position with the given
 * attributes.
 *
 * @param c the character to display on the screen
 * @param x the x coordinate
 * @param y the y coordinate
 * @param the attributes for the character
 */
void screen_putchar_attrib(char c, uint8_t x, uint8_t y, uint8_t attrib);

/**
 * Write a buffer of characters which is _EXACTLY_ DISPLAY_WIDTH x
 * DISPLAY_HEIGHT characters.
 *
 * @param buf the buffer to write to the screen
 */
void screen_putbuf(const char *buf);

/**
 * Write a buffer of characters which is _EXACTLY_ DISPLAY_WIDTH x
 * DISPLAY_HEIGHT characters and attributes.
 *
 * @param buf the buffer to write to the screen
 */
void screen_putbuf_attrib(const uint16_t *buf);

/**
 * Clear the screen.
 */
void screen_clear(void);
