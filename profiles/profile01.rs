use structopt::StructOpt;

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = read_test::Config::from_args_safe()?;

    let _dicts = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(config.threads as usize)
        .build()
        .unwrap()
        .block_on(async { read_test::run(config).await })
        .unwrap();

    Ok(())
}
