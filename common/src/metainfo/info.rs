use super::{File, Md5Value, Piece};
use crate::bencode::BencodeValue;
use crate::Error;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Info {
    SingleFile {
        piece_length: u64,
        pieces: Vec<Piece>,
        name: String,
        length: u64,
        md5sum: Option<Md5Value>,
    },
    MultiFile {
        piece_length: u64,
        pieces: Vec<Piece>,
        name: String,
        files: Vec<File>,
    },
}

impl Info {
    pub fn length(&self) -> u64 {
        match self {
            Self::SingleFile { length, .. } => *length,
            Self::MultiFile { files, .. } => files.iter().map(|f| f.length).sum(),
        }
    }
}

impl TryFrom<BencodeValue<'_>> for Info {
    type Error = Error;

    fn try_from(input: BencodeValue<'_>) -> Result<Self, Self::Error> {
        let mut input_dict = input.to_dict().ok_or("`info` value must be a dict")?;

        let (
            Some(BencodeValue::Integer(piece_length)),
            Some(BencodeValue::Bytes(pieces_bytes)),
            Some(name),
        ) = (
            input_dict.remove("piece length".as_bytes()),
            input_dict.remove("pieces".as_bytes()),
            input_dict
                .remove("name".as_bytes())
                .and_then(BencodeValue::to_string),
        )
        else {
            return Err("`info` dict must contain `piece length` and `pieces` keys".into());
        };

        let pieces = if !pieces_bytes.is_empty() && pieces_bytes.len() % 20 == 0 {
            pieces_bytes
                .chunks_exact(20)
                .map(|a| a.try_into())
                .collect::<Result<_, _>>()
                .unwrap()
        } else {
            return Err(
                "`pieces` must be a nonempty stream with a length that is a multiple of 20 bytes"
                    .into(),
            );
        };

        match (
            input_dict.remove("length".as_bytes()),
            input_dict.remove("files".as_bytes()),
        ) {
            (Some(BencodeValue::Integer(length)), None) => Ok(Info::SingleFile {
                piece_length: piece_length.try_into().map_err(|e| format!("{}", e))?,
                pieces,
                name,
                length: length.try_into().map_err(|e| format!("{}", e))?,
                md5sum: input_dict
                    .remove("md5sum".as_bytes())
                    .map(Md5Value::try_from)
                    .transpose()?,
            }),
            (None, Some(BencodeValue::List(files))) => Ok(Info::MultiFile {
                piece_length: piece_length.try_into().map_err(|e| format!("{}", e))?,
                pieces,
                name,
                files: files
                    .into_iter()
                    .map(|file| file.try_into())
                    .collect::<Result<_, _>>()?,
            }),
            _ => Err("Exactly one of `length` or `files` keys must be present".into()),
        }
    }
}

impl<'a> From<&'a Info> for BencodeValue<'a> {
    fn from(input: &'a Info) -> Self {
        match input {
            Info::SingleFile {
                piece_length,
                pieces,
                name,
                length,
                md5sum,
            } => [
                ("piece length", (*piece_length).into()),
                (
                    "pieces",
                    pieces
                        .iter()
                        .map(|piece| piece.iter())
                        .flatten()
                        .copied()
                        .collect::<Vec<u8>>()
                        .into(),
                ),
                ("name", name.as_str().into()),
                ("length", (*length).into()),
            ]
            .into_iter()
            .chain(md5sum.into_iter().map(|md5sum| ("md5sum", md5sum.into())))
            .collect(),
            Info::MultiFile {
                piece_length,
                pieces,
                name,
                files,
            } => [
                ("piece length", (*piece_length).into()),
                (
                    "pieces",
                    pieces
                        .iter()
                        .map(|piece| piece.iter())
                        .flatten()
                        .copied()
                        .collect::<Vec<u8>>()
                        .into(),
                ),
                ("name", name.as_str().into()),
                ("files", files.iter().map(BencodeValue::from).collect()),
            ]
            .into_iter()
            .collect(),
        }
    }
}
