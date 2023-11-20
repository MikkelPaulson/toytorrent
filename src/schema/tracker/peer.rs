use std::borrow::Cow;
use std::net::IpAddr;

use crate::bencode::BencodeValue;
use crate::schema::PeerId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Peer {
    peer_id: Option<PeerId>,
    ip: IpAddr,
    port: u16,
}

impl TryFrom<BencodeValue<'_>> for Peer {
    type Error = ();

    fn try_from(input: BencodeValue<'_>) -> Result<Self, Self::Error> {
        let mut input_dict = input.to_dict().ok_or(())?;

        let (
            Some(BencodeValue::Bytes(peer_id_value)),
            Some(BencodeValue::Bytes(ip_value)),
            Some(BencodeValue::Integer(port_value)),
        ) = (
            input_dict.remove("peer id".as_bytes()),
            input_dict.remove("ip".as_bytes()),
            input_dict.remove("port".as_bytes()),
        )
        else {
            return Err(());
        };

        let peer_id = peer_id_value.as_ref().try_into().map_err(|_| ())?;

        let ip = String::from_utf8(ip_value.to_vec())
            .map_err(|_| ())?
            .parse()
            .map_err(|_| ())?;

        let port = port_value.try_into().map_err(|_| ())?;

        Ok(Peer {
            peer_id: Some(peer_id),
            ip,
            port,
        })
    }
}

impl TryFrom<&[u8]> for Peer {
    type Error = ();

    fn try_from(input: &[u8]) -> Result<Self, Self::Error> {
        if input.len() != 6 {
            return Err(());
        }

        let ip = {
            let ip_value: [u8; 4] = input[0..4].try_into().unwrap();
            IpAddr::V4(ip_value.into())
        };

        let port = u16::from_be_bytes(input[4..6].try_into().unwrap());

        Ok(Peer {
            peer_id: None,
            ip,
            port,
        })
    }
}

impl TryFrom<Peer> for [u8; 6] {
    type Error = ();

    fn try_from(input: Peer) -> Result<Self, Self::Error> {
        let mut result = [0; 6];

        let IpAddr::V4(ipv4_addr) = input.ip else {
            return Err(());
        };

        ipv4_addr
            .octets()
            .into_iter()
            .chain(input.port.to_be_bytes().into_iter())
            .enumerate()
            .for_each(|(i, v)| result[i] = v);

        Ok(result)
    }
}

impl<'a> From<&'a Peer> for BencodeValue<'a> {
    fn from(input: &'a Peer) -> BencodeValue<'a> {
        BencodeValue::Dict(
            [
                (
                    "ip".as_bytes().into(),
                    BencodeValue::Bytes(Cow::Owned(input.ip.to_string().as_bytes().into())),
                ),
                (
                    "port".as_bytes().into(),
                    BencodeValue::Integer(input.port.into()),
                ),
            ]
            .into_iter()
            .chain(input.peer_id.iter().map(|peer_id| {
                (
                    "peer id".as_bytes().into(),
                    BencodeValue::Bytes(Cow::Owned(peer_id.0.into())),
                )
            }))
            .collect(),
        )
    }
}
