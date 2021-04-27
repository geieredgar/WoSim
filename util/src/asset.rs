cfg_if::cfg_if! {
    if #[cfg(feature="embed-assets")] {

        use std::io;

        #[macro_export]
        macro_rules! include_asset {
            ($e: expr) => {
                $crate::asset::AssetLoader::new(include_bytes!($e))
            };
        }

        pub struct AssetLoader {
            bytes: &'static [u8],
        }

        impl AssetLoader {
            pub const fn new(bytes: &'static [u8]) -> Self {
                Self { bytes }
            }

            pub fn load(&self) -> Result<Asset, io::Error> {
                Asset::new(self.bytes)
            }
        }

        pub struct Asset {
            bytes: &'static [u8],
        }

        impl Asset {
            fn new(bytes: &'static [u8]) -> Self {
                Self { bytes }
            }

            pub fn bytes(&self) -> &[u8] {
                self.bytes
            }
        }

    } else {
        use std::{fs::File, io};

        use memmap::{Mmap, MmapOptions};

        #[macro_export]
        macro_rules! include_asset {
            ($e: expr) => {
                $crate::asset::AssetLoader::new($e)
            };
        }

        pub struct AssetLoader {
            path: &'static str,
        }

        impl AssetLoader {
            pub const fn new(path: &'static str) -> Self {
                Self { path }
            }

            pub fn load(&self) -> Result<Asset, io::Error> {
                let file = File::open(self.path)?;
                Ok(Asset::new(unsafe { MmapOptions::new().map(&file) }?))
            }
        }

        pub struct Asset {
            mmap: Mmap,
        }

        impl Asset {
            fn new(mmap: Mmap) -> Self {
                Self { mmap }
            }

            pub fn bytes(&self) -> &[u8] {
                self.mmap.as_ref()
            }
        }
    }
}
