use anachro_icd::arbitrator::SubMsg;
use postcard;
use crate::client::PUBLISH_SHORTCODE_OFFSET;

pub enum TableError {
    NoMatch,
    Postcard(postcard::Error),
}

pub trait Table: Sized {
    fn sub_paths() -> &'static [&'static str];
    fn pub_paths() -> &'static [&'static str];
    fn from_pub_sub<'a>(msg: &'a SubMsg<'a>) -> Result<Self, TableError>;
}

// TODO: Postcard feature?

/// ## Example
/// pubsub_table!{
///     AnachroTable,
///     Subs => {
///         Something: "foo/bar/baz" => Demo,
///         Else: "bib/bim/bap" => (),
///     },
///     Pubs => {
///         Etwas: "short/send" => (),
///         Anders: "send/short" => (),
///     },
/// }
#[macro_export]
macro_rules! pubsub_table {
    (
        $enum_ty:ident,
        Subs => {
            $($sub_variant_name:ident: $sub_path:expr => $sub_variant_ty:ty,)+
        },
        Pubs => {
            $($pub_variant_name:ident: $pub_path:expr => $pub_variant_ty:ty,)+
        },
    ) => {
        #[derive(Debug, serde::Deserialize, Clone)]
        pub enum $enum_ty {
            $($sub_variant_name($sub_variant_ty)),+,
            $($pub_variant_name($pub_variant_ty)),+,
        }

        impl $crate::Table for $enum_ty {
            fn from_pub_sub<'a>(msg: &'a $crate::SubMsg<'a>) -> core::result::Result<Self, $crate::TableError> {
                let msg_path = match msg.path {
                    $crate::anachro_icd::PubSubPath::Long(ref path) => path.as_str(),
                    $crate::anachro_icd::PubSubPath::Short(sid) => {
                        if sid < PUBLISH_SHORTCODE_OFFSET {
                            // Subscribe
                            if (sid as usize) < Self::sub_paths().len() {
                                Self::sub_paths()[(sid as usize)]
                            } else {
                                return Err($crate::TableError::NoMatch);
                            }
                        } else {
                            // publish
                            let new_sid = (sid as usize) - (PUBLISH_SHORTCODE_OFFSET as usize);
                            if new_sid < Self::pub_paths().len() {
                                Self::pub_paths()[new_sid]
                            } else {
                                return Err($crate::TableError::NoMatch);
                            }
                        }
                    },
                };
                $(
                    if $crate::anachro_icd::matches(msg_path, $sub_path) {
                        return Ok(
                            $enum_ty::$sub_variant_name(
                                $crate::from_bytes(msg.payload)
                                    .map_err(|e| $crate::TableError::Postcard(e))?
                            )
                        );
                    }
                )+
                $(
                    if $crate::anachro_icd::matches(msg_path, $pub_path) {
                        return Ok(
                            $enum_ty::$pub_variant_name(
                                $crate::from_bytes(msg.payload)
                                    .map_err(|e| $crate::TableError::Postcard(e))?
                            )
                        );
                    }
                )+
                Err($crate::TableError::NoMatch)
            }

            fn sub_paths() -> &'static [&'static str] {
                Self::sub_paths()
            }

            fn pub_paths() -> &'static [&'static str] {
                Self::pub_paths()
            }
        }

        impl $enum_ty {
            pub fn get_pub_path(&self) -> core::option::Option<&'static str> {
                match self {
                    $(
                        $enum_ty::$pub_variant_name(_) => Some($pub_path),
                    )+
                    _ => None,
                }
            }

            pub fn serialize<'a>(&self, buffer: &'a mut [u8]) -> core::result::Result<$crate::SendMsg<'a>, ()> {
                match self {
                    $(
                        $enum_ty::$pub_variant_name(msg) => {
                            Ok($crate::SendMsg {
                                buf: $crate::to_slice(msg, buffer)
                                        .map_err(drop)?,
                                path: $pub_path,
                            })
                        },
                    )+
                    _ => Err(()),
                }
            }

            pub const fn sub_paths() -> &'static [&'static str] {
                const PATHS: &[&str] = &[
                    $($sub_path,)+
                ];

                PATHS
            }

            pub const fn pub_paths() -> &'static [&'static str] {
                const PATHS: &[&str] = &[
                    $($pub_path,)+
                ];

                PATHS
            }
        }
    };
}
