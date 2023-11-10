#!/usr/bin/env bash
set -e
# This script sets up a simple docker based test environment for us.  It's fairly basic but has enough to let us test a
# fair amount of functionality in isolation.  The original version of this used vagrant for VMs.  It gave us a little
# deeper control on per host networking options, but unfortunately GitHub has gotten better about not allowing VMs
# inside actions.
# First we need to set up a network for our containers to attach to
docker network create --subnet 172.0.0.0/24 test-network
# Now let's spawn a couple of containers to scan.
docker run --rm -d --name web --net test-network --ip 172.0.0.2 -p 80 nginx
docker run --rm -d --name no-ping --net test-network --ip 172.0.0.4 -p 80 nginx
# Okay now this is the weirdness.  We can't control if an individual docker container will respond to ping BUT docker
# does provide an interface where we can expand on their iptables rules.
# For details see: https://docs.docker.com/network/packet-filtering-firewalls/
#
# We want to be able to test how our system behaves when scanning host's with ping enabled and disable when doing an
# ICMP sweep before scanning.  By apply iptables rules we can decide the conditions to drop or accept these packets.
#
# In the future we will want to build more advanced tests, but the goal here is to the minimum for some basic
# integration tests.
# TODO: We should make this default drop then opt into the hosts with ping enabled
sudo iptables -I DOCKER-USER -p icmp -m conntrack --ctorigdst 172.0.0.4 -j DROP
# Now we *should* have a fully working test env locally.  If you want to experiment around start a container and join it
# to the test network.  An example of how to do that is:
# docker run -it --rm --net test-network -v $(pwd):/bowbend busybox sh