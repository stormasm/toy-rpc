#![allow(dead_code)]

use serde::{Deserialize, Serialize};

use toy_rpc::macros::export_impl;
use toy_rpc::client::{Client, Connected};

pub const COMMON_TEST_MAGIC_U8: u8 = 167;
pub const COMMON_TEST_MAGIC_U16: u16 = 512;
pub const COMMON_TEST_MAGIC_U32: u32 = 13131313;
pub const COMMON_TEST_MAGIC_U64: u64 = 3131313131;
pub const COMMON_TEST_MAGIC_I8: i8 = 120;
pub const COMMON_TEST_MAGIC_I16: i16 = 200;
pub const COMMON_TEST_MAGIC_I32: i32 = 78293749;
pub const COMMON_TEST_MAGIC_I64: i64 = 8912386968;
pub const COMMON_TEST_MAGIC_BOOL: bool = false;
pub const COMMON_TEST_MAGIC_STR: &str = "a magic";

pub const COMMON_TEST_SERVICE_NAME: &str = "CommonTest";

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialOrd, Ord, PartialEq, Eq)]
pub struct CustomStruct {
    field_u8: u8,
    field_string: String,
}

impl CustomStruct {
    pub fn new() -> Self {
        Self {
            field_u8: 13,
            field_string: "ahahahd".to_string(),
        }
    }
}

#[derive(Debug)]
pub struct CommonTest {
    magic_u8: u8,
    magic_u16: u16,
    magic_u32: u32,
    magic_u64: u64,
    magic_i8: i8,
    magic_i16: i16,
    magic_i32: i32,
    magic_i64: i64,
    magic_bool: bool,
    magic_str: &'static str,
    custom_struct: CustomStruct,
}

impl CommonTest {
    pub fn new() -> Self {
        Self {
            magic_u8: COMMON_TEST_MAGIC_U8,
            magic_u16: COMMON_TEST_MAGIC_U16,
            magic_u32: COMMON_TEST_MAGIC_U32,
            magic_u64: COMMON_TEST_MAGIC_U64,
            magic_i8: COMMON_TEST_MAGIC_I8,
            magic_i16: COMMON_TEST_MAGIC_I16,
            magic_i32: COMMON_TEST_MAGIC_I32,
            magic_i64: COMMON_TEST_MAGIC_I64,
            magic_bool: COMMON_TEST_MAGIC_BOOL,
            magic_str: COMMON_TEST_MAGIC_STR,
            custom_struct: CustomStruct::new(),
        }
    }
}

#[export_impl]
impl CommonTest {
    #[export_method]
    async fn get_magic_u8(&self, _: ()) -> Result<u8, String> {
        Ok(self.magic_u8)
    }

    #[export_method]
    async fn get_magic_u16(&self, _: ()) -> Result<u16, String> {
        Ok(self.magic_u16)
    }

    #[export_method]
    async fn get_magic_u32(&self, _: ()) -> Result<u32, String> {
        Ok(self.magic_u32)
    }

    #[export_method]
    async fn get_magic_u64(&self, _: ()) -> Result<u64, String> {
        Ok(self.magic_u64)
    }

    #[export_method]
    async fn get_magic_i8(&self, _: ()) -> Result<i8, String> {
        Ok(self.magic_i8)
    }

    #[export_method]
    async fn get_magic_i16(&self, _: ()) -> Result<i16, String> {
        Ok(self.magic_i16)
    }

    #[export_method]
    async fn get_magic_i32(&self, _: ()) -> Result<i32, String> {
        Ok(self.magic_i32)
    }

    #[export_method]
    async fn get_magic_i64(&self, _: ()) -> Result<i64, String> {
        Ok(self.magic_i64)
    }

    #[export_method]
    async fn get_magic_bool(&self, _: ()) -> Result<bool, String> {
        Ok(self.magic_bool)
    }

    #[export_method]
    async fn get_magic_str(&self, _: ()) -> Result<String, String> {
        Ok(self.magic_str.into())
    }

    #[export_method]
    async fn get_magic_custom_struct(&self, _: ()) -> Result<CustomStruct, String> {
        Ok(self.custom_struct.clone())
    }
}

pub async fn test_get_magic_u8(client: &Client<Connected>) {
    let reply: u8 = client
        .common_test()
        .get_magic_u8(())
        .await
        .expect("Unexpected error executing RPC");
    assert_eq!(COMMON_TEST_MAGIC_U8, reply);
}

pub async fn test_get_magic_u16(client: &Client<Connected>) {
    let reply: u16 = client
        .common_test()
        .get_magic_u16(())
        .await
        .expect("Unexpected error executing RPC");
    assert_eq!(COMMON_TEST_MAGIC_U16, reply);
}

pub async fn test_get_magic_u32(client: &Client<Connected>) {
    let reply: u32 = client
        .common_test()
        .get_magic_u32(())
        .await
        .expect("Unexpected error executing RPC");
    assert_eq!(COMMON_TEST_MAGIC_U32, reply);
}

pub async fn test_get_magic_u64(client: &Client<Connected>) {
    let reply: u64 = client
        .common_test()
        .get_magic_u64(())
        .await
        .expect("Unexpected error executing RPC");
    assert_eq!(COMMON_TEST_MAGIC_U64, reply);
}

pub async fn test_get_magic_i8(client: &Client<Connected>) {
    let reply: i8 = client
        .common_test()
        .get_magic_i8(())
        .await
        .expect("Unexpected error executing RPC");
    assert_eq!(COMMON_TEST_MAGIC_I8, reply);
}

pub async fn test_get_magic_i16(client: &Client<Connected>) {
    let reply: i16 = client
        .common_test()
        .get_magic_i16(())
        .await
        .expect("Unexpected error executing RPC");
    assert_eq!(COMMON_TEST_MAGIC_I16, reply);
}

pub async fn test_get_magic_i32(client: &Client<Connected>) {
    let reply: i32 = client
        .common_test()
        .get_magic_i32(())
        .await
        .expect("Unexpected error executing RPC");
    assert_eq!(COMMON_TEST_MAGIC_I32, reply);
}

pub async fn test_get_magic_i64(client: &Client<Connected>) {
    let reply: i64 = client
        .common_test()
        .get_magic_i64(())
        .await
        .expect("Unexpected error executing RPC");
    assert_eq!(COMMON_TEST_MAGIC_I64, reply);
}

pub async fn test_get_magic_bool(client: &Client<Connected>) {
    let reply: bool = client
        .common_test()
        .get_magic_bool(())
        .await
        .expect("Unexpected error executing RPC");
    assert_eq!(COMMON_TEST_MAGIC_BOOL, reply);
}

pub async fn test_get_magic_str(client: &Client<Connected>) {
    let reply: String = client
        .common_test()
        .get_magic_str(())
        .await
        .expect("Unexpected error executing RPC");
    let reply = &reply[..];
    assert_eq!(COMMON_TEST_MAGIC_STR, reply);
}
