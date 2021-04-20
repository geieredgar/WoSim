use std::{borrow::Cow, mem::size_of, ptr::copy_nonoverlapping};

#[macro_export]
macro_rules! include_shader {
    ($e:expr) => {
        $crate::include_asset!(concat!(env!("OUT_DIR"), "/", $e, ".spv"))
    };
}

pub fn align_bytes(bytes: &[u8]) -> Cow<'_, [u32]> {
    let (prefix, words, suffix) = unsafe { bytes.align_to::<u32>() };
    if prefix.is_empty() {
        assert!(suffix.is_empty(), "len must be a multiple of 4");
        Cow::from(words)
    } else {
        assert_eq!(
            bytes.len() % size_of::<u32>(),
            0,
            "len must be a multiple of 4"
        );
        let mut words = vec![0u32; bytes.len() / size_of::<u32>()];
        unsafe {
            copy_nonoverlapping(bytes.as_ptr(), words.as_mut_ptr() as *mut u8, bytes.len());
        }
        Cow::from(words)
    }
}
