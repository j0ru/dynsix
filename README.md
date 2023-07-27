# Dynsix

This project provides you with an IPv6 only dynamic DNS (DDNS) server, designed for the benefits of IPv6.

## Concept

Before we dive into how dynsix works, let's review some basics about IPv6 addresses.

An IPv6 address is a 128-bit value that is usually represented as eight groups of four
hexadecimal digits, each group representing 16 bits (two octets), separated by colons,
for example 2001:0db8:85a3:0000:0000:8a2e:0370:7334. This results in a significantly larger
IP address space than its predecessor, IPv4.

The IPv6 address is often divided into two logical parts: a 64-bit network prefix,
and a 64-bit host address part. The network prefix is usually shared between all devices
in the network, while the host is identified by the latter 64 bit.

It is possible to pin the host part of an address, i.e. in debian in the `/etc/network/interfaces`,
you could do it like this:

```
auto lo
iface lo inet loopback

auto eth0
iface eth0 inet dhcp
iface eth0 inet6 auto
  up ip token set ::104 dev eth0
```

This will result in your machines ip to start with the network prefix and end in `::104`

Now we have everything to predict the IP-address of our machines without having to install some custom piece
of software on them and to build up a centralized dyndns machine that caters to the whole network.

## Installation

For now the project has to be build from source:

```bash
git clone https://github.com/j0ru/dynsix.git
cd dynsix
cargo build --release

sudo cp target/release/dynsix /usr/local/bin/
```

In the `assets/systemd` directory you can find a unit and a timer to automatically run dynsix on a schedule

## Configuration


```toml
# query your externally used ipv6 address
query_server = "https://ifconfig.co"

# optional if only one provider is configured
default_provider = "gandi"

[provider.gandi]
token = "your gandi api token"

[service.bar]
suffix = "::101"

# This entry will result in a bar.example.org address
name = "bar"
fqdn = "example.org"
ttl = 600

# optional if only one provider is configured
provider = "gandi"
```
