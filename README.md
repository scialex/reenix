# Reenix

This is the start of a unix like operating system written in [Rust].  It is
based on the [Weenix] Operating system written for [Brown's CS167/9].  At the
moment it supports a basic kernel shell, mutiple processes with waitpid, and
writing to disk.  This was written as part of my CS Senior Thesis (To be posted
soon).

[Rust]: https://github.com/rust-lang/rust/
[Brown's CS167/9]: http://cs.brown.edu/courses/cs167/
<!--[CS Senior Thesis]: -->

## Building

This is only tested on Debian 7.8.

1. Build Requirements:

    * GCC (I use 4.9.0)
    * Rust (Version in external/rust should work)
    * qemu
    * python
    * make
    * grub-mkrescue
    * xorriso

2. Build Commands:

    * make

3. Run
    * ./weenix

## TODO

* Clean up the numerous sharp edges.
* Get VFS working
* Start making S5FS
* Get a userspace
