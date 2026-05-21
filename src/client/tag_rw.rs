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

        let res = self
            .route_cip_request(cip)
            .await
            .map_err(|_| CipError::VendorSpecific(0xFF))?;

        let payload = parse_cip_response_payload(&res)?;

        decode_cip_response(payload).ok_or(CipError::VendorSpecific(0xFC))
    }

    async fn write_tag(&mut self, tag: &str, value: CipValue) -> Result<(), CipError> {
        let cip = build_write_request(tag, &value, self.slot);

        let res = self
            .route_cip_request(cip)
            .await
            .map_err(|_| CipError::VendorSpecific(0xFF))?;

        match decode_write_response(&res) {
            Ok(()) => Ok(()),
            Err(status) => Err(CipError::from(status)),
        }
    }

    async fn read_tag_multi(&mut self, tag: &str, count: usize) -> Result<Vec<CipValue>, CipError> {
        let cip = super::build_read_request_count(tag, count, self.slot);

        let res = self
            .route_cip_request(cip)
            .await
            .map_err(|_| CipError::VendorSpecific(0xFF))?;

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
        let cip = build_get_attribute_single_request(
            class_id as u16,
            instance_id as u16,
            attribute_id as u16,
            self.slot,
        );

        let res = self
            .route_cip_request(cip)
            .await
            .map_err(|_| CipError::VendorSpecific(0xFF))?;

        let payload = parse_cip_response_payload(&res)?;

        decode_cip_response(payload).ok_or(CipError::VendorSpecific(0xFC))
    }

    async fn read_object_attributes(
        &mut self,
        class_id: u8,
        instance_id: u8,
    ) -> Result<Vec<u8>, CipError> {
        let cip = build_get_attribute_all_request(class_id as u16, instance_id as u16, self.slot);

        let res = self
            .route_cip_request(cip)
            .await
            .map_err(|_| CipError::VendorSpecific(0xFF))?;

        let payload = parse_cip_response_payload(&res)?;

        Ok(payload.to_vec())
    }

    async fn write_tag_multi(&mut self, tag: &str, values: &[CipValue]) -> Result<(), CipError> {
        if values.is_empty() {
            return Ok(());
        }

        // Try fast single-packet array write for scalar types
        if self.try_fast_array_write(tag, values).await? {
            return Ok(());
        }

        // Fallback: old per-element loop (works for all types, including STRING)
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

impl EthernetIpClient {
    /// Unified transport selection with simple connected→unconnected fallback.
    async fn route_cip_request(&mut self, cip: Vec<u8>) -> io::Result<Vec<u8>> {
        if self.connected {
            match self.send_unit_data(cip.clone()).await {
                Ok(res) => return Ok(res),
                Err(_) => {
                    // Drop connected state and fall back to unconnected
                    self.connected = false;
                    self.connection_id = None;
                }
            }
        }
        self.send_rr_data(cip).await
    }

    /// Fast single-packet array write for scalar types.
    /// Returns Ok(true) if fast path was used, Ok(false) if caller should fall back.
    async fn try_fast_array_write(
        &mut self,
        tag: &str,
        values: &[CipValue],
    ) -> Result<bool, CipError> {
        if values.is_empty() {
            return Ok(true);
        }

        // Determine type and pack bytes, enforcing homogeneous types
        let first = &values[0];
        let (type_id, raw_bytes) = match first {
            CipValue::Bool(_) => {
                let mut buf = Vec::with_capacity(values.len());
                for v in values {
                    match v {
                        CipValue::Bool(b) => buf.push(if *b { 1 } else { 0 }),
                        other => {
                            return Err(CipError::TypeMismatch {
                                expected: "BOOL",
                                actual: other.type_name(),
                            })
                        }
                    }
                }
                (0x00C1u16, buf)
            }
            CipValue::SInt(_) => {
                let mut buf = Vec::with_capacity(values.len());
                for v in values {
                    match v {
                        CipValue::SInt(s) => buf.push(*s as u8),
                        other => {
                            return Err(CipError::TypeMismatch {
                                expected: "SINT",
                                actual: other.type_name(),
                            })
                        }
                    }
                }
                (0x00C2u16, buf)
            }
            CipValue::Int(_) => {
                let mut buf = Vec::with_capacity(values.len() * 2);
                for v in values {
                    match v {
                        CipValue::Int(i) => buf.extend_from_slice(&i.to_le_bytes()),
                        other => {
                            return Err(CipError::TypeMismatch {
                                expected: "INT",
                                actual: other.type_name(),
                            })
                        }
                    }
                }
                (0x00C3u16, buf)
            }
            CipValue::DInt(_) => {
                let mut buf = Vec::with_capacity(values.len() * 4);
                for v in values {
                    match v {
                        CipValue::DInt(d) => buf.extend_from_slice(&d.to_le_bytes()),
                        other => {
                            return Err(CipError::TypeMismatch {
                                expected: "DINT",
                                actual: other.type_name(),
                            })
                        }
                    }
                }
                (0x00C4u16, buf)
            }
            CipValue::Real(_) => {
                let mut buf = Vec::with_capacity(values.len() * 4);
                for v in values {
                    match v {
                        CipValue::Real(r) => buf.extend_from_slice(&r.to_bits().to_le_bytes()),
                        other => {
                            return Err(CipError::TypeMismatch {
                                expected: "REAL",
                                actual: other.type_name(),
                            })
                        }
                    }
                }
                (0x00CAu16, buf)
            }
            // Complex types (STRING, STRUCT, etc.) fall back to per-element writes
            _ => return Ok(false),
        };

        // Use a template write to extract the correct path and routing
        let template_cip = build_write_request(tag, &values[0], self.slot);
        if template_cip.len() < 4 {
            return Err(CipError::VendorSpecific(0xFE));
        }

        let path_words = template_cip[1] as usize;
        let header_len = 2 + (path_words * 2);

        if template_cip.len() < header_len {
            return Err(CipError::VendorSpecific(0xFD));
        }

        // Build optimized CIP frame: [service + path] + type_id + count + data
        let mut cip = Vec::with_capacity(header_len + 4 + raw_bytes.len());
        cip.extend_from_slice(&template_cip[..header_len]);
        cip.extend_from_slice(&type_id.to_le_bytes());
        cip.extend_from_slice(&(values.len() as u16).to_le_bytes());
        cip.extend_from_slice(&raw_bytes);

        let res = self
            .route_cip_request(cip)
            .await
            .map_err(|_| CipError::VendorSpecific(0xFF))?;

        match decode_write_response(&res) {
            Ok(()) => Ok(true),
            Err(status) => Err(CipError::from(status)),
        }
    }
}
