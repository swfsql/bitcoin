use std::{collections::BTreeMap, convert::TryFrom};
use tokio::io::AsyncBufReadExt;

#[derive(Clone, Debug, structopt::StructOpt)]
pub struct Config {
    /// The path for the data.
    #[structopt(long, parse(from_os_str), default_value = "data_test")]
    pub path: std::path::PathBuf,

    /// Limits how many read lines may be awaiting to be parsed.
    #[structopt(long, default_value = "30")]
    pub read: u16,

    /// Limits how many parsed lines may be awaiting to be included in the dictionary.
    #[structopt(long, default_value = "40")]
    pub parsed: u16,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            path: "data_test".into(),
            read: 30,
            parsed: 40,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Key {
    pub upper: u128,
    pub lower: u128,
    pub tail: u8,
}

#[derive(Debug)]
pub struct Value {
    pub b: i32,
    pub amount: f32,
}

#[derive(Debug)]
struct LineItem {
    pub keys: Vec<Key>,
    pub values: Vec<Value>,
    pub del_keys: Vec<Key>,
}

impl LineItem {
    pub fn consume(self, dict: &mut BTreeMap<Key, Value>) {
        for (key, value) in self.keys.into_iter().zip(self.values.into_iter()) {
            dict.insert(key, value);
        }
        for del_key in self.del_keys.into_iter() {
            dict.remove(&del_key);
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error(transparent)]
    IntegerError(#[from] std::num::ParseIntError),
    #[error(transparent)]
    FloatError(#[from] std::num::ParseFloatError),
}

impl TryFrom<&str> for LineItem {
    type Error = ParseError;

    fn try_from(l: &str) -> Result<Self, Self::Error> {
        let v: Vec<&str> = l.trim().split(' ').collect();
        let b: i32 = v[0].parse()?;
        let cleanup_len: usize = v[2].parse()?;
        let values_len: usize = v[3].parse()?;

        let v = &v[4..];
        let del_keys = &v[0..cleanup_len];
        let values_and_keys = &v[cleanup_len..];
        let values = &values_and_keys[..values_len];
        let keys = &values_and_keys[values_len..];

        let del_keys: Vec<Key> = del_keys
            .iter()
            .map(|k| {
                let upper = u128::from_str_radix(&k[0..32], 16)?;
                let lower = u128::from_str_radix(&k[32..64], 16)?;
                let tail = u8::from_str_radix(&k[64..], 16)?;
                Ok(Key { upper, lower, tail })
            })
            .collect::<Result<_, std::num::ParseIntError>>()?;

        let values: Vec<Value> = values
            .iter()
            .map(|v| v.parse().map(|amount| Value { b, amount }))
            .collect::<Result<_, _>>()?;

        let keys: Vec<Key> = keys
            .iter()
            .map(|k| {
                let upper = u128::from_str_radix(&k[0..32], 16)?;
                let lower = u128::from_str_radix(&k[32..64], 16)?;
                let tail = u8::from_str_radix(&k[64..], 16)?;
                Ok(Key { upper, lower, tail })
            })
            .collect::<Result<_, std::num::ParseIntError>>()?;

        Ok(Self {
            keys,
            values,
            del_keys,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    ParseError(#[from] ParseError),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

// data_test
pub async fn run(config: Config) -> Result<BTreeMap<Key, Value>, Error> {
    let path = config.path;
    let (parser_tx, mut parser_rx) = tokio::sync::mpsc::channel::<String>(config.read as usize);
    // line reader task
    tokio::spawn(async move {
        let f = tokio::fs::File::open(path).await?;

        let f = tokio::io::BufReader::new(f);
        let mut lines = f.lines();

        while let Some(l) = lines.next_line().await? {
            parser_tx.send(l).await.unwrap();
        }
        Result::<(), Error>::Ok(())
    });

    let (inserter_tx, mut inserter_rx) =
        tokio::sync::mpsc::channel::<LineItem>(config.parsed as usize);
    // line parser task
    tokio::spawn(async move {
        while let Some(l) = parser_rx.recv().await {
            let l = LineItem::try_from(l.as_ref())?;
            inserter_tx.send(l).await.unwrap();
        }
        Result::<(), Error>::Ok(())
    });

    // dict inserter/remover task
    let d = tokio::spawn(async move {
        let mut d = BTreeMap::new();
        while let Some(l) = inserter_rx.recv().await {
            l.consume(&mut d);
        }
        d
    })
    .await
    .unwrap();

    Ok(d)
}
