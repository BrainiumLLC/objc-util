#![allow(soft_unstable)]
#![feature(test)]

extern crate test;

use objc_util::class;
use test_lib::*;

#[bench]
pub fn hash(b: &mut test::Bencher) {
    unsafe {
        let data = "aaaaa";
        let obj = nsobj_alloc(class!(NSData));
        let obj = nsdata_init_with_bytes(obj, data.as_ptr() as _, data.len() as _);
        b.iter(|| {
            for _ in 0..1_000_000 {
                nsobj_hash(obj);
            }
        });
    }
}
