#!/bin/sh
# Reading the game's traffic needs CAP_NET_RAW and nothing else.
setcap cap_net_raw=ep /usr/bin/zombolens || echo "ZomboLens: run 'sudo setcap cap_net_raw=ep /usr/bin/zombolens'"
exit 0
