use structopt::StructOpt;

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = read_test::Config::from_args_safe()?;

    let _dict = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .build()
        .unwrap()
        .block_on(async { read_test::run(config).await })
        .unwrap();
    Ok(())
}
