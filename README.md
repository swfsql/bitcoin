# PlanB calculation snippet

PlanB [shared a code snipped](https://twitter.com/100trillionUSD/status/1396508774446292993) that would read an txt file and proccess some information related to the stock to flow metrics of btc.

I rewrote-it-in-Rust, and got an initial [sync-version](https://github.com/swfsql/bitcoin/blob/45781490eea487e3693bb9c8b3683c8cf2779721/src/main.rs) - which ran in `190ms` and consumed `21MB` of RAM for. That high memory was because it was reading the input all at once.  

Making it [single-threaded async](https://github.com/swfsql/bitcoin/blob/async-single/src/lib.rs) increased the runtime to `~300ms`, so I also made use of an additional thread in the async version to speed it up.  
Note: in this linked version, the worker thread actually was still blocked during file reading operations.

The current version is the multi-threaded async, with 2 worker threads. It runs on `215ms`, consuming `7.6MB` of RAM.  
The two worker threads will share any pending task of (1) reading the file in-order; (2) parsing the file in-order; and (3) changing the "dictionary" accordingly, also in-order.  
There is a backpressure from 3-2-1, that is, if the workers are too busy doing the tasks (3) and (2), the task (1) won't receive any worker.  

---

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

