/// Macro for creating tuple struct wrappers for different u64 identifiers.
#[macro_export]
macro_rules! id_u64 {
($($(#[$cfg:meta])* $name:ident;)*) => {
        $(
            $(#[$cfg])*
            #[derive(Debug, PartialEq, Eq, Hash)]
            #[repr(transparent)]
            pub struct $name(::std::num::NonZeroU64);

            impl $name {
                #[doc = concat!("Creates a new ", stringify!($name), " from a u64.")]
                /// # Panics
                /// Panics if `id` is zero.
                #[inline]
                pub const fn new(id: u64) -> Self {
                    match ::std::num::NonZeroU64::new(id) {
                        Some(inner) => Self(inner),
                        None => panic!(concat!("Attempted to call ", stringify!($name), "::new with invalid (0) value"))
                    }
                }

                /// Retrieves the inner `id` as a [`u64`].
                pub const fn get(self) -> u64 {
                    self.0.get()
                }
            }
        )*
    }
}

#[macro_export]
macro_rules! into_collection {
    {$($name:ident: {
        @name: $collection_name:literal,
        @validator: $validator:expr $(,)?
    }),+ $(,)?} => {
        const COLLECTION_COUNT: usize = into_collection!(@count $($name),+);

        $(impl $crate::app_state::client::IntoCollection for $name {
            const COLLECTION_NAME: &str = $collection_name;

            fn validator() -> ::mongodb::bson::Document {
                $validator
            }
        })+
    };

    // Helper rule for counting - transforms each name into a unit type and counts them
    (@count $($name:ident),+) => {
        <[()]>::len(&[$(into_collection!(@replace $name)),+])
    };

    // Replace each name with a unit type for counting
    (@replace $_:ident) => { () };
}

#[macro_export]
macro_rules! bail {
    ($msg:literal $(,)?) => {
        return ::core::result::Result::Err(
            ::core::convert::Into::<$crate::types::AppError>::into(::anyhow::anyhow!($msg))
        )
    };
    ($err:expr $(,)?) => {
        return ::core::result::Result::Err(
            ::core::convert::Into::<$crate::types::AppError>::into(::anyhow::anyhow!($err))
        )
    };
    ($fmt:expr, $($arg:tt)*) => {
        return ::core::result::Result::Err(
            ::core::convert::Into::<$crate::types::AppError>::into(::anyhow::anyhow!($fmt, $($arg)*))
        )
    };
}

#[macro_export]
macro_rules! anyhow {
    ($msg:literal $(,)?) => {
        ::core::convert::Into::<$crate::types::AppError>::into(::anyhow::anyhow!($msg))
    };
    ($err:expr $(,)?) => {
        ::core::convert::Into::<$crate::types::AppError>::into(::anyhow::anyhow!($err))
    };
    ($fmt:expr, $($arg:tt)*) => {
        ::core::convert::Into::<$crate::types::AppError>::into(::anyhow::anyhow!($fmt, $($arg)*))
    };
}
