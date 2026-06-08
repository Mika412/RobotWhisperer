#![deny(missing_debug_implementations)]

use std::collections::HashMap;

use rw_canonical::MessageDef;

pub mod decode;
pub mod encode;

pub use decode::{decode_message, decode_message_body, DecodeError, DecodeResult};
pub use encode::{encode_message, encode_message_body, EncodeError, EncodeResult};

pub type Resolver = HashMap<String, MessageDef>;
