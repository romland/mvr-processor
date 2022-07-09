# Xorzee MVR
This is a helper program written for the server-side of [Xorzee](https://github.com/romland/xorzee), which is largely
written under node-js. Due to performance needs, this is written in [Rust](https://www.rust-lang.org/).

The program expects a stream of 32-bit motion vectors fetched while the GPU is 
encoding the sensor data to H264, making it essentially free. This program will
then output a stream of JSON messages passed to the node-server. Node-js will
process it for activity and then pass it on over websockets to clients.

It operates in the following ways on these motion vectors:
- Parse
- Filter
- Categorize
- Density based cluster
- Temporally track clusters

I think the density based scan is quite fast. That said, the way things
are implemented, `epsilon` might not mean _exactly_ what you would expect.
But as long as I am not using euclidean distance, I matters little.

Stand-alone, without Xorzee, this is probably of little use to you. :-)

## Cross compiling
You need to install arm linker; arm-linux-gnueabihf-gcc. This section
should probably be fleshed out.

```
cargo build --release --target armv7-unknown-linux-gnueabihf
```

## TODO
- I want to get rid of JSON and use a more binary format. Not because it's terrible
  in here, but because it takes a few milliseconds to parse it on the Node side
  if there is a lot of activity.
- Make configurable and finalize command-line arguments
