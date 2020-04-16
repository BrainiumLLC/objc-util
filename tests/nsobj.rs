use objc_util::class;
use test_lib::*;

#[test]
fn nsdata() {
    unsafe {
        let data = "aaaaa";
        let obj = nsobj_alloc(class!(NSData));
        let obj = nsdata_init_with_bytes(obj, data.as_ptr() as _, data.len() as _);
        let obj2 = nsdata_data_with_bytes(class!(NSData), data.as_ptr() as _, data.len() as _);
        assert_eq!(nsobj_hash(obj), nsobj_hash(obj2));
        assert!(match nsobj_is_equal(obj, obj2) {
            objc_util::runtime::YES => true,
            _ => false,
        });

        let data2 = "bbbbb";
        let obj3 = nsdata_data_with_bytes(class!(NSData), data2.as_ptr() as _, data2.len() as _);
        assert!(match nsobj_is_equal(obj, obj3) {
            objc_util::runtime::NO => true,
            _ => false,
        })
    }
}
