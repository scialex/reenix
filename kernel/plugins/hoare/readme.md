# LibHoare

Simple Rust support for design by contract-style assertions. Supports
* preconditions (`precond`),
* postconditions (`postcond`),
* invariants (pre and post)  (`invariant`).

Each macro takes a predicate given as a string parameter. Each macro is
available in a `debug_` version which only checks the assertion in debug builds,
they should be zero overhead in non-debug builds. You can use `result` inside a
postcondition to get the value returned by the function.

Preconditions are checked on entry to a function. Postcondiitons are checked when
leaving the function by any path.

(The library is named for Tony, not Graydon. Or rather it is named for the logic
which was named after Tony).


## Using libhoare

Theoretically you can use libhoare with Cargo by adding 

```
[dependencies.hoare]
git = "https://github.com/nick29581/libhoare.git"
```

to your projects Cargo manifest. I haven't actually managed to get Cargo to work,
so I haven't tested this.

Otherwise, download this repo, build it (see build instructions below), make
sure the path to the compiled libhoare is on your library path in some way (one
way of doing this is to `export LD_LIBRARY_PATH=/path/to/libhoare/obj` before
building).

Then (whether or not you used Cargo), in your crate you will need the following
boilerplate:

```
#![feature(phase)]

#[phase(plugin)]
extern crate hoare;
```

Then you can use the macros as shown below.


## Examples:

```
#[precond="x > 0"]
#[postcond="result > 1"]
fn foo(x: int) -> int {
    let y = 45 / x;
    y + 1
}


struct Bar {
    f1: int,
    f2: int
}

#[invariant="x.f1 < x.f2"]
fn baz(x: &mut Bar) {
    x.f1 += 10;
    x.f2 += 10;
}

fn main() {
    foo(12);
    foo(26);
    // task '<main>' failed at 'precondition of foo (x > 0)'
    // foo(-3);

    let mut b = Bar { f1: 0, f2: 10 };
    baz(&mut b);
    b.f2 = 100;
    baz(&mut b);
    b.f2 = -5;
    // task '<main>' failed at 'invariant entering baz (x.f1 < x.f2)'
    // baz(&mut b);
}
```

## Contents

All the code for checking conditions is in `libhoare`. Currently, there is only
a single file, `lib.rs`.

The `test` directory contains unit tests for the library.

The `eg` directory contains a few examples of how to use the library:

 * hello.rs is a very simple (hello world!) example of how to use an invariant
(useful as a basic test case);
 * doc.rs contains the examples above, so we can check they compiler and run;
 * lexer.rs is a more realistic example of use - a simple (and certainly not
industrial-strength) lexer for a very small language.


## Building

To build libhoare from the top level of your checked out repo run

```
$RUSTC ./libhoare/lib.rs
```

This will create libhoare.rs in the current directory, you might want to specify
an output file using `-o`.

To build the examples run `eg.sh` in the top level and to run the tests run `tests.sh`.
Both of these assume that you have a sibling directory called `obj` and that you
used

```
$RUSTC ./libhoare/lib.rs -o "../obj/libhoare.so"
```

to build libhoare. Examples are created in `../obj`


## TODO

* cargo support (maybe it works? I didn't have much success finding out)
* tests for debug_ versions of macros - what is the best way to test this?
* better use of macro stuff? (I'm a total beginner at syntax extensions, this all
could probably be implemented better).
* better spans? (I'm not sure if I'm doing the span-stuff correctly).
* better names for scopes (`<precondition>` rather than `<quote expansion>`, etc.
These appear in the user-visible error messages, so it would be nice if they could
be informative).

Wish list:

* work on methods as well as functions (requires changes to Rust compiler so that
 syntax extensions can be placed on methods as well as functions),
* object invariants (I think this would need compiler support, if it is possible
at all. You would add `[#invariant="..."]` to a struct, enum, etc. and the
invariant would be checked on entering and leaving every method defined in any
impl for the object).
