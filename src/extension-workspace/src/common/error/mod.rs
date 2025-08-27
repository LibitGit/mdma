macro_rules! make_errors {
    ($err_type: ident, $($variant:ident),+) => {
        $(
        ::paste::paste! {
            #[macro_export]
            macro_rules! [<_ obf _ $variant:lower>] {
                ($msg:expr) => {
                    ::std::boxed::Box::new($crate::error::$err_type::Error::$variant {
                        msg: ::obfstr::obfstr!($msg).into(),
                        line: line!(),
                    }) as ::std::boxed::Box<dyn ::std::error::Error>
                };
            }
            pub use [<_ obf _ $variant:lower>] as [<obf _ $variant:lower>];
        }
        )+
    };
}

use crate::file_names::find_file_index;

pub fn encode_location(pkg_name: Option<&str>, file: &str, line: u16) -> u32 {
    let file_hash = find_file_index(pkg_name, file).unwrap_or_default();

    ((file_hash as u32) << 16) | (line as u32)
}

pub fn decode_location(encoded: u32) -> (u16, u16) {
    let high = (encoded >> 16) as u16; // Extract the high 16 bits
    let low = (encoded & 0xFFFF) as u16; // Extract the low 16 bits
    (high, low)
}

//TODO: Add Error traits by hand obfuscating the error messages.
pub mod std {
    pub use thiserror::Error;

    #[derive(Error, Debug, Clone)]
    pub enum Error {
        #[error("[{line:?}] Could not get {msg:?}")]
        Get { msg: String, line: u32 },
        #[error("[{line:?}] Could not set {msg:?}")]
        Set { msg: String, line: u32 },
        #[error("[{line:?}] Could not cast {msg:?}")]
        Cast { msg: String, line: u32 },
        #[error("[{line:?}] Could not call {msg:?}")]
        Call { msg: String, line: u32 },
        #[error("[{line:?}] {msg:?} timed out")]
        Timeout { msg: String, line: u32 },
        #[error("[{line:?}] Could not delete {msg:?}")]
        Delete { msg: String, line: u32 },
        #[error("[{line:?}] Could not fetch {msg:?}")]
        Fetch { msg: String, line: u32 },
    }

    make_errors! {
        std,
        Get,
        Set,
        Cast,
        Call,
        Timeout,
        Delete,
        Fetch,
        Lock
    }
}

pub mod serde {
    pub use thiserror::Error;

    #[derive(Error, Debug, Clone)]
    pub enum Error {
        #[error("[{line:?}] Could not serialize {msg:?}")]
        Serialize { msg: String, line: u32 },
        #[error("[{line:?}] Could not deserialize {msg:?}")]
        Deserialize { msg: String, line: u32 },
    }

    make_errors! {
        serde,
        Serialize,
        Deserialize
    }
}

pub mod promise {
    pub use thiserror::Error;

    #[derive(Error, Debug, Clone)]
    pub enum Error {
        #[error("[{line:?}] Could not resolve {msg:?} promise")]
        Resolve { msg: String, line: u32 },
        #[error("[{line:?}] Could not reject {msg:?} promise")]
        Reject { msg: String, line: u32 },
    }

    make_errors! {
        promise,
        Resolve,
        Reject
    }
}

pub mod json {
    pub use thiserror::Error;

    #[derive(Error, Debug, Clone)]
    pub enum Error {
        #[error("[{line:?}] Could not parse {msg:?}")]
        Parse { msg: String, line: u32 },
        #[error("[{line:?}] Could not stringify {msg:?}")]
        Stringify { msg: String, line: u32 },
    }

    make_errors! {
        json,
        Parse,
        Stringify
    }
}
