#!/bin/bash

# download latest release from repo
repo="https://github.com/josiahbull/dds"

# download and extract release
curl -L $repo/releases/download/latest/dds --output dds

# move binary to /usr/local/bin
sudo mv dds /usr/local/bin

# make executable
sudo chmod +x /usr/local/bin/dds

# run dds --generate <bash> where <bash> is the name of your shell, either zsh, fish, or bash

# if the user has a .zshrc file, then run dds --generate zsh
if [ -f ~/.zshrc ]; then
    sudo dds --generate=zsh > /usr/local/share/zsh/site-functions/dds_completions
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
    sudo dds --generate=bash > /etc/bash_completion.d/dds_completions
fi

# remove install script
rm install.sh

echo "dds installed successfully, try running dds --help"
