use std::io;

use crate::cip::{
    build_cip_multiple_service_request, build_read_request, parse_cip_multiple_service_response,
    CipError,
};
use crate::client::{ConnectedMessaging, UnconnectedMessaging};
use crate::types::CipValue;
use crate::MultiResult;

use super::{EthernetIpClient, MultipleServicePacket};

#[async_trait::async_trait]
impl MultipleServicePacket for EthernetIpClient {
    async fn read_tags_msp(
        &mut self,
        tags: &[&str],
    ) -> Result<Vec<MultiResult<CipValue>>, CipError> {
        let mut reqs = Vec::with_capacity(tags.len());
        for tag in tags {
            let cip = build_read_request(tag, self.slot);
            reqs.push(cip);
        }

        let msp = build_cip_multiple_service_request(&reqs);

        let res: io::Result<Vec<u8>> = if self.connected {
            self.send_unit_data(msp).await
        } else {
            self.send_rr_data(msp).await
        };

        let res = res.map_err(|_| CipError::VendorSpecific(0xFF))?;

        Ok(parse_cip_multiple_service_response(&res))
    }
}
