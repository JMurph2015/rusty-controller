cargo build --release --target=arm-unknown-linux-gnueabihf

sudo cp ./target/arm-unknown-linux-gnueabihf/release/rusty_controller /media/murphyj/rootfs/usr/bin/rusty_controller
sudo cp ./examples/config.json /media/murphyj/rootfs/etc/rusty_controller/config.json