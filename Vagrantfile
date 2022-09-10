# Requires `vagrant plugin install vagrant-hosts` for sync_hosts
Vagrant.configure("2") do |config|
  # Provision the host where we will be doing most of our tests
  config.vm.define "scanner", primary: true do |scanner|
    scanner.vm.box = "ubuntu/focal64"
    scanner.vm.network "private_network", ip: "192.168.56.2"
    scanner.vm.provision :hosts, :sync_hosts => true
  end

 # Set up a host with a random service to scan.  We will create a more complex service later to allow better mocking
  config.vm.define "web" do |web|
    web.vm.box = "ubuntu/focal64"
    web.vm.provision "shell", inline: "apt-get install -y nginx"
    web.vm.hostname = "web"
    web.vm.network "private_network", ip: "192.168.56.3"
    web.vm.provision :hosts, :sync_hosts => true
  end
end
