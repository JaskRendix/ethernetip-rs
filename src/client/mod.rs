mod connected;
mod connection;
mod discovery;
mod fragmented;
mod msp;
mod symbols;
mod tag_rw;
mod traits;
mod unconnected;

pub use discovery::DiscoveryImpl;
pub use traits::*;

use tokio::net::TcpStream;

use crate::cip::ConnectionParams;
use crate::cip::*;

#[derive(Debug)]
pub struct EthernetIpClient {
    pub(crate) stream: TcpStream,
    pub(crate) session: u32,
    pub(crate) slot: Option<u8>,
    pub(crate) connection_id: Option<u32>,
    pub(crate) sequence: u16,
    pub(crate) retries: usize,
    pub(crate) ip: String,
    pub(crate) connected: bool,
    pub(crate) connection_params: ConnectionParams,
}

impl EthernetIpClient {
    pub fn set_connection_params(&mut self, params: ConnectionParams) {
        self.connection_params = params;
    }

    pub async fn try_send_unit_data(&mut self, cip: Vec<u8>) -> std::io::Result<Vec<u8>> {
        self.send_unit_data(cip).await
    }

    pub fn parse_cpf(data: &[u8]) -> std::io::Result<&[u8]> {
        if data.len() < 10 {
            return Err(std::io::Error::other("Data too short for CPF"));
        }

        let item_count = u16::from_le_bytes([data[6], data[7]]);
        let mut pos = 8;

        for _ in 0..item_count {
            if data.len() < pos + 4 {
                return Err(std::io::Error::other("CPF item header truncated"));
            }

            let type_id = u16::from_le_bytes([data[pos], data[pos + 1]]);
            let len = u16::from_le_bytes([data[pos + 2], data[pos + 3]]) as usize;
            pos += 4;

            if data.len() < pos + len {
                return Err(std::io::Error::other("CPF item length out of bounds"));
            }

            if type_id == 0x00B2 {
                return Ok(&data[pos..pos + len]);
            }

            pos += len;
        }

        Err(std::io::Error::other("No CIP data item found in CPF"))
    }
}

pub fn build_read_request_count(tag: &str, count: usize, slot: Option<u8>) -> Vec<u8> {
    let mut cip = build_read_request(tag, slot);
    let count_le = (count as u16).to_le_bytes();
    let pos = cip.len() - 2;
    cip[pos] = count_le[0];
    cip[pos + 1] = count_le[1];
    cip
}
