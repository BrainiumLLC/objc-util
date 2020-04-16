use objc_util::*;

#[extern_objc(framework = "Foundation")]
extern "ObjC" {
    #[objc(selector = "alloc", macos = "10", ios = "2")]
    pub fn nsobj_alloc(class: *const runtime::Class) -> *mut runtime::Object;

    #[objc(selector = "init", macos = "10", ios = "2")]
    #[inline(never)]
    pub fn nsobj_init(obj: *mut runtime::Object) -> *mut runtime::Object;

    #[objc(selector = "retain", macos = "10", ios = "2")]
    pub fn nsobj_retain(obj: *mut runtime::Object) -> *mut runtime::Object;

    #[objc(selector = "hash", macos = "10", ios = "2")]
    #[inline]
    pub fn nsobj_hash(obj: *const runtime::Object) -> NSUInteger;

    #[objc(selector = "isEqual:", macos = "10", ios = "2")]
    pub fn nsobj_is_equal(
        lhs: *const runtime::Object,
        rhs: *const runtime::Object,
    ) -> objc::runtime::BOOL;

    #[objc(selector = "data", macos = "10", ios = "2")]
    pub fn nsdata_data(class: *const runtime::Class) -> *mut runtime::Object;

    #[objc(selector = "initWithBytes:length:", macos = "10", ios = "2")]
    pub fn nsdata_init_with_bytes(
        obj: *const runtime::Object,
        bytes: *const std::os::raw::c_void,
        length: NSUInteger,
    ) -> *mut runtime::Object;

    #[objc(selector = "dataWithBytes:length:", macos = "10", ios = "2")]
    pub fn nsdata_data_with_bytes(
        class: *const runtime::Class,
        bytes: *const std::os::raw::c_void,
        length: NSUInteger,
    ) -> *mut runtime::Object;
}
