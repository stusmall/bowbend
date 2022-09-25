# Requires `vagrant plugin install vagrant-hosts` for sync_hosts
Vagrant.configure("2") do |config|
  # Provision the host where we will be doing most of our tests
  config.vm.define "scanner", primary: true do |scanner|
    scanner.vm.box = "ubuntu/jammy64"
    scanner.vm.provision "shell", inline: "apt-get update && apt-get install -y python3-pip"
    scanner.vm.network "private_network", ip: "192.168.56.2"
    scanner.vm.provision :hosts, :sync_hosts => true
  end

 # Set up a host with a random service to scan.  We will create a more complex service later to allow better mocking
  config.vm.define "web" do |web|
    web.vm.box = "ubuntu/jammy64"
    web.vm.provision "shell", inline: "apt-get update && apt-get install -y nginx"
    web.vm.hostname = "web"
    web.vm.network "private_network", ip: "192.168.56.3"
    web.vm.provision :hosts, :sync_hosts => true
  end

  config.vm.define "noping" do |no_ping|
    no_ping.vm.box = "ubuntu/jammy64"
    no_ping.vm.provision "shell", inline: "echo 1 > /proc/sys/net/ipv4/icmp_echo_ignore_all"
    no_ping.vm.hostname = "noping"
    no_ping.vm.network "private_network", ip: "192.168.56.4"
    no_ping.vm.provision :hosts, :sync_hosts => true
  end
end