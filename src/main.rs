use structopt::StructOpt;

pub fn main() {
    let config = read_test::Config::from_args();

    let _dict = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .build()
        .unwrap()
        .block_on(async { read_test::run(config).await })
        .unwrap();
}
