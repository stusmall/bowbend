#!/usr/bin/env bash
docker stop web
docker stop no-ping
docker network rm test-network
# I'm sure there is a more graceful way to do this, but I want to nuke it from orbit a bit.
# This flushes the chain to remove all rules.  Then we restart the docker service.  As part
# of the service's start up it will make sure the chain is in a healthy place, in this case it
# will re-add the one return rule.
sudo iptables -F DOCKER-USER
sudo service docker restart