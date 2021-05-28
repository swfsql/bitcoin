use std::time::Instant;
use structopt::StructOpt;

pub fn main() {
    let start = Instant::now();
    let config = read_test::Config::from_args();

    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();
    tracing::info!("Execution started");

    let dicts = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(config.threads as usize)
        .build()
        .unwrap()
        .block_on(async { read_test::run(config).await })
        .unwrap();

    // /*
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    for d in dicts {
        use std::io::Write;
        for (key, value) in d.into_iter() {
            writeln!(
                &mut handle,
                "{:x}{:x}{:x} {:?}",
                key.upper, key.lower, key.tail, value
            )
            .unwrap();
        }
    }
    // */
    let duration = start.elapsed();
    tracing::info!("Execution finished in {:?}", duration);
}
