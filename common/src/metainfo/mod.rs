mod file;
mod info;
mod md5;
mod piece;

pub use file::File;
pub use info::Info;
pub use md5::Md5Value;
pub use piece::Piece;

use crate::bencode::BencodeValue;
use crate::{Error, InfoHash};

use std::time::SystemTime;

use sha1::{Digest, Sha1};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MetainfoFile {
    pub info: Info,
    pub announce: String,
    pub announce_list: Option<Vec<Vec<String>>>,
    pub creation_date: Option<SystemTime>,
    pub comment: Option<String>,
    pub created_by: Option<String>,
    pub encoding: Option<String>,

    info_hash: InfoHash,
}

impl MetainfoFile {
    pub fn info_hash(&self) -> &InfoHash {
        &self.info_hash
    }
}

impl TryFrom<&[u8]> for MetainfoFile {
    type Error = Error;

    fn try_from(input: &[u8]) -> Result<Self, Self::Error> {
        BencodeValue::decode(input)?.try_into()
    }
}

impl TryFrom<BencodeValue<'_>> for MetainfoFile {
    type Error = Error;

    fn try_from(input: BencodeValue<'_>) -> Result<Self, Self::Error> {
        let mut input_dict = input
            .to_dict()
            .ok_or("Torrent file must be a bencoded dict")?;

        let (Some(info_benc), Some(announce)) = (
            input_dict.remove(&b"info"[..]),
            input_dict
                .remove(&b"announce"[..])
                .and_then(BencodeValue::to_string),
        ) else {
            return Err("Torrent file must contain `info` and `announce` keys".into());
        };

        let info_hash_array: [u8; 20] = Sha1::new_with_prefix(info_benc.encode()).finalize().into();

        let announce_list =
            if let Some(announce_tiers_benc) = input_dict.remove(&b"announce-list"[..]) {
                let announce_tiers = announce_tiers_benc
                    .to_list()
                    .ok_or("`announce-list` must be a list")?;
                let mut announce_tiers_vec = Vec::with_capacity(announce_tiers.len());

                for announce_tier_benc in announce_tiers {
                    let announce_tier = announce_tier_benc
                        .to_list()
                        .ok_or("`announce-list` must consist exclusively of lists")?;
                    let mut announce_tier_vec = Vec::with_capacity(announce_tier.len());

                    for announce_benc in announce_tier {
                        let announce = announce_benc.to_string().ok_or(
                            "Each `announce-list` tier must consist exclusively of strings",
                        )?;
                        announce_tier_vec.push(announce);
                    }

                    announce_tiers_vec.push(announce_tier_vec);
                }

                Some(announce_tiers_vec)
            } else {
                None
            };

        let creation_date = input_dict
            .remove(&b"creation date"[..])
            .map(|creation_date_benc| {
                creation_date_benc
                    .to_time()
                    .ok_or("`creation date` must be a valid UNIX timestamp")
            })
            .transpose()?;

        let comment = input_dict
            .remove(&b"comment"[..])
            .map(|comment_benc| {
                comment_benc
                    .to_string()
                    .ok_or("`comment` must be a valid string")
            })
            .transpose()?;

        let created_by = input_dict
            .remove(&b"created by"[..])
            .map(|created_by_benc| {
                created_by_benc
                    .to_string()
                    .ok_or("`created by` must be a valid string")
            })
            .transpose()?;

        let encoding = input_dict
            .remove(&b"encoding"[..])
            .map(|encoding_benc| {
                encoding_benc
                    .to_string()
                    .ok_or("`encoding` must be a valid string")
            })
            .transpose()?;

        Ok(MetainfoFile {
            info: info_benc.try_into()?,
            info_hash: info_hash_array.into(),
            announce,
            announce_list,
            creation_date,
            comment,
            created_by,
            encoding,
        })
    }
}

impl From<&MetainfoFile> for Vec<u8> {
    fn from(input: &MetainfoFile) -> Vec<u8> {
        BencodeValue::from(input).encode()
    }
}

impl<'a> From<&'a MetainfoFile> for BencodeValue<'a> {
    fn from(input: &'a MetainfoFile) -> Self {
        [
            ("info", (&input.info).into()),
            ("announce", input.announce.as_str().into()),
        ]
        .into_iter()
        .chain(input.announce_list.iter().map(|announce_list| {
            (
                "announce-list",
                announce_list
                    .iter()
                    .map(|v| {
                        v.into_iter()
                            .map(|s| BencodeValue::from(s.as_str()))
                            .collect::<BencodeValue<'_>>()
                    })
                    .collect(),
            )
        }))
        .chain(
            input
                .creation_date
                .iter()
                .map(|d| ("creation date", d.into())),
        )
        .chain(input.comment.iter().map(|s| ("comment", s.as_str().into())))
        .chain(
            input
                .created_by
                .iter()
                .map(|s| ("created by", s.as_str().into())),
        )
        .chain(
            input
                .encoding
                .iter()
                .map(|s| ("encoding", s.as_str().into())),
        )
        .collect()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_test() {
        let metainfo_bytes =
            include_bytes!("../../../tests/examples/ubuntu-22.04.3-desktop-amd64.iso.torrent");
        let metainfo: MetainfoFile = metainfo_bytes[..].try_into().unwrap();

        {
            let Info::SingleFile {
                piece_length,
                pieces,
                name,
                length,
                md5sum,
            } = &metainfo.info
            else {
                panic!("Expected file to parse as single file")
            };

            assert_eq!(256 * 1024, *piece_length);
            assert_eq!(
                (*length as f64 / *piece_length as f64).ceil(),
                pieces.len() as f64,
                "length: {}, piece_length: {}, pieces.len(): {}",
                length,
                piece_length,
                pieces.len(),
            );
            assert_eq!(name, "ubuntu-22.04.3-desktop-amd64.iso");
            assert_eq!(&None, md5sum);
        }

        assert_eq!(
            InfoHash(
                "75439d5de343999ab377c617c2c647902956e282"
                    .as_bytes()
                    .chunks_exact(2)
                    .map(|b| u8::from_str_radix(std::str::from_utf8(b).unwrap(), 16).unwrap())
                    .collect::<Vec<u8>>()
                    .try_into()
                    .unwrap(),
            ),
            metainfo.info_hash,
        );

        /*
        assert_eq!(
            Some(vec![
                vec!["https://torrent.ubuntu.com/announce".to_string()],
                vec!["https://ipv6.torrent.ubuntu.com/announce".to_string()],
            ]),
            metainfo.announce_list,
        );
        */

        // Validate that the input file is byte-for-byte the same as the output
        assert_eq!(metainfo_bytes[..], Vec::<u8>::from(&metainfo)[..]);
    }
}
