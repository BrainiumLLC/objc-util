#![cfg(any(target_os = "macos", target_os = "ios"))]

mod macros;

#[doc(hidden)]
pub use objc;
pub use objc::{runtime, Encode, Encoding, Message};
#[doc(hidden)]
pub use objc_macros::{class_impl, extern_objc, os_supports_impl, sel_impl};

pub type NSUInteger = std::os::raw::c_ulong;
pub type NSInteger = std::os::raw::c_long;

#[cfg(feature = "compile-time")]
#[link_section = "__DATA,__objc_imageinfo,regular,no_dead_strip"]
#[export_name = "\x01L_OBJC_IMAGE_INFO"]
#[used]
static IMAGE_INFO: [u32; 2] = [0, 64];

#[link(name = "Foundation", kind = "framework")]
extern "C" {}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NSOperatingSystemVersion {
    pub major: NSInteger,
    pub minor: NSInteger,
    pub patch: NSInteger,
}

unsafe impl objc::Encode for NSOperatingSystemVersion {
    fn encode() -> objc::Encoding {
        unsafe { objc::Encoding::from_str("{?=qqq}") }
    }
}

impl NSOperatingSystemVersion {
    pub fn as_tuple(&self) -> (NSInteger, NSInteger, NSInteger) {
        (self.major, self.minor, self.patch)
    }

    pub fn atleast(&self, major: NSInteger, minor: NSInteger, patch: NSInteger) -> bool {
        self.as_tuple() >= (major, minor, patch)
    }
}

#[cold]
unsafe fn get_os_version() -> NSOperatingSystemVersion {
    use objc::{class, msg_send, sel, sel_impl};
    let process_info: *mut runtime::Object = msg_send![class!(NSProcessInfo), processInfo];
    msg_send![process_info, operatingSystemVersion]
}

lazy_static::lazy_static! {
    pub static ref OS_VERSION: NSOperatingSystemVersion = unsafe { get_os_version() };
}
