bassert
=======

# A better `assert!` for Rust: `bassert!`

This macro eliminates the need for macros like `assert_eq` and friends
by parsing the AST manually to determine what type of printing should
be done. It also does things that no macro could ever do, like printing
out the arguments of a function that is being called.

## Usage

```
#![feature(phase)]
#[phase(plugin)] extern crate bassert;

fn main() {
    if cfg!(first) {
        bassert!((1u8 + 1) << 2 < 3, "We are {} with math.", "AMAZING");
    } else {
        bassert!(stuff(123, "hello world"), "NOOO NOT STUFF!");
    }
}

fn stuff(i: u32, j: &'static str) -> bool {
    // STUFF
    false
}
```

Running this with `--cfg first` causes it to print:
```
task '<main>' panicked at 'assertion failed: ( 1u8 + 1 ) << 2 < 3:
left:  `( 1u8 + 1 ) << 2` = `8`
right: `3` = `3`
We are AMAZING with math.', test.rs:6
```
and running it without that config causes it to print
```
task '<main>' panicked at 'assertion failed: stuff ( 123 , "hello world" ):
argument 0: 123
argument 1: hello world
NOOO NOT STUFF!', test.rs:8
```

## Acknowledgments
This was originally based upon [P1Start's assert\_ng](https://github.com/P1start/assert_ng)
although I rewrote so much of it that there is basically nothing left. The
tests have largely remained the same but there is little of lib.rs that is from
that repository.
