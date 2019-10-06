// Copyright 2015 Keegan McAllister.
// Copyright 2016 Corey Farwell.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// See `LICENSE` in this repository.

use std::io::{self, Read};
use std::{panic, process};

/// Utility that reads a `Vec` of bytes from standard input (stdin)
/// and passes it to `closure`. All panics that occur within
/// `closure` will be treated as aborts. This is done so that
/// AFL considers a panic to be a crash.
///
/// # Examples
///
/// ```no_run
/// extern crate afl;
/// # struct Image;
/// # impl Image {
/// #     fn parse(_: &[u8]) {}
/// # }
///
/// fn main() {
///     afl::read_stdio_bytes(|read| {
///         Image::parse(&read)
///     })
/// }
/// ```
#[deprecated(
    since = "0.3.3",
    note = "This function does not use the `persistent mode` and `defered forkserver mode` and is therefore very slow.
Please use fuzz() or fuzz!() instead."
)]
pub fn read_stdio_bytes<F>(closure: F)
where
    F: Fn(Vec<u8>) + panic::RefUnwindSafe,
{
    let mut input = vec![];
    let result = io::stdin().read_to_end(&mut input);
    if result.is_err() {
        return;
    }
    let was_panic = panic::catch_unwind(|| {
        closure(input);
    });
    if was_panic.is_err() {
        process::abort();
    }
}

/// Utility that reads a `String` from standard input (stdin) and
/// passes it to `closure`. If a `String` cannot be constructed from
/// the data provided by standard input, `closure` will _not_ be
/// called. All panics that occur within `closure` will be treated as
/// aborts. This is done so that AFL considers a panic to be a crash.
///
/// # Examples
///
/// ```no_run
/// extern crate afl;
/// # struct Url;
/// # impl Url {
/// #     fn parse(_: &str) {}
/// # }
///
/// fn main() {
///     afl::read_stdio_string(|string| {
///         Url::parse(&string)
///     })
/// }
/// ```
#[deprecated(
    since = "0.3.3",
    note = "This function does not use the `persistent mode` and `defered forkserver mode` and is therefore very slow.
Please use fuzz() or fuzz!() instead."
)]
pub fn read_stdio_string<F>(closure: F)
where
    F: Fn(String) + panic::RefUnwindSafe,
{
    let mut input = String::new();
    let result = io::stdin().read_to_string(&mut input);
    if result.is_err() {
        return;
    }
    let was_panic = panic::catch_unwind(|| {
        closure(input);
    });
    if was_panic.is_err() {
        process::abort();
    }
}

// those functions are provided by the afl-llvm-rt static library
extern "C" {
    fn __afl_persistent_loop(counter: usize) -> isize;
    fn __afl_manual_init();
}

/// Fuzz a closure by passing it a `&[u8]`
///
/// This slice contains a "random" quantity of "random" data.
///
/// ```rust,no_run
/// # extern crate afl;
/// # use afl::fuzz;
/// # fn main() {
/// fuzz(true, |data|{
///     if data.len() != 6 {return}
///     if data[0] != b'q' {return}
///     if data[1] != b'w' {return}
///     if data[2] != b'e' {return}
///     if data[3] != b'r' {return}
///     if data[4] != b't' {return}
///     if data[5] != b'y' {return}
///     panic!("BOOM")
/// });
/// # }
/// ```
pub fn fuzz<F>(hook: bool, closure: F)
where
    F: Fn(&[u8]) + std::panic::RefUnwindSafe,
{
    // this marker strings needs to be in the produced executable for
    // afl-fuzz to detect `persistent mode` and `defered mode`
    static PERSIST_MARKER: &'static str = "##SIG_AFL_PERSISTENT##\0";
    static DEFERED_MARKER: &'static str = "##SIG_AFL_DEFER_FORKSRV##\0";

    // we now need a fake instruction to prevent the compiler from optimizing out
    // those marker strings
    unsafe { std::ptr::read_volatile(&PERSIST_MARKER) }; // hack used in https://github.com/bluss/bencher for black_box()
    unsafe { std::ptr::read_volatile(&DEFERED_MARKER) };
    // unsafe { asm!("" : : "r"(&PERSIST_MARKER)) }; // hack used in nightly's back_box(), requires feature asm
    // unsafe { asm!("" : : "r"(&DEFERED_MARKER)) };

    if hook {
      // sets panic hook to abort
      std::panic::set_hook(Box::new(|_| {
          std::process::abort();
      }));
    }

    let mut input = vec![];

    // initialize forkserver there
    unsafe { __afl_manual_init() };

    while unsafe { __afl_persistent_loop(1000) } != 0 {
        // get buffer from AFL through stdin
        let result = io::stdin().read_to_end(&mut input);
        if result.is_err() {
            return;
        }

        // We still catch unwinding panics just in case the fuzzed code modifies
        // the panic hook.
        // If so, the fuzzer will be unable to tell different bugs appart and you will
        // only be able to find one bug at a time before fixing it to then find a new one.
        let did_panic = std::panic::catch_unwind(|| {
            closure(&input);
        })
        .is_err();

        if did_panic {
            // hopefully the custom panic hook will be called before and abort the
            // process before the stack frames are unwinded.
            std::process::abort();
        }
        input.clear();
    }
}

/// Fuzz a closure-like block of code by passing it an object of arbitrary type.
///
/// You can choose the type of the argument using the syntax as in the example below.
/// Please check out the `arbitrary` crate to see which types are available.
///
/// For performance reasons, it is recommended that you use the native type `&[u8]` when possible.
///
/// ```rust,no_run
/// # #[macro_use] extern crate afl;
/// # fn main() {
/// fuzz!(|data: &[u8]| {
///     if data.len() != 6 {return}
///     if data[0] != b'q' {return}
///     if data[1] != b'w' {return}
///     if data[2] != b'e' {return}
///     if data[3] != b'r' {return}
///     if data[4] != b't' {return}
///     if data[5] != b'y' {return}
///     panic!("BOOM")
/// });
/// # }
/// ```
#[macro_export]
macro_rules! fuzz {
    (|$buf:ident| $body:block) => {
        afl::fuzz(true, |$buf| $body);
    };
    (|$buf:ident: &[u8]| $body:block) => {
        afl::fuzz(true, |$buf| $body);
    };
    (|$buf:ident: $dty: ty| $body:block) => {
        afl::fuzz(true, |$buf| {
            let $buf: $dty = {
                use arbitrary::{Arbitrary, RingBuffer};
                if let Ok(d) = RingBuffer::new($buf, $buf.len()).and_then(|mut b|{
                        Arbitrary::arbitrary(&mut b).map_err(|_| "")
                    }) {
                    d
                } else {
                    return
                }
            };

            $body
        });
    };
}

/// Fuzz a closure-like block of code by passing it an object of arbitrary type. Panics that are
/// caught inside the fuzzed code are not turned into crashes.
///
/// You can choose the type of the argument using the syntax as in the example below.
/// Please check out the `arbitrary` crate to see which types are available.
///
/// For performance reasons, it is recommended that you use the native type `&[u8]` when possible.
///
/// ```rust,no_run
/// # #[macro_use] extern crate afl;
/// # fn main() {
/// fuzz!(|data: &[u8]| {
///     if data.len() != 6 {return}
///     if data[0] != b'q' {return}
///     if data[1] != b'w' {return}
///     if data[2] != b'e' {return}
///     if data[3] != b'r' {return}
///     if data[4] != b't' {return}
///     if data[5] != b'y' {return}
///     panic!("BOOM")
/// });
/// # }
/// ```
#[macro_export]
macro_rules! fuzz_nohook {
    (|$buf:ident| $body:block) => {
        afl::fuzz(false, |$buf| $body);
    };
    (|$buf:ident: &[u8]| $body:block) => {
        afl::fuzz(false, |$buf| $body);
    };
    (|$buf:ident: $dty: ty| $body:block) => {
        afl::fuzz(false, |$buf| {
            let $buf: $dty = {
                use arbitrary::{Arbitrary, RingBuffer};
                if let Ok(d) = RingBuffer::new($buf, $buf.len())
                    .and_then(|mut b| Arbitrary::arbitrary(&mut b).map_err(|_| ""))
                {
                    d
                } else {
                    return;
                }
            };

            $body
        });
    };
}

#[cfg(test)]
mod test {
    /*
    use std::path::PathBuf;
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time;

    extern crate libc;
    extern crate tempdir;

    fn target_path() -> PathBuf {
        if PathBuf::from("../target/debug/cargo-afl-fuzz").exists() {
            PathBuf::from("../target/debug/")
        } else if PathBuf::from("target/debug/cargo-afl-fuzz").exists() {
            PathBuf::from("target/debug/")
        } else {
            panic!("Could not find cargo-afl-fuzz!");
        }
    }

    #[test]
    fn test_cargo_afl_fuzz() {
        let temp_dir = tempdir::TempDir::new("aflrs").expect("Could not create temporary directory");
        let temp_dir_path = temp_dir.path();
        let mut child = Command::new(target_path().join("cargo-afl-fuzz"))
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .arg("-i")
            .arg(".")
            .arg("-o")
            .arg(temp_dir_path)
            .arg(target_path().join("examples").join("hello"))
            .spawn()
            .expect("Could not run cargo-afl-fuzz");
        thread::sleep(time::Duration::from_secs(7));
        child.kill().unwrap();
        assert!(temp_dir_path.join("fuzzer_stats").is_file());
    }
    */
}
