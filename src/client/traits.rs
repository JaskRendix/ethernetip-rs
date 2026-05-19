use std::io;

use crate::cip::CipError;
use crate::client::ForwardOpenError;
use crate::types::{CipValue, ConnectionManagerInfo, IdentityInfo, SymbolInfo};
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

    async fn read_object_attribute(
        &mut self,
        class_id: u8,
        instance_id: u8,
        attribute_id: u8,
    ) -> Result<CipValue, CipError>;

    async fn read_object_attributes(
        &mut self,
        class_id: u8,
        instance_id: u8,
    ) -> Result<Vec<u8>, CipError>;

    async fn read_identity_attribute(&mut self, attribute_id: u8) -> Result<CipValue, CipError> {
        self.read_object_attribute(0x01, 0x01, attribute_id).await
    }

    async fn read_identity_attributes(&mut self) -> Result<Vec<u8>, CipError> {
        self.read_object_attributes(0x01, 0x01).await
    }

    async fn read_identity(&mut self) -> Result<IdentityInfo, CipError> {
        let raw = self.read_identity_attributes().await?;
        IdentityInfo::decode(&raw).map_err(|_| CipError::VendorSpecific(0xFC))
    }

    async fn read_connection_manager_attribute(
        &mut self,
        attribute_id: u8,
    ) -> Result<CipValue, CipError> {
        self.read_object_attribute(0x06, 0x01, attribute_id).await
    }

    async fn read_connection_manager_attributes(&mut self) -> Result<Vec<u8>, CipError> {
        self.read_object_attributes(0x06, 0x01).await
    }

    async fn read_connection_manager(&mut self) -> Result<ConnectionManagerInfo, CipError> {
        let raw = self.read_connection_manager_attributes().await?;
        ConnectionManagerInfo::decode(&raw).map_err(|_| CipError::VendorSpecific(0xFC))
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    struct StubIdentityClient {
        raw_attributes: Vec<u8>,
    }

    #[async_trait::async_trait]
    impl TagReadWrite for StubIdentityClient {
        async fn read_tag(&mut self, _: &str) -> Result<CipValue, CipError> {
            Err(CipError::VendorSpecific(0xFF))
        }

        async fn write_tag(&mut self, _: &str, _: CipValue) -> Result<(), CipError> {
            Err(CipError::VendorSpecific(0xFF))
        }

        async fn read_tag_multi(&mut self, _: &str, _: usize) -> Result<Vec<CipValue>, CipError> {
            Err(CipError::VendorSpecific(0xFF))
        }

        async fn write_tag_multi(&mut self, _: &str, _: &[CipValue]) -> Result<(), CipError> {
            Err(CipError::VendorSpecific(0xFF))
        }

        async fn read_object_attribute(
            &mut self,
            _: u8,
            _: u8,
            _: u8,
        ) -> Result<CipValue, CipError> {
            Err(CipError::VendorSpecific(0xFF))
        }

        async fn read_object_attributes(&mut self, _: u8, _: u8) -> Result<Vec<u8>, CipError> {
            Ok(self.raw_attributes.clone())
        }

        async fn read_bool(&mut self, _: &str) -> Result<bool, CipError> {
            Err(CipError::VendorSpecific(0xFF))
        }

        async fn read_sint(&mut self, _: &str) -> Result<i8, CipError> {
            Err(CipError::VendorSpecific(0xFF))
        }

        async fn read_int(&mut self, _: &str) -> Result<i16, CipError> {
            Err(CipError::VendorSpecific(0xFF))
        }

        async fn read_dint(&mut self, _: &str) -> Result<i32, CipError> {
            Err(CipError::VendorSpecific(0xFF))
        }

        async fn read_real(&mut self, _: &str) -> Result<f32, CipError> {
            Err(CipError::VendorSpecific(0xFF))
        }

        async fn read_string(&mut self, _: &str) -> Result<String, CipError> {
            Err(CipError::VendorSpecific(0xFF))
        }

        async fn write_bool(&mut self, _: &str, _: bool) -> Result<(), CipError> {
            Err(CipError::VendorSpecific(0xFF))
        }

        async fn write_sint(&mut self, _: &str, _: i8) -> Result<(), CipError> {
            Err(CipError::VendorSpecific(0xFF))
        }

        async fn write_int(&mut self, _: &str, _: i16) -> Result<(), CipError> {
            Err(CipError::VendorSpecific(0xFF))
        }

        async fn write_dint(&mut self, _: &str, _: i32) -> Result<(), CipError> {
            Err(CipError::VendorSpecific(0xFF))
        }

        async fn write_real(&mut self, _: &str, _: f32) -> Result<(), CipError> {
            Err(CipError::VendorSpecific(0xFF))
        }

        async fn write_string(&mut self, _: &str, _: &str) -> Result<(), CipError> {
            Err(CipError::VendorSpecific(0xFF))
        }
    }

    #[tokio::test]
    async fn test_read_identity_default_impl() {
        let mut raw = Vec::new();
        raw.extend_from_slice(&0x1234u16.to_le_bytes());
        raw.extend_from_slice(&0x5678u16.to_le_bytes());
        raw.extend_from_slice(&0x9ABCu16.to_le_bytes());
        raw.extend_from_slice(&[0x01, 0x02]);
        raw.extend_from_slice(&0x3344u16.to_le_bytes());
        raw.extend_from_slice(&0x11223344u32.to_le_bytes());

        let product_name = b"TestProduct";
        raw.extend_from_slice(&(product_name.len() as u16).to_le_bytes());
        raw.extend_from_slice(product_name);
        raw.extend(std::iter::repeat_n(0, 82 - product_name.len()));

        raw.push(0x05);

        let mut client = StubIdentityClient {
            raw_attributes: raw,
        };
        let identity = client
            .read_identity()
            .await
            .expect("read_identity should succeed");

        assert_eq!(identity.vendor_id, 0x1234);
        assert_eq!(identity.device_type, 0x5678);
        assert_eq!(identity.product_code, 0x9ABC);
        assert_eq!(identity.revision_major, 0x01);
        assert_eq!(identity.revision_minor, 0x02);
        assert_eq!(identity.status, 0x3344);
        assert_eq!(identity.serial_number, 0x11223344);
        assert_eq!(identity.product_name, "TestProduct");
        assert_eq!(identity.state, 0x05);
    }

    #[tokio::test]
    async fn test_read_connection_manager_default_impl() {
        let mut raw = Vec::new();
        raw.extend_from_slice(&0x0102u16.to_le_bytes());
        raw.extend_from_slice(&0x0304u16.to_le_bytes());
        raw.extend_from_slice(&0x0506u16.to_le_bytes());

        let mut client = StubIdentityClient {
            raw_attributes: raw,
        };
        let info = client
            .read_connection_manager()
            .await
            .expect("read_connection_manager should succeed");

        assert_eq!(info.revision, 0x0102);
        assert_eq!(info.status, 0x0304);
        assert_eq!(info.configuration_capability, 0x0506);
    }
}
