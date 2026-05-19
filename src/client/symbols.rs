use std::io;

use crate::cip::{build_symbol_browse_request, parse_symbol_browse_response, CipError};
use crate::client::{ConnectedMessaging, UnconnectedMessaging};

use super::{EthernetIpClient, SymbolBrowsing};

pub use crate::types::SymbolInfo;

#[async_trait::async_trait]
impl SymbolBrowsing for EthernetIpClient {
    async fn browse_symbols(&mut self) -> Result<Vec<SymbolInfo>, CipError> {
        let cip = build_symbol_browse_request();

        let res: io::Result<Vec<u8>> = if self.connected {
            self.send_unit_data(cip).await
        } else {
            self.send_rr_data(cip).await
        };

        let res = res.map_err(|_| CipError::VendorSpecific(0xFF))?;

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

        let symbols = parse_symbol_browse_response(&res[data_start..]);
        Ok(symbols)
    }
}
