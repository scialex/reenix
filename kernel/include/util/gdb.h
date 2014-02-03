#pragma once

#define GDB_DEFINE_HOOK(name, ...) \
        void __py_hook_ ## name ( __VA_ARGS__ ) {}
#define GDB_CALL_HOOK(name, ...) \
        __py_hook_ ## name ( __VA_ARGS__ )
