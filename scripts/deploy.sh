#!/bin/sh
export PATH="/j/rust/SysGCC/raspberry/bin/:$PATH"
cargo build --release --target armv7-unknown-linux-gnueabihf
#scp target/armv7-unknown-linux-gnueabihf/release/mvr pi@192.168.178.101:~/mvr
#ssh pi@192.168.178.101 'chmod +x ~/mvr;strip ~/mvr'
scp target/armv7-unknown-linux-gnueabihf/release/mvr dev@192.168.178.101:/home/dev/tmp/streamtest/bettermotion/server/bin/mvr
ssh dev@192.168.178.101 'cd /home/dev/tmp/streamtest/bettermotion/server/bin;chmod +x mvr;strip mvr'
# on raspi:
#/usr/bin/raspivid --verbose --inline --spstimings --hflip --vflip --annotate 1036 --annotate " My Awesome Sensor \n %Y-%m-%d %X " --annotateex 16,0x00,0x4C96B0,2,0,990 --width 1920 --height 1080 --timeout 0 --framerate 30 --bitrate 1700000 --profile baseline --vectors tcp://127.0.0.1:8001 --output - > /dev/null
