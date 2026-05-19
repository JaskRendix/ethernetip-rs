use std::io;

use crate::cip::{
    build_get_attribute_all_request, build_get_attribute_single_request, build_read_request,
    build_write_request, decode_cip_response, decode_write_response, CipError,
};
use crate::client::{ConnectedMessaging, TagReadWrite, UnconnectedMessaging};
use crate::types::CipValue;

use super::EthernetIpClient;

#[async_trait::async_trait]
impl TagReadWrite for EthernetIpClient {
    async fn read_tag(&mut self, tag: &str) -> Result<CipValue, CipError> {
        let cip = build_read_request(tag, self.slot);

        let res: io::Result<Vec<u8>> = if self.connected {
            self.send_unit_data(cip).await
        } else {
            self.send_rr_data(cip).await
        };

        let res = res.map_err(|_| CipError::VendorSpecific(0xFF))?;
        let payload = parse_cip_response_payload(&res)?;

        decode_cip_response(payload).ok_or(CipError::VendorSpecific(0xFC))
    }

    async fn write_tag(&mut self, tag: &str, value: CipValue) -> Result<(), CipError> {
        let cip = build_write_request(tag, &value, self.slot);

        let res: io::Result<Vec<u8>> = if self.connected {
            self.send_unit_data(cip).await
        } else {
            self.send_rr_data(cip).await
        };

        let res = res.map_err(|_| CipError::VendorSpecific(0xFF))?;

        match decode_write_response(&res) {
            Ok(()) => Ok(()),
            Err(status) => Err(CipError::from(status)),
        }
    }

    async fn read_tag_multi(&mut self, tag: &str, count: usize) -> Result<Vec<CipValue>, CipError> {
        let cip = super::build_read_request_count(tag, count, self.slot);

        let res: io::Result<Vec<u8>> = if self.connected {
            self.send_unit_data(cip).await
        } else {
            self.send_rr_data(cip).await
        };

        let res = res.map_err(|_| CipError::VendorSpecific(0xFF))?;
        let payload = parse_cip_response_payload(&res)?;

        if payload.len() < 2 {
            return Err(CipError::VendorSpecific(0xFD));
        }

        let type_id = u16::from_le_bytes([payload[0], payload[1]]);
        let payload = &payload[2..];

        Ok(crate::cip::decode_cip_data_list(type_id, payload))
    }

    async fn read_object_attribute(
        &mut self,
        class_id: u8,
        instance_id: u8,
        attribute_id: u8,
    ) -> Result<CipValue, CipError> {
        let cip =
            build_get_attribute_single_request(class_id, instance_id, attribute_id, self.slot);

        let res: io::Result<Vec<u8>> = if self.connected {
            self.send_unit_data(cip).await
        } else {
            self.send_rr_data(cip).await
        };

        let res = res.map_err(|_| CipError::VendorSpecific(0xFF))?;
        let payload = parse_cip_response_payload(&res)?;

        decode_cip_response(payload).ok_or(CipError::VendorSpecific(0xFC))
    }

    async fn read_object_attributes(
        &mut self,
        class_id: u8,
        instance_id: u8,
    ) -> Result<Vec<u8>, CipError> {
        let cip = build_get_attribute_all_request(class_id, instance_id, self.slot);

        let res: io::Result<Vec<u8>> = if self.connected {
            self.send_unit_data(cip).await
        } else {
            self.send_rr_data(cip).await
        };

        let res = res.map_err(|_| CipError::VendorSpecific(0xFF))?;
        let payload = parse_cip_response_payload(&res)?;

        Ok(payload.to_vec())
    }

    async fn write_tag_multi(&mut self, tag: &str, values: &[CipValue]) -> Result<(), CipError> {
        for (i, v) in values.iter().enumerate() {
            let indexed = format!("{tag}[{i}]");
            self.write_tag(&indexed, v.clone()).await?;
        }
        Ok(())
    }

    async fn read_bool(&mut self, tag: &str) -> Result<bool, CipError> {
        match self.read_tag(tag).await? {
            CipValue::Bool(v) => Ok(v),
            other => Err(CipError::TypeMismatch {
                expected: "BOOL",
                actual: other.type_name(),
            }),
        }
    }

    async fn read_sint(&mut self, tag: &str) -> Result<i8, CipError> {
        match self.read_tag(tag).await? {
            CipValue::SInt(v) => Ok(v),
            other => Err(CipError::TypeMismatch {
                expected: "SINT",
                actual: other.type_name(),
            }),
        }
    }

    async fn read_int(&mut self, tag: &str) -> Result<i16, CipError> {
        match self.read_tag(tag).await? {
            CipValue::Int(v) => Ok(v),
            other => Err(CipError::TypeMismatch {
                expected: "INT",
                actual: other.type_name(),
            }),
        }
    }

    async fn read_dint(&mut self, tag: &str) -> Result<i32, CipError> {
        match self.read_tag(tag).await? {
            CipValue::DInt(v) => Ok(v),
            other => Err(CipError::TypeMismatch {
                expected: "DINT",
                actual: other.type_name(),
            }),
        }
    }

    async fn read_real(&mut self, tag: &str) -> Result<f32, CipError> {
        match self.read_tag(tag).await? {
            CipValue::Real(v) => Ok(v),
            other => Err(CipError::TypeMismatch {
                expected: "REAL",
                actual: other.type_name(),
            }),
        }
    }

    async fn read_string(&mut self, tag: &str) -> Result<String, CipError> {
        match self.read_tag(tag).await? {
            CipValue::String(s) => Ok(s),
            other => Err(CipError::TypeMismatch {
                expected: "STRING",
                actual: other.type_name(),
            }),
        }
    }

    async fn write_bool(&mut self, tag: &str, value: bool) -> Result<(), CipError> {
        self.write_tag(tag, CipValue::Bool(value)).await
    }

    async fn write_sint(&mut self, tag: &str, value: i8) -> Result<(), CipError> {
        self.write_tag(tag, CipValue::SInt(value)).await
    }

    async fn write_int(&mut self, tag: &str, value: i16) -> Result<(), CipError> {
        self.write_tag(tag, CipValue::Int(value)).await
    }

    async fn write_dint(&mut self, tag: &str, value: i32) -> Result<(), CipError> {
        self.write_tag(tag, CipValue::DInt(value)).await
    }

    async fn write_real(&mut self, tag: &str, value: f32) -> Result<(), CipError> {
        self.write_tag(tag, CipValue::Real(value)).await
    }

    async fn write_string(&mut self, tag: &str, value: &str) -> Result<(), CipError> {
        self.write_tag(tag, CipValue::String(value.to_string()))
            .await
    }
}

fn parse_cip_response_payload(res: &[u8]) -> Result<&[u8], CipError> {
    if res.len() < 4 {
        return Err(CipError::VendorSpecific(0xFE));
    }

    let general_status = res[2];
    if general_status != 0 {
        return Err(CipError::from(general_status));
    }

    let ext_words = res[3] as usize;
    let data_start = 4 + ext_words * 2;
    if res.len() < data_start {
        return Err(CipError::VendorSpecific(0xFD));
    }

    Ok(&res[data_start..])
}
