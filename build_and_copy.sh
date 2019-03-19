cargo build --release --target=arm-unknown-linux-gnueabihf

sudo cp ./target/arm-unknown-linux-gnueabihf/release/rusty_controller /run/media/$USER/rootfs/usr/bin/rusty_controller
sudo cp ./examples/config.json /run/media/$USER/rootfs/etc/rusty_controller/config.json