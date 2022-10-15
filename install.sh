#!/bin/bash

# download latest release from repo
repo="https://github.com/josiahbull/dds"
release=$(curl -sL $repo/releases/latest | grep -oP '(?<=/tag/).*(?=")')

# download and extract release
curl -sL $repo/releases/download/$release/dds.tar.gz | tar -xz

# move binary to /usr/local/bin
sudo mv dds /usr/local/bin

# make executable
sudo chmod +x /usr/local/bin/dds

# run dds --generate <bash> where <bash> is the name of your shell, either zsh, fish, or bash

# if the user has a .zshrc file, then run dds --generate zsh
if [ -f ~/.zshrc ]; then
    dds --generate=zsh > /usr/local/share/zsh/site-functions/_value_hints_derive
    compinit
fi

# if the user has a .config/fish/config.fish file, then run dds --generate fish
if [ -f ~/.config/fish/config.fish ]; then
    dds --generate=fish > _value_hints_derive.fish
    . ./_value_hints_derive.fish
    rm _value_hints_derive.fish
fi

# if the user has a .bashrc file, then run dds --generate bash
if [ -f ~/.bashrc ]; then
    dds --generate=bash > /etc/bash_completion.d/value_hints_derive
fi

# remove install script
rm install.sh

echo "dds installed successfully, try running dds --help"
