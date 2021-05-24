use std::collections::BTreeMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let f = std::fs::read_to_string("data_test").unwrap();

    let mut d = BTreeMap::new();

    for l in f.lines() {
        let v: Vec<&str> = l.trim().split(' ').collect();
        let b: i32 = v[0].parse()?;
        let cleanup_len: usize = v[2].parse()?;
        let values_len: usize = v[3].parse()?;

        let v = &v[4..];
        let del_keys = &v[0..cleanup_len];
        let keys_values = &v[cleanup_len..];
        let values = &keys_values[..values_len];
        let keys = &keys_values[values_len..];

        // dbg!(&del_keys);
        // dbg!(&keys_values);
        // dbg!(&keys);
        // dbg!(&values);

        let del_keys: Vec<(u128, u128, u8)> = del_keys
            .iter()
            .map(|k| {
                let upper = u128::from_str_radix(&k[0..32], 16)?;
                let lower = u128::from_str_radix(&k[32..64], 16)?;
                let tail: u8 = (&k[64..]).parse()?;
                Ok((upper, lower, tail))
            })
            .collect::<Result<_, std::num::ParseIntError>>()?;

        let values: Vec<(i32, f32)> = values
            .iter()
            .map(|v| v.parse().map(|v| (b, v)))
            .collect::<Result<_, _>>()?;

        let keys: Vec<(u128, u128, u8)> = keys
            .iter()
            .map(|k| {
                let upper = u128::from_str_radix(&k[0..32], 16)?;
                let lower = u128::from_str_radix(&k[32..64], 16)?;
                let tail: u8 = (&k[64..]).parse()?;
                Ok((upper, lower, tail))
            })
            .collect::<Result<_, std::num::ParseIntError>>()?;

        for (key, value) in keys.into_iter().zip(values) {
            d.insert(key, value);
        }
        for del_key in del_keys {
            d.remove(&del_key);
        }
    }

    // println!("{:?}", d);

    Ok(())
}
