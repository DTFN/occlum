extern crate libc;
use std::{time, thread};
use std::thread::sleep;
use std::time::Duration;

extern "C" {
    fn increment_by_one(input: *mut libc::c_int);
}

fn main() {
    let mut input = 5;
    let old = input;
    unsafe { increment_by_one(&mut input) };
    println!("{} + 1 = {}", old, input);

    let mut number = 1;
    while number != 10 {
        number += 1;

        let handle = thread::spawn(|| {
            for i in 1..10 {
                println!("hi number {} from the spawned thread!", i);
                thread::sleep(Duration::from_millis(500));
            }
        });

        for i in 1..5 {
            println!("hi number {} from the main thread!", i);
            thread::sleep(Duration::from_millis(500));
        }

        handle.join().unwrap();

    }
}
