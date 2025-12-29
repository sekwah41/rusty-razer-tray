Razer Battery Tray
=======================

I also thought it would be easier to install than the python version I saw, so I wanted to make my own basic test.

I may take a look at properly releasing this package if I end up using it but as my device has an obvious indicator on the wireless base there is not a lot of use for this.

This was just an experiment for doing tray icons for using Rust on linux easily.

I may add more but for now the plans are just battery level, and maybe quick access to adjust the polling rate.

As all of the rust tools for openrazer are either out of date or highly specific, ive made a simple wrapper of my own for the basic functionality I need.

Was tempted to use a similar approach to https://github.com/EchelonFour/razer_driver_rs/tree/main to avoid needing openrazer but decided to work with the existing ecosystem.

If I expand the functionality I may look to make a full port of [libopenrazer](https://github.com/z3ntu/libopenrazer) in rust.

If anyone would like me to continue this project feel free to open an issue or discussion. Otherwise, it'll just be for me messing around.
