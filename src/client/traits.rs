use std::io;

use crate::cip::CipError;
use crate::client::ForwardOpenError;
use crate::types::{CipValue, SymbolInfo};
use crate::MultiResult;

pub trait ConnectionManagement {
    fn set_slot(&mut self, slot: u8);
    fn is_connected(&self) -> bool;
    fn sequence(&self) -> u16;

    fn set_retries(&mut self, retries: usize);

    fn connection_ip(&self) -> &str;
}

#[async_trait::async_trait]
pub trait Connectable: Sized {
    async fn connect(ip: &str) -> io::Result<Self>;
    async fn close(self) -> io::Result<()>;
}

#[async_trait::async_trait]
pub trait Discovery {
    async fn discover() -> io::Result<Vec<(String, String)>>;
}

#[async_trait::async_trait]
pub trait UnconnectedMessaging {
    async fn send_rr_data(&mut self, cip: Vec<u8>) -> io::Result<Vec<u8>>;
}

#[async_trait::async_trait]
pub trait ConnectedMessaging {
    async fn forward_open(&mut self) -> Result<(), ForwardOpenError>;
    async fn forward_close(&mut self) -> io::Result<()>;
    async fn large_forward_open(&mut self) -> io::Result<()>;
    async fn large_forward_close(&mut self) -> io::Result<()>;
    async fn forward_open_with_fallback(&mut self) -> io::Result<()>;
    async fn send_unit_data(&mut self, cip: Vec<u8>) -> io::Result<Vec<u8>>;
}

#[async_trait::async_trait]
pub trait TagReadWrite {
    async fn read_tag(&mut self, tag: &str) -> Result<CipValue, CipError>;
    async fn write_tag(&mut self, tag: &str, value: CipValue) -> Result<(), CipError>;

    async fn read_tag_multi(&mut self, tag: &str, count: usize) -> Result<Vec<CipValue>, CipError>;
    async fn write_tag_multi(&mut self, tag: &str, values: &[CipValue]) -> Result<(), CipError>;

    async fn read_bool(&mut self, tag: &str) -> Result<bool, CipError>;
    async fn read_sint(&mut self, tag: &str) -> Result<i8, CipError>;
    async fn read_int(&mut self, tag: &str) -> Result<i16, CipError>;
    async fn read_dint(&mut self, tag: &str) -> Result<i32, CipError>;
    async fn read_real(&mut self, tag: &str) -> Result<f32, CipError>;
    async fn read_string(&mut self, tag: &str) -> Result<String, CipError>;

    async fn write_bool(&mut self, tag: &str, value: bool) -> Result<(), CipError>;
    async fn write_sint(&mut self, tag: &str, value: i8) -> Result<(), CipError>;
    async fn write_int(&mut self, tag: &str, value: i16) -> Result<(), CipError>;
    async fn write_dint(&mut self, tag: &str, value: i32) -> Result<(), CipError>;
    async fn write_real(&mut self, tag: &str, value: f32) -> Result<(), CipError>;
    async fn write_string(&mut self, tag: &str, value: &str) -> Result<(), CipError>;
}

#[async_trait::async_trait]
pub trait FragmentedRead {
    async fn read_tag_fragmented(
        &mut self,
        tag: &str,
        count: u16,
    ) -> Result<(u16, Vec<u8>), CipError>;

    async fn read_array(&mut self, tag: &str, count: u16) -> Result<Vec<CipValue>, CipError>;
}

#[async_trait::async_trait]
pub trait MultipleServicePacket {
    async fn read_tags_msp(
        &mut self,
        tags: &[&str],
    ) -> Result<Vec<MultiResult<CipValue>>, CipError>;
}

#[async_trait::async_trait]
pub trait SymbolBrowsing {
    async fn browse_symbols(&mut self) -> Result<Vec<SymbolInfo>, CipError>;
}
