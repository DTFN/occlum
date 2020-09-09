extern crate libc;
use std::time;
use std::thread::sleep;

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
        println!("loop {}",number);
        sleep(time::Duration::from_millis(1000));
    }
}
