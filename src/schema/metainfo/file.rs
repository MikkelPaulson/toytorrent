use super::Md5Value;

use crate::bencode::BencodeValue;
use crate::schema::Error;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct File {
    pub length: u64,
    pub md5sum: Option<Md5Value>,
    pub path: Vec<String>,
}

impl TryFrom<BencodeValue<'_>> for File {
    type Error = Error;

    fn try_from(input: BencodeValue<'_>) -> Result<Self, Self::Error> {
        let mut input_dict = input.to_dict().ok_or("`file` value must be a dict")?;

        let (Some(BencodeValue::Integer(length)), Some(BencodeValue::List(path_list))) = (
            input_dict.remove("length".as_bytes()),
            input_dict.remove("path".as_bytes()),
        ) else {
            return Err("`file` dict must have length and path values".into());
        };

        let md5sum = input_dict
            .remove("md5sum".as_bytes())
            .map(Md5Value::try_from)
            .transpose()?;

        let path = path_list
            .into_iter()
            .map(|benc| benc.to_string().ok_or("`path` components must be strings"))
            .collect::<Result<_, _>>()?;

        Ok(File {
            length: length.try_into().map_err(|e| format!("{}", e))?,
            md5sum,
            path,
        })
    }
}

impl<'a> From<&'a File> for BencodeValue<'a> {
    fn from(input: &'a File) -> Self {
        [
            ("length", input.length.into()),
            (
                "path",
                input
                    .path
                    .iter()
                    .map(|s| BencodeValue::from(s.as_str()))
                    .collect(),
            ),
        ]
        .into_iter()
        .chain(input.md5sum.iter().map(|md5sum| ("md5sum", md5sum.into())))
        .collect()
    }
}
