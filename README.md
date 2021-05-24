PlanB [shared a code snipped](https://twitter.com/100trillionUSD/status/1396508774446292993) that would read and proccess some information.

I rewrite-it-in-Rust'd, and got an initial [sync-version main.rs version](https://github.com/swfsql/bitcoin/blob/45781490eea487e3693bb9c8b3683c8cf2779721/src/main.rs) - which ran the test in `190ms` and consumed `21MB` of RAM (it was reading the input all at once).

The current version is now async, with 2 worker threads. It runs on `215ms`, consuming `7.6MB` of RAM.  
The two worker threads will share any pending task of (1) reading the file in-order; (2) parsing the file in-order; and (3) changing the "dictionary" accordingly, also in-order.  
There is a backpressure from 3-2-1, that is, if the workers are too busy doing the tasks (3) and (2), the task (1) won't receive worker.  

To run, you'll need rust installed, and run, inside this project:

```bash
cargo run --release
```

You may also pass some args, such as:

```bash
cargo run -- --path "data_test" --read 30 --parsed 40
```

And for more info, use:

```bash
cargo run -- --help
```

