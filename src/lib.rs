use std::{collections::BTreeMap, convert::TryFrom};
use tokio::io::AsyncBufReadExt;
use tracing::{debug, info, Instrument};

#[derive(Clone, Debug, structopt::StructOpt)]
pub struct Config {
    /// The path for the data.
    #[structopt(long, parse(from_os_str), default_value = "data_test")]
    pub path: std::path::PathBuf,

    /// Limits how many worker threads to use.
    #[structopt(long, default_value = "2")]
    pub threads: u16,

    /// Limits how many read lines may be awaiting to be parsed.
    #[structopt(long, default_value = "400")]
    pub read: u16,

    /// Limits how many parsed lines may be awaiting to be included in the dictionary.
    #[structopt(long, default_value = "400")]
    pub parsed: u16,

    /// Limits how many dictionaries may be build in parallel.
    #[structopt(long, default_value = "1")]
    pub dicts: u16,
}

impl Default for Config {
    fn default() -> Self {
        // use structopt::StructOpt;
        // Self::from_args()
        Config {
            threads: 2,
            path: "data_test".into(),
            read: 400,
            parsed: 400,
            dicts: 1,
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
pub struct LineItem {
    pub keys: Vec<Key>,
    pub values: Vec<Value>,
    pub del_keys: Vec<Key>,
}

impl LineItem {
    /// tries to insert the key/values and also to clean-up some keys
    /// accordingly to `del_keys`.
    ///
    /// Returns the keys that were not found during clean-up.
    pub fn consume(self, dict: &mut BTreeMap<Key, Value>) -> Vec<Key> {
        for (key, value) in self.keys.into_iter().zip(self.values.into_iter()) {
            dict.insert(key, value);
        }
        let mut not_found = vec![];
        for del_key in self.del_keys.into_iter() {
            if dict.remove(&del_key).is_none() {
                not_found.push(del_key);
            };
        }
        not_found
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

// TODO: actually use SPSC.
/// Creates a SPSC-style channel.
///
/// For `count = 3`, it returns senders/receivers for the tasks
/// `[0]->[1]->[2]->[0]`.  
/// That is, the first index is supposed to be used by the first task,
/// and it contains a sender into the task `[1]`, and also a receiver from
/// task `[2]`.
pub fn ring_formation<T>(
    count: usize,
    buffer_qty: usize,
) -> Vec<(tokio::sync::mpsc::Sender<T>, tokio::sync::mpsc::Receiver<T>)> {
    let mut tx = vec![];
    let mut rx = vec![];
    for _i in 0..count {
        let (t, r) = tokio::sync::mpsc::channel::<T>(buffer_qty);
        tx.push(t);
        rx.push(r);
    }
    for i in (0..count).rev().skip(1).rev() {
        tx.swap(i, (i + 1) % count);
        rx.swap(count - i - 1, count - i - 2);
    }
    tx.into_iter().zip(rx.into_iter()).collect()
}

pub async fn run(config: Config) -> Result<Vec<BTreeMap<Key, Value>>, Error> {
    let path = config.path;

    // TODO: sender is SP; still try actually having MC.
    let (parser_tx, mut parser_rx) = tokio::sync::mpsc::channel::<String>(config.read as usize);
    // line reader task
    tokio::spawn(
        async move {
            info!("opening file");
            let f = tokio::fs::File::open(path).await?;

            let f = tokio::io::BufReader::new(f);
            let mut lines = f.lines();

            let mut count: usize = 0;
            while let Some(l) = lines.next_line().await? {
                debug!("read line {}", count);
                count += 1;
                parser_tx.send(l).await.unwrap();
            }
            info!("closing file");
            Result::<(), Error>::Ok(())
        }
        .instrument(tracing::info_span!("line_reader")),
    );

    let (inserter_tx, inserter_rx) = async_channel::bounded::<LineItem>(config.parsed as usize);
    // line parser task
    // TODO: try having multiple of this
    tokio::spawn(
        async move {
            info!("starting");
            let mut count: usize = 0;
            while let Some(l) = parser_rx.recv().await {
                let l = LineItem::try_from(l.as_ref())?;
                debug!("parsed line {}", count);
                count += 1;
                inserter_tx.send(l).await.unwrap();
            }
            info!("finished");
            Result::<(), Error>::Ok(())
        }
        .instrument(tracing::info_span!("line_parser")),
    );

    // dict inserter/remover task

    let handles = futures::stream::FuturesUnordered::new();
    let mut removal_channels = ring_formation::<Key>(config.dicts as usize, 30);
    removal_channels.reverse();

    for id in 0..config.dicts {
        // TODO: SPSC
        let (removal_tx, mut removal_rx) = removal_channels.pop().unwrap();
        let inserter_rx = inserter_rx.clone();
        let mut remaining_removal_keys = vec![];

        let handle = tokio::spawn(
            async move {
                // info!("starting");

                let mut d = BTreeMap::new();
                let mut previous_task_dropped = false;

                loop {
                    tokio::select! {
                        biased;
                        rm = removal_rx.recv(), if !previous_task_dropped => {
                            if let Some(rm) = rm {
                                if d.remove(&rm).is_none() {
                                    match removal_tx.send(rm).await {
                                        Ok(_) => (),
                                        // the next/receiver task has dropped it's
                                        // receiver end (which would receive from
                                        // this sender)
                                        Err(rm) => {
                                            remaining_removal_keys.push(rm.0);
                                        }
                                    }
                                }
                            } else {
                                // the previous task has dropped it's sender
                                // end (which would send into this receiver)
                                previous_task_dropped = true;
                            }
                        }
                        line_item = inserter_rx.recv() => {
                            if let Ok(line_item) = line_item {
                                line_item.consume(&mut d);
                            } else {
                                // the item sender has been dropped
                                // (no more items will be produced)
                                break;
                            }
                        }
                        else => {
                            unreachable!();
                        }
                    };
                }

                // info!("finished");

                // each task end up with a dict, and also some keys
                // that will be used as a cleanup for other tasks
                // (the channels were disengaged before those keys
                // could be sent)
                (d, remaining_removal_keys)
            }
            .instrument(tracing::info_span!("handler", id)),
        );
        handles.push(handle);
    }

    let (mut ds, removals): (Vec<_>, Vec<_>) = futures::future::join_all(handles)
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
        .into_iter()
        .unzip();
    let removals: Vec<Key> = removals.into_iter().flatten().collect();
    info!("remaining removals: {}", removals.len());
    for d in &mut ds {
        for rm in &removals {
            d.remove(rm);
        }
    }
    Ok(ds)
}
