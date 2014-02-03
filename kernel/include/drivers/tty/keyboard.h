#pragma once

typedef void (*keyboard_char_handler_t)(char);

/**
 * Initializes the keyboard subsystem.
 */
void keyboard_init(void);

/**
 * Registers a handler to receive key press events from the keyboard.
 *
 * @param handler the handler to register
 */
void keyboard_register_handler(keyboard_char_handler_t handler);
