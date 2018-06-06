# i3 Time Tracker
[![Build Status](https://travis-ci.org/danbruce/i3-tracker-rs.svg?branch=master)](https://travis-ci.org/danbruce/i3-tracker-rs)

A simple event logging tool to track time spent per window/tab in i3.

## Alpha Build

This project is early stage. Do not expect this project will work as expected,
run out of the box, and not kill your cat. You have been warned.

## Installation

### From Source

```bash
cd ~
git clone git@github.com:danbruce/i3-tracker-rs.git
cd i3-tracker-rs
cargo build --release
```

### Start with i3

```bash dd
#~/.config/i3/config

exec --no-startup-id ~/i3-tracker-rs/target/release/time-tracker
```
