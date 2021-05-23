# Armada
*A High-Performance TCP SYN scanner*

## What is Armada?
Armada is a high performance TCP SYN scanner. This is equivalent to the type of scanning that nmap might perform when you use the `-sS` scan type. Armada's main goal is to answer the basic question "Is this port open?". It is then up to you, or your tooling, to dig further to identify what an open port is for. 

## How do I install Armada?
If you don't have `rustup` installed, visit the [rustup](https://rustup.rs) website and follow the instructions there to get started.

After you have `cargo` installed, run `cargo install armada`.

As Armada uses raw sockets to perform port scanning, you'll either need to be running as root or give the Armada binary the `CAP_NET_RAW` capability. My suggestion is the latter.

A full installation, after `cargo` has been installed via `rustup`, looks like this:

```
cargo install armada

sudo setcap 'cap_net_raw+ep' $(which armada)
```

## How do I run Armada?
Armada comes with help docs by running `armada -h`; however, if you want to get started immediately, the typical way to perform a port scan is the following:

```
armada <IP or CIDR> -p <PORT or PORT RANGE>
```

e.g.

```
armada 8.8.8.0/24 -p 1-1000
```

Happy Scanning
